use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use russh::client;
use russh::keys::{Algorithm, HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use russh::{ChannelMsg, ChannelWriteHalf};
use serde::{Deserialize, Serialize};
use tauri::{command, Emitter};
use tokio::sync::Mutex;

const SSH_KEYS_SERVICE: &str = "dev.hrdtr.sheil.ssh_keys";

pub struct SshState {
    sessions: Mutex<HashMap<String, client::Handle<Client>>>,
    channels: Mutex<HashMap<String, ChannelWriteHalf<client::Msg>>>,
}

impl SshState {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
        }
    }
}

fn next_session_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

#[derive(Clone)]
struct Client;

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum SshAuth {
    Password(String),
    Key(String),
}

#[derive(Serialize)]
pub struct ImportedKeyInfo {
    name: String,
    key_type: String,
    fingerprint: String,
}

#[derive(Clone, Serialize)]
struct SshOutputEvent {
    session_id: String,
    data: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
enum SshError {
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("Key error: {0}")]
    Key(String),
    #[error("Keychain error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),
    #[error("Unsupported key algorithm: {0}")]
    UnsupportedKeyType(String),
    #[error("Authentication failed")]
    AuthFailed,
}

impl From<SshError> for String {
    fn from(e: SshError) -> Self {
        e.to_string()
    }
}

fn store_ssh_key(name: &str, key_data: &str) -> Result<(), SshError> {
    let entry = keyring::Entry::new(SSH_KEYS_SERVICE, name)?;
    entry.set_password(key_data)?;
    Ok(())
}

fn retrieve_ssh_key(name: &str) -> Result<String, SshError> {
    let entry = keyring::Entry::new(SSH_KEYS_SERVICE, name)?;
    Ok(entry.get_password()?)
}

fn parse_private_key(key_data: &str) -> Result<PrivateKey, SshError> {
    let key = PrivateKey::from_openssh(key_data).map_err(|e| SshError::Key(e.to_string()))?;

    let algorithm = key.algorithm();
    match algorithm {
        Algorithm::Ed25519 | Algorithm::Rsa { .. } => Ok(key),
        other => Err(SshError::UnsupportedKeyType(other.to_string())),
    }
}

fn key_fingerprint(key: &PrivateKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

#[command]
pub async fn ssh_connect(
    state: tauri::State<'_, SshState>,
    host: String,
    port: u16,
    username: String,
    auth: SshAuth,
) -> Result<String, String> {
    let config = Arc::new(client::Config::default());
    let addr = format!("{}:{}", host, port);

    let mut handle = client::connect(config, addr.as_str(), Client)
        .await
        .map_err(SshError::Ssh)?;

    match auth {
        SshAuth::Password(password) => {
            let result = handle
                .authenticate_password(&username, &password)
                .await
                .map_err(SshError::Ssh)?;

            if !result.success() {
                return Err(SshError::AuthFailed.into());
            }
        }
        SshAuth::Key(key_name) => {
            let key_data = retrieve_ssh_key(&key_name)?;
            let key = parse_private_key(&key_data)?;
            let key_with_hash =
                PrivateKeyWithHashAlg::new(Arc::new(key), Some(HashAlg::Sha256));

            let result = handle
                .authenticate_publickey(&username, key_with_hash)
                .await
                .map_err(SshError::Ssh)?;

            if !result.success() {
                return Err(SshError::AuthFailed.into());
            }
        }
    }

    let session_id = next_session_id();
    state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), handle);

    log::info!("SSH session {} connected to {}:{}", session_id, host, port);
    Ok(session_id)
}

#[command]
pub async fn ssh_disconnect(
    state: tauri::State<'_, SshState>,
    session_id: String,
) -> Result<(), String> {
    state.channels.lock().await.remove(&session_id);
    let removed = state.sessions.lock().await.remove(&session_id);

    match removed {
        Some(_handle) => {
            log::info!("SSH session {} disconnected", session_id);
            Ok(())
        }
        None => Err(SshError::SessionNotFound(session_id).into()),
    }
}

#[command]
pub async fn ssh_import_key(name: String, key_data: String) -> Result<ImportedKeyInfo, String> {
    let key = parse_private_key(&key_data).map_err(|e| e.to_string())?;

    let key_type = key.algorithm().to_string();
    let fingerprint = key_fingerprint(&key);

    store_ssh_key(&name, &key_data).map_err(|e| e.to_string())?;

    log::info!(
        "SSH key '{}' imported (type: {}, fingerprint: {})",
        name,
        key_type,
        fingerprint
    );

    Ok(ImportedKeyInfo {
        name,
        key_type,
        fingerprint,
    })
}

#[command]
pub async fn ssh_list_sessions(
    state: tauri::State<'_, SshState>,
) -> Result<Vec<String>, String> {
    let sessions = state.sessions.lock().await;
    Ok(sessions.keys().cloned().collect())
}

#[command]
pub async fn ssh_open_channel(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let sessions = state.sessions.lock().await;
    let handle = sessions
        .get(&session_id)
        .ok_or_else(|| SshError::SessionNotFound(session_id.clone()))?;

    let channel = handle
        .channel_open_session()
        .await
        .map_err(SshError::Ssh)?;

    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
        .await
        .map_err(SshError::Ssh)?;

    channel
        .request_shell(false)
        .await
        .map_err(SshError::Ssh)?;

    let (mut read_half, write_half) = channel.split();
    drop(sessions);

    state
        .channels
        .lock()
        .await
        .insert(session_id.clone(), write_half);

    let sid = session_id.clone();
    tokio::spawn(async move {
        loop {
            match read_half.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    let _ = app_handle.emit(
                        "ssh-output",
                        SshOutputEvent {
                            session_id: sid.clone(),
                            data: data.to_vec(),
                        },
                    );
                }
                Some(ChannelMsg::ExitStatus { .. }) | None => {
                    let _ = app_handle.emit(
                        "ssh-exit",
                        serde_json::json!({ "sessionId": sid }),
                    );
                    break;
                }
                _ => {}
            }
        }
    });

    log::info!("PTY channel opened on session {}", session_id);
    Ok(())
}

#[command]
pub async fn ssh_write(
    state: tauri::State<'_, SshState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let channels = state.channels.lock().await;
    let write_half = channels
        .get(&session_id)
        .ok_or_else(|| SshError::ChannelNotFound(session_id.clone()))?;

    write_half
        .data_bytes(data)
        .await
        .map_err(SshError::Ssh)?;

    Ok(())
}

#[command]
pub async fn ssh_resize(
    state: tauri::State<'_, SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let channels = state.channels.lock().await;
    let write_half = channels
        .get(&session_id)
        .ok_or_else(|| SshError::ChannelNotFound(session_id.clone()))?;

    write_half
        .window_change(cols, rows, 0, 0)
        .await
        .map_err(SshError::Ssh)?;

    Ok(())
}

#[command]
pub async fn ssh_close_channel(
    state: tauri::State<'_, SshState>,
    session_id: String,
) -> Result<(), String> {
    let removed = state.channels.lock().await.remove(&session_id);
    if removed.is_some() {
        log::info!("PTY channel closed on session {}", session_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed25519_test_key() -> &'static str {
        "-----BEGIN OPENSSH PRIVATE KEY-----\n\
         b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
         QyNTUxOQAAACCzPq7zfqLffKoBDe/eo04kH2XxtSmk9D7RQyf1xUqrYgAAAJgAIAxdACAM\n\
         XQAAAAtzc2gtZWQyNTUxOQAAACCzPq7zfqLffKoBDe/eo04kH2XxtSmk9D7RQyf1xUqrYg\n\
         AAAEC2BsIi0QwW2uFscKTUUXNHLsYX4FxlaSDSblbAj7WR7bM+rvN+ot98qgEN796jTiQf\n\
         ZfG1KaT0PtFDJ/XFSqtiAAAAEHVzZXJAZXhhbXBsZS5jb20BAgMEBQ==\n\
         -----END OPENSSH PRIVATE KEY-----"
    }

    #[test]
    fn parse_ed25519_openssh_key() {
        let key_data = ed25519_test_key();
        let result = parse_private_key(key_data);
        assert!(result.is_ok());
        let key = result.unwrap();
        assert_eq!(key.algorithm(), Algorithm::Ed25519);
    }

    #[test]
    fn parse_invalid_key_data() {
        let key_data = "not a valid key";
        let result = parse_private_key(key_data);
        assert!(result.is_err());
    }

    #[test]
    fn next_session_id_is_unique() {
        let id1 = next_session_id();
        let id2 = next_session_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn fingerprint_format() {
        let key_data = ed25519_test_key();
        let key = parse_private_key(key_data).unwrap();
        let fp = key_fingerprint(&key);
        assert!(fp.starts_with("SHA256:"));
        assert!(fp.len() >= 50);
    }

    #[test]
    fn ssh_error_display() {
        let err = SshError::AuthFailed;
        assert_eq!(err.to_string(), "Authentication failed");

        let err = SshError::SessionNotFound("123".into());
        assert!(err.to_string().contains("123"));

        let err = SshError::ChannelNotFound("456".into());
        assert!(err.to_string().contains("456"));
    }

    #[test]
    fn ssh_error_into_string() {
        let err = SshError::AuthFailed;
        let s: String = err.into();
        assert_eq!(s, "Authentication failed");
    }
}
