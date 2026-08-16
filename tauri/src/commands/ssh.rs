use std::collections::HashMap;
use std::sync::Arc;

use russh::client;
use russh::keys::{Algorithm, HashAlg, PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use russh::{Channel, ChannelMsg, ChannelWriteHalf};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::{command, Emitter};
use tokio::sync::Mutex;

use crate::credentials;
use crate::MasterKey;

pub struct SshState {
    sessions: Mutex<HashMap<String, client::Handle<Client>>>,
    channels: Mutex<HashMap<String, ChannelWriteHalf<client::Msg>>>,
    db: SqlitePool,
}

impl SshState {
    pub fn new(db: SqlitePool) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
            db,
        }
    }

    /// Open a direct-tcpip channel on a session. Used by port forwarding.
    /// Holds the sessions lock across the async call (tokio Mutex guard is Send).
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub(crate) async fn open_direct_tcpip_channel(
        &self,
        session_id: &str,
        remote_host: &str,
        remote_port: u32,
        originator_addr: &str,
        originator_port: u32,
    ) -> Result<Channel<client::Msg>, String> {
        let sessions = self.sessions.lock().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        handle
            .channel_open_direct_tcpip(remote_host, remote_port, originator_addr, originator_port)
            .await
            .map_err(|e| format!("SSH error: {e}"))
    }

    /// Cancel a remote TCP/IP forward on a session.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub(crate) async fn cancel_tcpip_forward(
        &self,
        session_id: &str,
        address: &str,
        port: u32,
    ) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        handle
            .cancel_tcpip_forward(address, port)
            .await
            .map_err(|e| format!("SSH error: {e}"))
    }

    /// Initiate a remote TCP/IP forward on a session.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    pub(crate) async fn tcpip_forward(
        &self,
        session_id: &str,
        address: &str,
        port: u32,
    ) -> Result<u32, String> {
        let sessions = self.sessions.lock().await;
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        handle
            .tcpip_forward(address, port)
            .await
            .map_err(|e| format!("SSH error: {e}"))
    }

    /// Open an SFTP subsystem channel on a session. Returns the channel
    /// initialized with the "sftp" subsystem requested.
    pub(crate) async fn channel_open_sftp(
        &self,
        session_id: &str,
    ) -> Result<Channel<client::Msg>, String> {
        let mut sessions = self.sessions.lock().await;
        let handle = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("session not found: {session_id}"))?;
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| format!("SSH error: {e}"))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("SSH error: {e}"))?;
        Ok(channel)
    }
}

#[derive(Clone)]
struct Client {
    host: String,
    port: u16,
    db: SqlitePool,
    /// Sender for dispatching incoming forwarded-tcpip channels (remote forwarding).
    remote_forward_tx: Option<tokio::sync::mpsc::Sender<super::port_forward::RemoteForwardEvent>>,
    /// Session id, included in each forwarded event.
    session_id: Option<String>,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        let key_type = server_public_key.algorithm().to_string();

        let known_fp: Option<String> = sqlx::query_scalar(
            "SELECT \"fingerprint\" FROM known_host WHERE \"host\" = ? AND \"port\" = ?",
        )
        .bind(&self.host)
        .bind(i64::from(self.port))
        .fetch_optional(&self.db)
        .await
        .map_err(|e| russh::Error::IO(std::io::Error::other(e)))?;

        match known_fp {
            Some(stored) if stored == fingerprint => Ok(true),
            Some(stored) => {
                log::error!(
                    "Host key mismatch for {}:{} — expected {}, got {}",
                    self.host,
                    self.port,
                    stored,
                    fingerprint
                );
                Ok(false)
            }
            None => {
                sqlx::query(
                    "INSERT INTO known_host (\"host\", \"port\", \"key_type\", \"fingerprint\") VALUES (?, ?, ?, ?)",
                )
                .bind(&self.host)
                .bind(i64::from(self.port))
                .bind(&key_type)
                .bind(&fingerprint)
                .execute(&self.db)
                .await
                .map_err(|e| russh::Error::IO(std::io::Error::other(e)))?;

                log::info!(
                    "Trusted new host key for {}:{} (type: {}, fingerprint: {})",
                    self.host,
                    self.port,
                    key_type,
                    fingerprint
                );
                Ok(true)
            }
        }
    }

    /// Handle incoming forwarded-tcpip channels (remote forwarding).
    /// Sends the channel to the forwarding engine via mpsc.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        _session: &mut russh::client::Session,
    ) -> impl core::future::Future<Output = Result<(), Self::Error>> + Send {
        use super::port_forward::RemoteForwardEvent;

        let tx = self.remote_forward_tx.clone();
        let sid = self.session_id.clone();
        let connected_address = connected_address.to_string();
        let originator_address = originator_address.to_string();

        async move {
            if let (Some(tx), Some(sid)) = (tx, sid) {
                let event = RemoteForwardEvent {
                    channel,
                    connected_address,
                    connected_port,
                    originator_address,
                    originator_port,
                    session_id: sid,
                };
                let _ = tx.send(event).await;
            }
            Ok(())
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum SshAuth {
    Password(String),
    Key(String),
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SshOutputEvent {
    session_id: String,
    data: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SshError {
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("Key error: {0}")]
    Key(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
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

pub(crate) fn parse_private_key(
    key_data: &str,
    passphrase: Option<&str>,
) -> Result<PrivateKey, SshError> {
    let mut key = PrivateKey::from_openssh(key_data).map_err(|e| SshError::Key(e.to_string()))?;
    if key.is_encrypted() {
        let pw = passphrase
            .ok_or_else(|| SshError::Key("key is encrypted but no passphrase provided".into()))?;
        key = key
            .decrypt(pw)
            .map_err(|e| SshError::Key(format!("failed to decrypt key: {e}")))?;
    }
    match key.algorithm() {
        Algorithm::Ed25519 | Algorithm::Rsa { .. } => Ok(key),
        other => Err(SshError::UnsupportedKeyType(other.to_string())),
    }
}

pub(crate) fn key_fingerprint(key: &PrivateKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// Try to parse a key for listing. Encrypted keys still show their
/// algorithm — only the fingerprint requires decryption.
pub(crate) fn try_parse_key_info(key_data: &str) -> Option<(String, String)> {
    let key = PrivateKey::from_openssh(key_data).ok()?;
    let key_type = key.algorithm().to_string();
    if key.is_encrypted() {
        return Some((key_type, "encrypted".into()));
    }
    Some((key_type, key_fingerprint(&key)))
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn ssh_connect(
    state: tauri::State<'_, SshState>,
    master_key: tauri::State<'_, MasterKey>,
    fwd_state: tauri::State<'_, super::port_forward::ForwardingState>,
    host: String,
    port: u16,
    username: String,
    auth: SshAuth,
    keepalive_interval: Option<u64>,
    connect_timeout: Option<u64>,
) -> Result<String, String> {
    let mut config = client::Config::default();

    if let Some(interval) = keepalive_interval {
        if interval > 0 {
            config.keepalive_interval = Some(std::time::Duration::from_secs(interval));
        }
    }

    let config = Arc::new(config);
    let addr = format!("{host}:{port}");
    let db = state.db.clone();
    let session_id = crate::db::new_table_row_id();

    // Get the remote-forward sender (only on desktop).
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    let remote_forward_tx = Some(fwd_state.remote_forward_sender());
    #[cfg(any(target_os = "ios", target_os = "android"))]
    let remote_forward_tx: Option<
        tokio::sync::mpsc::Sender<super::port_forward::RemoteForwardEvent>,
    > = None;

    let connect_fut = client::connect(
        config,
        addr.as_str(),
        Client {
            host: host.clone(),
            port,
            db,
            remote_forward_tx,
            session_id: Some(session_id.clone()),
        },
    );

    let mut handle = if let Some(timeout) = connect_timeout {
        if timeout > 0 {
            tokio::time::timeout(std::time::Duration::from_secs(timeout), connect_fut)
                .await
                .map_err(|_| SshError::Key("connection timed out".into()))?
                .map_err(SshError::Ssh)?
        } else {
            connect_fut.await.map_err(SshError::Ssh)?
        }
    } else {
        connect_fut.await.map_err(SshError::Ssh)?
    };

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
        SshAuth::Key(credential_id) => {
            let key_data = credentials::retrieve_value(&state.db, &master_key.0, &credential_id)
                .await
                .map_err(SshError::Encryption)?;
            let passphrase = credentials::retrieve_key_passphrase_value(
                &state.db,
                &master_key.0,
                &credential_id,
            )
            .await
            .map_err(SshError::Encryption)?;
            let key = parse_private_key(&key_data, passphrase.as_deref())?;
            let key_with_hash = PrivateKeyWithHashAlg::new(Arc::new(key), Some(HashAlg::Sha256));
            let result = handle
                .authenticate_publickey(&username, key_with_hash)
                .await
                .map_err(SshError::Ssh)?;
            if !result.success() {
                return Err(SshError::AuthFailed.into());
            }
        }
    }

    state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), handle);

    log::info!("SSH session {session_id} connected to {host}:{port}");
    Ok(session_id)
}

#[command]
pub async fn ssh_disconnect(
    state: tauri::State<'_, SshState>,
    fwd_state: tauri::State<'_, super::port_forward::ForwardingState>,
    session_id: String,
) -> Result<(), String> {
    // Clean up any active port forwarding tunnels for this session.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    fwd_state.stop_all_for_session(&session_id).await;

    state.channels.lock().await.remove(&session_id);
    match state.sessions.lock().await.remove(&session_id) {
        Some(_) => {
            log::info!("SSH session {session_id} disconnected");
            Ok(())
        }
        None => Err(SshError::SessionNotFound(session_id).into()),
    }
}

#[command]
pub async fn ssh_list_sessions(state: tauri::State<'_, SshState>) -> Result<Vec<String>, String> {
    Ok(state.sessions.lock().await.keys().cloned().collect())
}

#[command]
pub async fn ssh_open_channel(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().await;
    let handle = sessions
        .get_mut(&session_id)
        .ok_or_else(|| SshError::SessionNotFound(session_id.clone()))?;

    let channel = handle.channel_open_session().await.map_err(SshError::Ssh)?;

    channel
        .request_pty(false, "xterm-256color", cols, rows, 0, 0, &[])
        .await
        .map_err(SshError::Ssh)?;
    channel.request_shell(true).await.map_err(SshError::Ssh)?;

    let (mut read_half, write_half) = channel.split();

    // Spawn a task that reads from the SSH channel and emits data to the
    // frontend as window events.
    let emit_handle = app_handle.clone();
    let emit_session_id = session_id.clone();
    tokio::spawn(async move {
        loop {
            match read_half.wait().await {
                Some(ChannelMsg::Data { data }) => {
                    let _ = emit_handle.emit(
                        "ssh-output",
                        SshOutputEvent {
                            session_id: emit_session_id.clone(),
                            data: data.to_vec(),
                        },
                    );
                }
                Some(ChannelMsg::Eof) | None => {
                    let _ = emit_handle.emit("ssh-exit", emit_session_id);
                    break;
                }
                Some(_) => { /* ignore non-data messages */ }
            }
        }
    });

    state
        .channels
        .lock()
        .await
        .insert(session_id.clone(), write_half);
    log::info!("SSH channel opened for session {session_id}");
    Ok(())
}

#[command]
pub async fn ssh_write(
    state: tauri::State<'_, SshState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut channels = state.channels.lock().await;
    let channel = channels
        .get_mut(&session_id)
        .ok_or_else(|| SshError::ChannelNotFound(session_id.clone()))?;

    channel.data(&data[..]).await.map_err(SshError::Ssh)?;

    Ok(())
}

#[command]
pub async fn ssh_resize(
    state: tauri::State<'_, SshState>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let mut channels = state.channels.lock().await;
    let channel = channels
        .get_mut(&session_id)
        .ok_or_else(|| SshError::ChannelNotFound(session_id.clone()))?;

    channel
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
    let mut channels = state.channels.lock().await;
    if let Some(channel) = channels.remove(&session_id) {
        let _ = channel.eof().await;
    }
    Ok(())
}

#[command]
pub async fn known_host_clear_all(state: tauri::State<'_, SshState>) -> Result<u64, String> {
    let result = sqlx::query("DELETE FROM known_host")
        .execute(&state.db)
        .await
        .map_err(SshError::from)?;
    log::info!("Cleared {} known host entries", result.rows_affected());
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed25519_test_key() -> &'static str {
        include_str!("../../tests/fixtures/test_ed25519_key")
    }

    #[test]
    fn parse_ed25519_openssh_key() {
        let key = parse_private_key(ed25519_test_key(), None).unwrap();
        assert_eq!(key.algorithm().to_string(), "ssh-ed25519");
    }

    #[test]
    fn parse_invalid_key_data() {
        assert!(parse_private_key("not a key", None).is_err());
    }

    #[test]
    fn fingerprint_format() {
        let key = parse_private_key(ed25519_test_key(), None).unwrap();
        let fp = key_fingerprint(&key);
        assert!(fp.starts_with("SHA256:"));
    }

    #[test]
    fn ssh_error_display() {
        assert_eq!(SshError::AuthFailed.to_string(), "Authentication failed");
        assert_eq!(
            SshError::SessionNotFound("s1".into()).to_string(),
            "Session not found: s1"
        );
        assert_eq!(
            SshError::Encryption("bad key".into()).to_string(),
            "Encryption error: bad key"
        );
    }

    #[test]
    fn ssh_error_into_string() {
        let e: String = SshError::ChannelNotFound("c1".into()).into();
        assert_eq!(e, "Channel not found: c1");
    }

    #[test]
    fn ssh_error_ssh_variant() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        assert!(SshError::Ssh(russh::Error::IO(io_err))
            .to_string()
            .contains("refused"));
    }

    #[test]
    fn ssh_error_key_variant() {
        assert_eq!(
            SshError::Key("parse failure".into()).to_string(),
            "Key error: parse failure"
        );
    }

    #[test]
    fn ssh_error_unsupported_key_type() {
        assert_eq!(
            SshError::UnsupportedKeyType("dsa".into()).to_string(),
            "Unsupported key algorithm: dsa"
        );
    }

    #[test]
    fn ssh_error_session_not_found_roundtrip() {
        let s: String = SshError::SessionNotFound("abc".into()).into();
        assert_eq!(s, "Session not found: abc");
    }

    #[test]
    fn ssh_error_channel_not_found_roundtrip() {
        let s: String = SshError::ChannelNotFound("xyz".into()).into();
        assert_eq!(s, "Channel not found: xyz");
    }

    #[test]
    fn ssh_auth_password_serde() {
        let auth: SshAuth =
            serde_json::from_str(r#"{"type":"password","value":"hunter2"}"#).unwrap();
        match auth {
            SshAuth::Password(p) => assert_eq!(p, "hunter2"),
            SshAuth::Key(_) => panic!(),
        }
    }

    #[test]
    fn ssh_auth_key_serde() {
        let auth: SshAuth = serde_json::from_str(r#"{"type":"key","value":"my-ed25519"}"#).unwrap();
        match auth {
            SshAuth::Key(k) => assert_eq!(k, "my-ed25519"),
            SshAuth::Password(_) => panic!(),
        }
    }

    #[test]
    fn ssh_output_event_serialization() {
        let event = SshOutputEvent {
            session_id: "s1".into(),
            data: vec![104, 101, 108, 108, 111],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("s1"));
    }

    #[test]
    fn fingerprint_is_consistent() {
        let key = parse_private_key(ed25519_test_key(), None).unwrap();
        assert_eq!(
            key_fingerprint(&key),
            key_fingerprint(&parse_private_key(ed25519_test_key(), None).unwrap())
        );
    }

    #[test]
    fn ssh_state_channels_map_is_accessible() {
        let _state = SshState::new(tauri::async_runtime::block_on(crate::db::test_pool()));
    }
}
