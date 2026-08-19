use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::command;

/// A single setting to write. `value_type` is intentionally absent — the type
/// is fixed by the seeded row and never changes.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingInput {
    pub key: String,
    pub value: String,
}

/// A setting row as returned to the frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    pub default_value: String,
    pub value_type: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Defaults for every known setting: (key, value, `value_type`).
///
/// `value` is written into both `value` and `default_value` on first seed.
/// Values are TEXT-encoded: numbers/booleans stringified, `null` as empty
/// string, arrays/objects as JSON.
fn seeds() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Terminal appearance
        // Default terminal color scheme: Catppuccin Mocha
        // (https://github.com/catppuccin/catppuccin, MIT).
        (
            "terminal.appearance.color_scheme",
            r##"{"background":"#1e1e2e","foreground":"#cdd6f4","cursor":"#f5e0dc","cursorAccent":"#1e1e2e","selectionBackground":"#f5e0dc","selectionForeground":"#1e1e2e","black":"#45475a","red":"#f38ba8","green":"#a6e3a1","yellow":"#f9e2af","blue":"#89b4fa","magenta":"#f5c2e7","cyan":"#94e2d5","white":"#bac2de","brightBlack":"#585b70","brightRed":"#f38ba8","brightGreen":"#a6e3a1","brightYellow":"#f9e2af","brightBlue":"#89b4fa","brightMagenta":"#f5c2e7","brightCyan":"#94e2d5","brightWhite":"#a6adc8"}"##,
            "json",
        ),
        (
            "terminal.appearance.font_family",
            "\"JetBrains Mono\", \"Fira Code\", monospace",
            "string",
        ),
        ("terminal.appearance.font_size", "14", "number"),
        ("terminal.appearance.font_weight", "400", "number"),
        ("terminal.appearance.font_weight_bold", "700", "number"),
        ("terminal.appearance.line_height", "1.2", "number"),
        ("terminal.appearance.cursor_style", "block", "string"),
        ("terminal.appearance.cursor_blink", "true", "boolean"),
        ("terminal.appearance.minimum_contrast_ratio", "1", "number"),
        // Terminal behavior
        ("terminal.behavior.copy_on_select", "false", "boolean"),
        ("terminal.behavior.scrollback", "1000", "number"),
        ("terminal.behavior.scroll_sensitivity", "1", "number"),
        // AI
        ("ai.enabled", "false", "boolean"),
        ("ai.model_id", "qwen2.5-coder-0.5b-instruct", "string"),
        ("ai.quant", "Q4_K_M", "string"),
        ("ai.inline_completion_enabled", "true", "boolean"),
        ("ai.command_generator_enabled", "true", "boolean"),
        ("ai.max_tokens", "32", "number"),
        ("ai.temperature", "0.2", "number"),
        ("ai.top_p", "0.95", "number"),
        ("ai.context_lines", "20", "number"),
        // SSH
        ("ssh.keepalive_interval", "", "null"),
        ("ssh.connect_timeout", "", "null"),
    ]
}

/// Insert default rows for every known setting, preserving existing rows.
pub async fn seed_settings(pool: &SqlitePool) -> Result<(), String> {
    for (key, value, value_type) in seeds() {
        sqlx::query(
            r#"INSERT INTO setting ("key", "value", "default_value", "value_type")
               VALUES (?, ?, ?, ?)
               ON CONFLICT("key") DO NOTHING"#,
        )
        .bind(key)
        .bind(value)
        .bind(value)
        .bind(value_type)
        .execute(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    }
    Ok(())
}

type SettingRow = (String, String, String, String, String, String);

async fn get_all_inner(pool: &SqlitePool) -> Result<Vec<SettingEntry>, String> {
    let rows = sqlx::query_as::<_, SettingRow>(
        r#"SELECT "key", "value", "default_value", "value_type", "created_at", "updated_at"
           FROM setting ORDER BY "key""#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(rows.into_iter().map(row_to_entry).collect())
}

async fn get_inner(pool: &SqlitePool, key: &str) -> Result<Option<SettingEntry>, String> {
    let row = sqlx::query_as::<_, SettingRow>(
        r#"SELECT "key", "value", "default_value", "value_type", "created_at", "updated_at"
           FROM setting WHERE "key" = ?"#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(row.map(row_to_entry))
}

fn row_to_entry(
    (key, value, default_value, value_type, created_at, updated_at): SettingRow,
) -> SettingEntry {
    SettingEntry {
        key,
        value,
        default_value,
        value_type,
        created_at,
        updated_at,
    }
}

async fn set_inner(pool: &SqlitePool, key: &str, value: &str) -> Result<(), String> {
    sqlx::query(
        r#"UPDATE setting SET "value" = ?, "updated_at" = datetime('now') WHERE "key" = ?"#,
    )
    .bind(value)
    .bind(key)
    .execute(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

async fn set_many_inner(pool: &SqlitePool, entries: &[SettingInput]) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("database error: {e}"))?;
    for entry in entries {
        sqlx::query(
            r#"UPDATE setting SET "value" = ?, "updated_at" = datetime('now') WHERE "key" = ?"#,
        )
        .bind(&entry.value)
        .bind(&entry.key)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    }
    tx.commit()
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

async fn reset_inner(pool: &SqlitePool, key: &str) -> Result<(), String> {
    sqlx::query(r#"UPDATE setting SET "value" = "default_value", "updated_at" = datetime('now') WHERE "key" = ?"#)
        .bind(key)
        .execute(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

async fn reset_many_inner(pool: &SqlitePool, keys: &[String]) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("database error: {e}"))?;
    for key in keys {
        sqlx::query(r#"UPDATE setting SET "value" = "default_value", "updated_at" = datetime('now') WHERE "key" = ?"#)
            .bind(key)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("database error: {e}"))?;
    }
    tx.commit()
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

async fn reset_all_inner(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(r#"UPDATE setting SET "value" = "default_value", "updated_at" = datetime('now')"#)
        .execute(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

#[command]
pub async fn settings_get_all(
    db: tauri::State<'_, SqlitePool>,
) -> Result<Vec<SettingEntry>, String> {
    get_all_inner(db.inner()).await
}

#[command]
pub async fn settings_get(
    db: tauri::State<'_, SqlitePool>,
    key: String,
) -> Result<Option<SettingEntry>, String> {
    get_inner(db.inner(), &key).await
}

#[command]
pub async fn settings_set(
    db: tauri::State<'_, SqlitePool>,
    key: String,
    value: String,
) -> Result<(), String> {
    set_inner(db.inner(), &key, &value).await
}

#[command]
pub async fn settings_set_many(
    db: tauri::State<'_, SqlitePool>,
    entries: Vec<SettingInput>,
) -> Result<(), String> {
    set_many_inner(db.inner(), &entries).await
}

#[command]
pub async fn settings_reset(db: tauri::State<'_, SqlitePool>, key: String) -> Result<(), String> {
    reset_inner(db.inner(), &key).await
}

#[command]
pub async fn settings_reset_many(
    db: tauri::State<'_, SqlitePool>,
    keys: Vec<String>,
) -> Result<(), String> {
    reset_many_inner(db.inner(), &keys).await
}

#[command]
pub async fn settings_reset_all(db: tauri::State<'_, SqlitePool>) -> Result<(), String> {
    reset_all_inner(db.inner()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn seed_is_idempotent() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();
        seed_settings(&pool).await.unwrap();

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM setting")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, i64::try_from(seeds().len()).unwrap());
    }

    #[tokio::test]
    async fn seed_preserves_existing_value() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();
        set_inner(&pool, "terminal.appearance.font_size", "20")
            .await
            .unwrap();
        seed_settings(&pool).await.unwrap();

        let entry = get_inner(&pool, "terminal.appearance.font_size")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value, "20");
        assert_eq!(entry.default_value, "14");
    }

    #[tokio::test]
    async fn set_updates_value_only() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();
        set_inner(&pool, "ai.max_tokens", "48").await.unwrap();

        let entry = get_inner(&pool, "ai.max_tokens").await.unwrap().unwrap();
        assert_eq!(entry.value, "48");
        assert_eq!(entry.default_value, "32");
        assert_eq!(entry.value_type, "number");
    }

    #[tokio::test]
    async fn set_many_writes_in_transaction() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let inputs = vec![
            SettingInput {
                key: "terminal.behavior.scrollback".into(),
                value: "2000".into(),
            },
            SettingInput {
                key: "ssh.keepalive_interval".into(),
                value: String::new(),
            },
        ];
        set_many_inner(&pool, &inputs).await.unwrap();

        let scrollback = get_inner(&pool, "terminal.behavior.scrollback")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(scrollback.value, "2000");

        let keepalive = get_inner(&pool, "ssh.keepalive_interval")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(keepalive.value, "");
    }

    #[tokio::test]
    async fn reset_restores_default() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();
        set_inner(&pool, "ai.temperature", "0.9").await.unwrap();
        reset_inner(&pool, "ai.temperature").await.unwrap();

        let entry = get_inner(&pool, "ai.temperature").await.unwrap().unwrap();
        assert_eq!(entry.value, "0.2");
    }

    #[tokio::test]
    async fn reset_all_restores_every_row() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();
        set_inner(&pool, "ai.temperature", "0.9").await.unwrap();
        set_inner(&pool, "terminal.appearance.font_size", "20")
            .await
            .unwrap();
        reset_all_inner(&pool).await.unwrap();

        let temperature = get_inner(&pool, "ai.temperature").await.unwrap().unwrap();
        assert_eq!(temperature.value, "0.2");

        let font_size = get_inner(&pool, "terminal.appearance.font_size")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(font_size.value, "14");
    }

    #[tokio::test]
    async fn get_all_returns_every_seeded_key() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let all = get_all_inner(&pool).await.unwrap();
        assert_eq!(all.len(), seeds().len());

        for (key, _, _) in seeds() {
            assert!(
                all.iter().any(|e| e.key == key),
                "missing seeded key: {key}"
            );
        }
        for entry in &all {
            assert!(!entry.created_at.is_empty());
            assert!(!entry.updated_at.is_empty());
        }
    }

    #[tokio::test]
    async fn get_unknown_key_returns_none() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        assert!(get_inner(&pool, "does.not.exist").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_returns_full_row() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let entry = get_inner(&pool, "terminal.appearance.cursor_blink")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.key, "terminal.appearance.cursor_blink");
        assert_eq!(entry.value, "true");
        assert_eq!(entry.default_value, "true");
        assert_eq!(entry.value_type, "boolean");
        assert!(!entry.created_at.is_empty());
        assert!(!entry.updated_at.is_empty());
    }

    #[tokio::test]
    async fn set_unknown_key_is_noop() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM setting")
            .fetch_one(&pool)
            .await
            .unwrap();

        set_inner(&pool, "does.not.exist", "x").await.unwrap();

        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM setting")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(after, before);
        assert!(get_inner(&pool, "does.not.exist").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn set_does_not_change_created_at() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let before = get_inner(&pool, "ai.enabled").await.unwrap().unwrap();
        set_inner(&pool, "ai.enabled", "true").await.unwrap();
        let after = get_inner(&pool, "ai.enabled").await.unwrap().unwrap();

        assert_eq!(after.value, "true");
        assert_eq!(after.default_value, "false");
        assert_eq!(after.created_at, before.created_at);
    }

    #[tokio::test]
    async fn set_many_empty_is_noop() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        set_many_inner(&pool, &[]).await.unwrap();
    }

    #[tokio::test]
    async fn set_many_ignores_unknown_keys() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let inputs = vec![
            SettingInput {
                key: "ai.context_lines".into(),
                value: "50".into(),
            },
            SettingInput {
                key: "does.not.exist".into(),
                value: "x".into(),
            },
        ];
        set_many_inner(&pool, &inputs).await.unwrap();

        let context_lines = get_inner(&pool, "ai.context_lines").await.unwrap().unwrap();
        assert_eq!(context_lines.value, "50");
        assert!(get_inner(&pool, "does.not.exist").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn reset_preserves_default_value() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        set_inner(&pool, "terminal.behavior.scrollback", "5000")
            .await
            .unwrap();
        reset_inner(&pool, "terminal.behavior.scrollback")
            .await
            .unwrap();

        let entry = get_inner(&pool, "terminal.behavior.scrollback")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value, "1000");
        assert_eq!(entry.default_value, "1000");
    }

    #[tokio::test]
    async fn reset_unknown_key_is_noop() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        reset_inner(&pool, "does.not.exist").await.unwrap();
    }

    #[tokio::test]
    async fn reset_all_restores_null_defaults() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        set_inner(&pool, "ssh.keepalive_interval", "60")
            .await
            .unwrap();
        set_inner(&pool, "ssh.connect_timeout", "30").await.unwrap();
        reset_all_inner(&pool).await.unwrap();

        let keepalive = get_inner(&pool, "ssh.keepalive_interval")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(keepalive.value, "");
        assert_eq!(keepalive.value_type, "null");

        let timeout = get_inner(&pool, "ssh.connect_timeout")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(timeout.value, "");
        assert_eq!(timeout.value_type, "null");
    }

    #[tokio::test]
    async fn seed_entries_have_valid_value_types() {
        const VALID_TYPES: [&str; 5] = ["string", "number", "boolean", "null", "json"];

        for (key, value, value_type) in seeds() {
            assert!(
                VALID_TYPES.contains(&value_type),
                "invalid value_type for {key}: {value_type}"
            );
            if value_type == "null" {
                assert!(value.is_empty(), "null default for {key} must be empty");
            }
        }
    }

    #[tokio::test]
    async fn reset_many_restores_defaults() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        set_inner(&pool, "ai.temperature", "0.9").await.unwrap();
        set_inner(&pool, "ai.max_tokens", "64").await.unwrap();
        reset_many_inner(
            &pool,
            &["ai.temperature".to_string(), "ai.max_tokens".to_string()],
        )
        .await
        .unwrap();

        assert_eq!(
            get_inner(&pool, "ai.temperature")
                .await
                .unwrap()
                .unwrap()
                .value,
            "0.2"
        );
        assert_eq!(
            get_inner(&pool, "ai.max_tokens")
                .await
                .unwrap()
                .unwrap()
                .value,
            "32"
        );
    }

    #[tokio::test]
    async fn seed_color_scheme_is_valid_json() {
        let pool = db::test_pool().await;
        seed_settings(&pool).await.unwrap();

        let entry = get_inner(&pool, "terminal.appearance.color_scheme")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.value_type, "json");

        let theme: serde_json::Value = serde_json::from_str(&entry.value).unwrap();
        assert_eq!(theme["background"].as_str(), Some("#1e1e2e"));
        assert_eq!(theme["foreground"].as_str(), Some("#cdd6f4"));
        assert_eq!(theme["cursorAccent"].as_str(), Some("#1e1e2e"));
        assert_eq!(theme["selectionBackground"].as_str(), Some("#f5e0dc"));
        assert_eq!(theme["selectionForeground"].as_str(), Some("#1e1e2e"));
    }
}
