use encoding_rs::UTF_8;
use hf_hub::progress::{DownloadEvent, ProgressEvent, ProgressHandler};
use hf_hub::HFClient;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::Serialize;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, State};

static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

fn get_backend() -> &'static LlamaBackend {
    BACKEND.get_or_init(|| LlamaBackend::init().expect("Failed to init llama backend"))
}

pub struct AiState {
    model: Mutex<Option<LlamaModel>>,
    models_dir: PathBuf,
}

impl AiState {
    pub fn new(models_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&models_dir)
            .map_err(|e| format!("Failed to create models directory: {e}"))?;
        get_backend();
        Ok(Self {
            model: Mutex::new(None),
            models_dir,
        })
    }

    pub fn unload(&self) {
        if let Ok(mut guard) = self.model.lock() {
            *guard = None;
        }
    }

    pub fn is_loaded(&self) -> Result<bool, String> {
        let guard = self
            .model
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        Ok(guard.is_some())
    }

    pub fn list_models_sync(&self) -> Result<Vec<AiModelFile>, String> {
        let mut models = Vec::new();
        let entries = std::fs::read_dir(&self.models_dir)
            .map_err(|e| format!("Failed to read models dir: {e}"))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "gguf") {
                let metadata = entry
                    .metadata()
                    .map_err(|e| format!("Failed to read metadata: {e}"))?;
                models.push(AiModelFile {
                    filename: entry.file_name().to_string_lossy().to_string(),
                    size_bytes: metadata.len(),
                });
            }
        }

        Ok(models)
    }

    pub fn delete_model_sync(&self, filename: &str) -> Result<(), String> {
        let path = self.models_dir.join(filename);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Failed to delete model: {e}"))?;
        }
        Ok(())
    }

    pub fn model_path(&self, filename: &str) -> PathBuf {
        self.models_dir.join(filename)
    }

    #[cfg(test)]
    pub fn models_dir(&self) -> &std::path::Path {
        &self.models_dir
    }
}

#[derive(Serialize)]
pub struct AiModelFile {
    pub filename: String,
    pub size_bytes: u64,
}

#[derive(Serialize, Clone)]
pub struct AiDownloadProgress {
    pub filename: String,
    pub percent: u8,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
}

struct DownloadProgressEmitter {
    app: AppHandle,
    filename: String,
    total_bytes: AtomicU64,
    downloaded_bytes: AtomicU64,
}

impl DownloadProgressEmitter {
    fn emit_progress(&self) {
        let total = self.total_bytes.load(Ordering::Relaxed);
        let downloaded = self.downloaded_bytes.load(Ordering::Relaxed);
        let raw = downloaded
            .saturating_mul(100)
            .checked_div(total)
            .unwrap_or(0);
        let percent: u8 = u8::try_from(raw.min(100)).unwrap_or(100);
        let _ = self.app.emit(
            "ai://download-progress",
            AiDownloadProgress {
                filename: self.filename.clone(),
                percent,
                bytes_downloaded: downloaded,
                total_bytes: total,
            },
        );
    }
}

impl ProgressHandler for DownloadProgressEmitter {
    fn on_progress(&self, event: &ProgressEvent) {
        match event {
            ProgressEvent::Download(DownloadEvent::Start { total_bytes, .. }) => {
                self.total_bytes.store(*total_bytes, Ordering::Relaxed);
                self.emit_progress();
            }
            ProgressEvent::Download(DownloadEvent::Progress { files }) => {
                for file in files {
                    if file.filename == self.filename || files.len() == 1 {
                        self.downloaded_bytes
                            .store(file.bytes_completed, Ordering::Relaxed);
                        if file.total_bytes > 0 {
                            self.total_bytes.store(file.total_bytes, Ordering::Relaxed);
                        }
                    }
                }
                self.emit_progress();
            }
            ProgressEvent::Download(DownloadEvent::AggregateProgress {
                bytes_completed,
                total_bytes,
                ..
            }) => {
                self.downloaded_bytes
                    .store(*bytes_completed, Ordering::Relaxed);
                if *total_bytes > 0 {
                    self.total_bytes.store(*total_bytes, Ordering::Relaxed);
                }
                self.emit_progress();
            }
            _ => {}
        }
    }
}

#[tauri::command]
pub async fn ai_download_model(
    repo: String,
    filename: String,
    app: AppHandle,
    state: State<'_, AiState>,
) -> Result<(), String> {
    let dest_dir = state.models_dir.clone();

    let (owner, name) = hf_hub::split_id(&repo);
    let client = HFClient::builder()
        .cache_enabled(false)
        .build()
        .map_err(|e| format!("Failed to create HF client: {e}"))?;

    let emitter = DownloadProgressEmitter {
        app: app.clone(),
        filename: filename.clone(),
        total_bytes: AtomicU64::new(0),
        downloaded_bytes: AtomicU64::new(0),
    };

    let model = client.model(owner, name);
    let path = model
        .download_file()
        .filename(&filename)
        .local_dir(dest_dir)
        .progress(emitter)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;

    if !path.exists() {
        return Err("Download completed but file not found".to_string());
    }

    let _ = app.emit(
        "ai://download-progress",
        AiDownloadProgress {
            filename: filename.clone(),
            percent: 100,
            bytes_downloaded: 0,
            total_bytes: 0,
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn ai_delete_model(filename: String, state: State<'_, AiState>) -> Result<(), String> {
    state.delete_model_sync(&filename)
}

#[tauri::command]
pub async fn ai_list_models(state: State<'_, AiState>) -> Result<Vec<AiModelFile>, String> {
    state.list_models_sync()
}

#[tauri::command]
pub async fn ai_load_model(
    filename: String,
    n_ctx: u32,
    state: State<'_, AiState>,
) -> Result<(), String> {
    let path = state.model_path(&filename);
    if !path.exists() {
        return Err(format!("Model file not found: {filename}"));
    }

    let n_ctx = if cfg!(target_os = "ios") || cfg!(target_os = "android") {
        n_ctx.min(1024)
    } else {
        n_ctx
    };

    let backend = get_backend();
    let model_params = LlamaModelParams::default().with_n_gpu_layers(0);

    let model = LlamaModel::load_from_file(backend, &path, &model_params)
        .map_err(|e| format!("Failed to load model: {e}"))?;

    let ctx_params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(n_ctx));
    let context = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {e}"))?;
    drop(context);

    let mut guard = state
        .model
        .lock()
        .map_err(|e| format!("Lock poisoned: {e}"))?;
    *guard = Some(model);

    Ok(())
}

#[tauri::command]
pub async fn ai_unload_model(state: State<'_, AiState>) -> Result<(), String> {
    state.unload();
    Ok(())
}

#[tauri::command]
pub async fn ai_is_loaded(state: State<'_, AiState>) -> Result<bool, String> {
    state.is_loaded()
}

const COMMAND_SYSTEM_PROMPT: &str = "You are a shell command generator. Respond with a single shell command only. No explanations, no markdown, no code fences, no quotes. Output just the raw command on one line.";

const COMPLETE_SYSTEM_PROMPT: &str = "You complete shell commands. Given a partial command, output ONLY the exact text that should follow to finish it. Do not repeat any text that is already present. No explanations, no markdown, no code fences. Output only the completion on one line.";

fn system_prompt_for(mode: &str) -> &'static str {
    match mode {
        "complete" => COMPLETE_SYSTEM_PROMPT,
        _ => COMMAND_SYSTEM_PROMPT,
    }
}

fn first_command_line(text: &str) -> String {
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let first = first
        .strip_prefix("$ ")
        .or_else(|| first.strip_prefix("# "))
        .unwrap_or(first);
    let first = first.trim();
    let first = if first.len() >= 2 && first.starts_with('`') && first.ends_with('`') {
        first[1..first.len() - 1].trim()
    } else {
        first
    };
    first.trim().to_string()
}

fn clean_command(raw: &str) -> String {
    let text = raw.trim();

    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        let content_start = match after_start.find('\n') {
            Some(idx) => &after_start[idx + 1..],
            None => after_start,
        };
        let content = match content_start.find("```") {
            Some(end) => &content_start[..end],
            None => content_start,
        };
        return first_command_line(content);
    }

    first_command_line(text)
}

#[tauri::command]
pub async fn ai_generate(
    prompt: String,
    max_tokens: u32,
    temperature: f32,
    top_p: f32,
    mode: String,
    state: State<'_, AiState>,
) -> Result<String, String> {
    let guard = state
        .model
        .lock()
        .map_err(|e| format!("Lock poisoned: {e}"))?;

    let model = guard.as_ref().ok_or("No model loaded")?;
    let backend = get_backend();

    let ctx_params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(2048));
    let mut context = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {e}"))?;

    let use_template = mode == "command" || mode == "complete";
    let (prompt_text, add_bos) = if use_template {
        let template = model
            .chat_template(None)
            .map_err(|e| format!("Failed to get chat template: {e}"))?;
        let messages = vec![
            LlamaChatMessage::new("system".to_string(), system_prompt_for(&mode).to_string())
                .map_err(|e| format!("Failed to build message: {e}"))?,
            LlamaChatMessage::new("user".to_string(), prompt.clone())
                .map_err(|e| format!("Failed to build message: {e}"))?,
        ];
        let formatted = model
            .apply_chat_template(&template, &messages, true)
            .map_err(|e| format!("Failed to apply chat template: {e}"))?;
        (formatted, AddBos::Never)
    } else {
        (prompt.clone(), AddBos::Always)
    };

    let tokens = model
        .str_to_token(&prompt_text, add_bos)
        .map_err(|e| format!("Failed to tokenize prompt: {e}"))?;

    let mut batch = LlamaBatch::new(tokens.len(), 1);
    let last_idx = tokens.len().saturating_sub(1);
    for (i, &token) in tokens.iter().enumerate() {
        let pos = i32::try_from(i).map_err(|_| "Token index overflow")?;
        batch
            .add(token, pos, &[0], i == last_idx)
            .map_err(|e| format!("Failed to add token to batch: {e}"))?;
    }

    context
        .decode(&mut batch)
        .map_err(|e| format!("Failed to decode batch: {e}"))?;

    let mut sampler = if temperature > 0.0 {
        LlamaSampler::chain(
            vec![
                LlamaSampler::penalties(64, 1.1, 0.0, 0.0),
                LlamaSampler::top_p(top_p, 1),
                LlamaSampler::temp(temperature),
                LlamaSampler::dist(rand::random()),
            ],
            false,
        )
    } else {
        LlamaSampler::greedy()
    };

    let mut decoder = UTF_8.new_decoder();
    let mut generated = String::new();
    let mut n_generated: u32 = 0;
    let mut pos = i32::try_from(tokens.len()).map_err(|_| "Token count overflow")?;

    loop {
        if n_generated >= max_tokens {
            break;
        }

        let next_token = sampler.sample(&context, -1);

        if model.is_eog_token(next_token) {
            break;
        }

        let piece = model
            .token_to_piece(next_token, &mut decoder, false, None)
            .map_err(|e| format!("Failed to decode token: {e}"))?;

        generated.push_str(&piece);
        n_generated += 1;
        sampler.accept(next_token);

        let mut new_batch = LlamaBatch::new(1, 1);
        new_batch
            .add(next_token, pos, &[0], true)
            .map_err(|e| format!("Failed to add token to batch: {e}"))?;
        pos += 1;

        context
            .decode(&mut new_batch)
            .map_err(|e| format!("Failed to decode: {e}"))?;
    }

    let output = generated.trim().to_string();
    if use_template {
        Ok(clean_command(&output))
    } else {
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn test_state() -> (AiState, TempDir) {
        let dir = TempDir::new().expect("failed to create temp dir");
        let state = AiState::new(dir.path().join("hf-models")).expect("failed to create AiState");
        (state, dir)
    }

    fn write_fake_gguf(dir: &std::path::Path, name: &str, size: usize) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(path).expect("failed to create file");
        let data = vec![0u8; size];
        file.write_all(&data).expect("failed to write file");
    }

    #[test]
    fn new_creates_models_directory() {
        let dir = TempDir::new().unwrap();
        let models_dir = dir.path().join("nested").join("hf-models");
        let _state = AiState::new(models_dir.clone()).unwrap();
        assert!(models_dir.exists());
    }

    #[test]
    fn is_loaded_false_initially() {
        let (state, _dir) = test_state();
        assert!(!state.is_loaded().unwrap());
    }

    #[test]
    fn unload_when_no_model_does_not_panic() {
        let (state, _dir) = test_state();
        state.unload();
        assert!(!state.is_loaded().unwrap());
    }

    #[test]
    fn list_models_empty_dir() {
        let (state, _dir) = test_state();
        let models = state.list_models_sync().unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn list_models_only_gguf_files() {
        let (state, _dir) = test_state();
        let models_dir = state.models_dir();

        write_fake_gguf(models_dir, "model-a.gguf", 1024);
        write_fake_gguf(models_dir, "model-b.gguf", 2048);
        write_fake_gguf(models_dir, "readme.txt", 100);
        write_fake_gguf(models_dir, "config.json", 50);

        let models = state.list_models_sync().unwrap();
        assert_eq!(models.len(), 2);

        let names: Vec<&str> = models.iter().map(|m| m.filename.as_str()).collect();
        assert!(names.contains(&"model-a.gguf"));
        assert!(names.contains(&"model-b.gguf"));
        assert!(!names.contains(&"readme.txt"));
        assert!(!names.contains(&"config.json"));
    }

    #[test]
    fn list_models_reports_correct_size() {
        let (state, _dir) = test_state();
        let models_dir = state.models_dir();

        write_fake_gguf(models_dir, "sized.gguf", 4096);

        let models = state.list_models_sync().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].size_bytes, 4096);
        assert_eq!(models[0].filename, "sized.gguf");
    }

    #[test]
    fn delete_model_removes_file() {
        let (state, _dir) = test_state();
        let models_dir = state.models_dir();

        write_fake_gguf(models_dir, "to-delete.gguf", 512);
        assert!(state.model_path("to-delete.gguf").exists());

        state.delete_model_sync("to-delete.gguf").unwrap();
        assert!(!state.model_path("to-delete.gguf").exists());
    }

    #[test]
    fn delete_model_nonexistent_is_noop() {
        let (state, _dir) = test_state();
        let result = state.delete_model_sync("does-not-exist.gguf");
        assert!(result.is_ok());
    }

    #[test]
    fn model_path_joins_correctly() {
        let (state, _dir) = test_state();
        let path = state.model_path("test-model.gguf");
        assert!(path.ends_with("hf-models/test-model.gguf"));
    }

    #[test]
    fn download_progress_serialization() {
        let progress = AiDownloadProgress {
            filename: "model.gguf".to_string(),
            percent: 42,
            bytes_downloaded: 4200,
            total_bytes: 10000,
        };
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["filename"], "model.gguf");
        assert_eq!(json["percent"], 42);
        assert_eq!(json["bytes_downloaded"], 4200);
        assert_eq!(json["total_bytes"], 10000);
    }

    #[test]
    fn model_file_serialization() {
        let file = AiModelFile {
            filename: "qwen.gguf".to_string(),
            size_bytes: 398_000_000,
        };
        let json = serde_json::to_value(&file).unwrap();
        assert_eq!(json["filename"], "qwen.gguf");
        assert_eq!(json["size_bytes"], 398_000_000);
    }

    #[test]
    fn percent_calculation_zero_total() {
        let total_bytes: u64 = 0;
        let bytes_downloaded: u64 = 100;
        let raw = bytes_downloaded
            .saturating_mul(100)
            .checked_div(total_bytes)
            .unwrap_or(0);
        let percent: u8 = u8::try_from(raw.min(100)).unwrap_or(100);
        assert_eq!(percent, 0);
    }

    #[test]
    fn percent_calculation_partial() {
        let total_bytes: u64 = 1000;
        let bytes_downloaded: u64 = 500;
        let raw = bytes_downloaded
            .saturating_mul(100)
            .checked_div(total_bytes)
            .unwrap_or(0);
        let percent: u8 = u8::try_from(raw.min(100)).unwrap_or(100);
        assert_eq!(percent, 50);
    }

    #[test]
    fn percent_calculation_complete() {
        let total_bytes: u64 = 1000;
        let bytes_downloaded: u64 = 1000;
        let raw = bytes_downloaded
            .saturating_mul(100)
            .checked_div(total_bytes)
            .unwrap_or(0);
        let percent: u8 = u8::try_from(raw.min(100)).unwrap_or(100);
        assert_eq!(percent, 100);
    }

    #[test]
    fn percent_calculation_saturates_at_100() {
        let total_bytes: u64 = 100;
        let bytes_downloaded: u64 = 200;
        let raw = bytes_downloaded
            .saturating_mul(100)
            .checked_div(total_bytes)
            .unwrap_or(0);
        let percent: u8 = u8::try_from(raw.min(100)).unwrap_or(100);
        assert_eq!(percent, 100);
    }

    #[test]
    fn clean_command_plain() {
        assert_eq!(clean_command("ls -la"), "ls -la");
    }

    #[test]
    fn clean_command_strips_fences() {
        assert_eq!(
            clean_command("```\nfind . -name '*.md'\n```"),
            "find . -name '*.md'"
        );
    }

    #[test]
    fn clean_command_strips_fences_with_lang() {
        assert_eq!(
            clean_command("```bash\nfind . -name '*.md'\n```"),
            "find . -name '*.md'"
        );
    }

    #[test]
    fn clean_command_strips_dollar_prompt() {
        assert_eq!(clean_command("$ ls -la"), "ls -la");
    }

    #[test]
    fn clean_command_takes_first_line() {
        assert_eq!(
            clean_command("find . -name '*.md'\nsome explanation"),
            "find . -name '*.md'"
        );
    }

    #[test]
    fn clean_command_extracts_fenced_block_from_prose() {
        let raw = "Files with .md in home folder:\n```\nfind ~ -name '*.md'\n```";
        assert_eq!(clean_command(raw), "find ~ -name '*.md'");
    }

    #[test]
    fn clean_command_strips_single_backticks() {
        assert_eq!(
            clean_command("`find /home -type f -name \"*.md\"`"),
            "find /home -type f -name \"*.md\""
        );
    }

    #[test]
    fn clean_command_empty() {
        assert_eq!(clean_command(""), "");
        assert_eq!(clean_command("   \n  "), "");
    }
}
