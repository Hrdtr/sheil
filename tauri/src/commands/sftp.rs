use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{command, State};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::commands::ssh::SshState;

// ---------------------------------------------------------------------------
// Serializable types
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    /// ISO 8601 modified time (e.g. "2024-01-15T10:30:00Z")
    pub modified: Option<String>,
    /// Unix permission bits (e.g. 0o644)
    pub permissions: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SftpMetadata {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    /// ISO 8601 modified time
    pub modified: Option<String>,
    /// ISO 8601 accessed time
    pub accessed: Option<String>,
    /// Unix permission bits
    pub permissions: u32,
    pub owner: Option<u32>,
    pub group: Option<u32>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `SystemTime` to an ISO 8601 UTC string.
fn system_time_to_iso8601(time: SystemTime) -> String {
    let Ok(duration) = time.duration_since(UNIX_EPOCH) else {
        return String::from("1970-01-01T00:00:00Z");
    };

    let total_secs = duration.as_secs();
    let sec_of_day = total_secs % 86_400;
    let hour = sec_of_day / 3_600;
    let minute = (sec_of_day % 3_600) / 60;
    let second = sec_of_day % 60;

    // Howard Hinnant civil-date algorithm (all integer arithmetic, no loops)
    let shifted_days = total_secs / 86_400 + 719_468;
    let era = shifted_days / 146_097;
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year_base = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 { month_prime + 3 } else { month_prime - 9 };
    let year = if month <= 2 { year_base + 1 } else { year_base };

    let result = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z");
    result
}

/// Convert a u32 Unix timestamp to an ISO 8601 string, if present.
fn mtime_to_iso8601(mtime: Option<u32>) -> Option<String> {
    mtime.map(|t| {
        let d = Duration::from_secs(u64::from(t));
        system_time_to_iso8601(UNIX_EPOCH + d)
    })
}

// ---------------------------------------------------------------------------
// SFTP state
// ---------------------------------------------------------------------------

/// Holds active SFTP sessions, keyed by SSH session id.
/// One SFTP session per SSH connection.
pub struct SftpState {
    sessions: Mutex<HashMap<String, Arc<russh_sftp::client::SftpSession>>>,
}

impl SftpState {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
enum SftpError {
    #[error("SFTP error: {0}")]
    Sftp(String),
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SFTP session not found for SSH session: {0}")]
    SessionNotFound(String),
    #[error("SFTP already connected for SSH session: {0}")]
    AlreadyConnected(String),
}

impl From<SftpError> for String {
    fn from(e: SftpError) -> Self {
        e.to_string()
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve the SFTP session for a given SSH session id
// ---------------------------------------------------------------------------

impl SftpState {
    async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Arc<russh_sftp::client::SftpSession>, SftpError> {
        self.sessions
            .lock()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| SftpError::SessionNotFound(session_id.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Open an SFTP subsystem channel on an existing SSH session.
#[command]
pub async fn sftp_connect(
    ssh_state: State<'_, SshState>,
    sftp_state: State<'_, SftpState>,
    session_id: String,
) -> Result<(), String> {
    // Check if already connected
    if sftp_state.sessions.lock().await.contains_key(&session_id) {
        return Err(SftpError::AlreadyConnected(session_id).into());
    }

    // Open a new SSH channel for the SFTP subsystem
    let channel = ssh_state
        .channel_open_sftp(&session_id)
        .await
        .map_err(SftpError::Sftp)?;

    // Convert Channel to ChannelStream which implements AsyncRead + AsyncWrite
    let stream = channel.into_stream();

    // Initialize the SFTP session over the channel stream
    let sftp = russh_sftp::client::SftpSession::new(stream)
        .await
        .map_err(|e| SftpError::Sftp(format!("SFTP init failed: {e}")))?;

    sftp_state
        .sessions
        .lock()
        .await
        .insert(session_id.clone(), Arc::new(sftp));

    log::info!("SFTP session opened for SSH session {session_id}");
    Ok(())
}

/// Close the SFTP session for a given SSH session.
#[command]
pub async fn sftp_disconnect(
    sftp_state: State<'_, SftpState>,
    session_id: String,
) -> Result<(), String> {
    let sftp = sftp_state
        .sessions
        .lock()
        .await
        .remove(&session_id)
        .ok_or_else(|| SftpError::SessionNotFound(session_id.clone()))?;

    sftp.close()
        .await
        .map_err(|e| SftpError::Sftp(format!("SFTP close failed: {e}")))?;

    log::info!("SFTP session closed for SSH session {session_id}");
    Ok(())
}

/// List entries in a directory on the remote host.
#[command]
pub async fn sftp_list_dir(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<Vec<SftpEntry>, String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    let dir = sftp
        .read_dir(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("read_dir failed: {e}")))?;

    let mut entries = Vec::new();
    for entry in dir {
        let metadata = entry.metadata();
        let file_type = entry.file_type();
        let name = entry.file_name();

        entries.push(SftpEntry {
            path: format!("{}/{}", path.trim_end_matches('/'), name),
            name,
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
            size: metadata.len(),
            modified: mtime_to_iso8601(metadata.mtime),
            permissions: metadata.permissions.unwrap_or(0),
        });
    }

    // Sort: directories first, then alphabetical
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

/// Get metadata for a remote file or directory.
#[command]
pub async fn sftp_stat(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<SftpMetadata, String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    let metadata = sftp
        .metadata(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("stat failed: {e}")))?;

    Ok(SftpMetadata {
        is_dir: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        size: metadata.len(),
        modified: mtime_to_iso8601(metadata.mtime),
        accessed: mtime_to_iso8601(metadata.atime),
        permissions: metadata.permissions.unwrap_or(0),
        owner: metadata.uid,
        group: metadata.gid,
    })
}

/// Check if a path exists on the remote host.
#[command]
pub async fn sftp_exists(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<bool, String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    Ok(sftp
        .try_exists(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("exists check failed: {e}")))?)
}

/// Resolve a path to its canonical absolute form. Use "." for the home directory.
#[command]
pub async fn sftp_canonicalize(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<String, String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    Ok(sftp
        .canonicalize(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("canonicalize failed: {e}")))?)
}

/// Create a directory on the remote host.
#[command]
pub async fn sftp_create_dir(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    sftp.create_dir(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("create_dir failed: {e}")))?;

    Ok(())
}

/// Remove a file from the remote host.
#[command]
pub async fn sftp_remove_file(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    sftp.remove_file(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("remove_file failed: {e}")))?;

    Ok(())
}

/// Remove an empty directory from the remote host.
#[command]
pub async fn sftp_remove_dir(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    sftp.remove_dir(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("remove_dir failed: {e}")))?;

    Ok(())
}

/// Rename or move a file or directory on the remote host.
#[command]
pub async fn sftp_rename(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    sftp.rename(&old_path, &new_path)
        .await
        .map_err(|e| SftpError::Sftp(format!("rename failed: {e}")))?;

    Ok(())
}

/// Read a chunk of a remote file (for download or preview).
/// Uses `AsyncSeek` to position at the given offset.
#[command]
pub async fn sftp_read_file(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
    offset: u64,
    length: u64,
) -> Result<Vec<u8>, String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    let mut file = sftp
        .open(&path)
        .await
        .map_err(|e| SftpError::Sftp(format!("open failed: {e}")))?;

    // Seek to offset
    file.seek(std::io::SeekFrom::Start(offset))
        .await
        .map_err(|e| SftpError::Sftp(format!("seek failed: {e}")))?;

    let buf_len = usize::try_from(length).unwrap_or(usize::MAX);
    let mut buf: Vec<u8> = vec![0u8; buf_len];
    let n = {
        use tokio::io::AsyncReadExt;
        file.read(&mut buf).await
    }
    .map_err(|e| SftpError::Sftp(format!("read failed: {e}")))?;

    buf.truncate(n);
    Ok(buf)
}

/// Write data to a remote file at a given offset.
/// For offset == 0, creates a new file; for offset > 0, reads-modifies-writes.
#[command]
pub async fn sftp_write_file(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    path: String,
    data: Vec<u8>,
    offset: u64,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    if offset == 0 {
        let mut file = sftp
            .create(&path)
            .await
            .map_err(|e| SftpError::Sftp(format!("create failed: {e}")))?;

        file.write_all(&data)
            .await
            .map_err(|e| SftpError::Sftp(format!("write failed: {e}")))?;

        file.flush()
            .await
            .map_err(|e| SftpError::Sftp(format!("flush failed: {e}")))?;
    } else {
        // Read existing content, patch, and rewrite
        let existing = sftp
            .read(&path)
            .await
            .map_err(|e| SftpError::Sftp(format!("read existing file failed: {e}")))?;

        let offset_usize = usize::try_from(offset).unwrap_or(usize::MAX);
        let end = (offset_usize + data.len()).max(existing.len());
        let mut new_content = vec![0u8; end];

        // Copy existing content before offset
        let before_len = existing.len().min(offset_usize);
        new_content[..before_len].copy_from_slice(&existing[..before_len]);

        // Write new data at offset
        let data_end = offset_usize + data.len();
        new_content[offset_usize..data_end].copy_from_slice(&data);

        // Copy existing content after the written region
        if existing.len() > data_end {
            new_content[data_end..].copy_from_slice(&existing[data_end..]);
        }

        let mut file = sftp
            .create(&path)
            .await
            .map_err(|e| SftpError::Sftp(format!("create failed: {e}")))?;

        file.write_all(&new_content)
            .await
            .map_err(|e| SftpError::Sftp(format!("write failed: {e}")))?;

        file.flush()
            .await
            .map_err(|e| SftpError::Sftp(format!("flush failed: {e}")))?;
    }

    Ok(())
}

/// Download a remote file to a local path.
#[command]
pub async fn sftp_download(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    let data = sftp
        .read(&remote_path)
        .await
        .map_err(|e| SftpError::Sftp(format!("download read failed: {e}")))?;

    // Write to local file
    std::fs::write(&local_path, &data).map_err(SftpError::Io)?;

    log::info!(
        "Downloaded {} bytes from {} to {}",
        data.len(),
        remote_path,
        local_path
    );
    Ok(())
}

/// Upload a local file to a remote path.
#[command]
pub async fn sftp_upload(
    sftp_state: State<'_, SftpState>,
    session_id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let sftp = sftp_state.get_session(&session_id).await?;

    let data = std::fs::read(&local_path).map_err(SftpError::Io)?;
    let size = data.len();

    let mut file = sftp
        .create(&remote_path)
        .await
        .map_err(|e| SftpError::Sftp(format!("upload create failed: {e}")))?;

    file.write_all(&data)
        .await
        .map_err(|e| SftpError::Sftp(format!("upload write failed: {e}")))?;

    file.flush()
        .await
        .map_err(|e| SftpError::Sftp(format!("upload flush failed: {e}")))?;

    log::info!("Uploaded {size} bytes from {local_path} to {remote_path}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    // -----------------------------------------------------------------------
    // Error conversions
    // -----------------------------------------------------------------------

    #[test]
    fn error_sftp_to_string() {
        let err = SftpError::Sftp("permission denied".to_string());
        let s: String = err.into();
        assert!(s.contains("permission denied"));
    }

    #[test]
    fn error_session_not_found_to_string() {
        let err = SftpError::SessionNotFound("abc123".to_string());
        let s: String = err.into();
        assert!(s.contains("abc123"));
    }

    #[test]
    fn error_already_connected_to_string() {
        let err = SftpError::AlreadyConnected("abc123".to_string());
        let s: String = err.into();
        assert!(s.contains("abc123"));
    }

    // -----------------------------------------------------------------------
    // Serializable types
    // -----------------------------------------------------------------------

    #[test]
    fn sftp_entry_serialization() {
        let entry = SftpEntry {
            name: "readme.txt".to_string(),
            path: "/home/user/readme.txt".to_string(),
            is_dir: false,
            is_symlink: false,
            size: 1024,
            modified: Some("2024-01-15T10:30:00Z".to_string()),
            permissions: 0o644,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("readme.txt"));
        assert!(json.contains("camelCase") || json.contains("isDir")); // serde renamed
    }

    #[test]
    fn sftp_metadata_serialization() {
        let meta = SftpMetadata {
            is_dir: true,
            is_symlink: false,
            size: 4096,
            modified: Some("2024-01-15T10:30:00Z".to_string()),
            accessed: None,
            permissions: 0o755,
            owner: Some(1000),
            group: Some(1000),
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("4096"));
        assert!(json.contains("755") || json.contains("permissions"));
    }

    // -----------------------------------------------------------------------
    // Date / time helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_unix_epoch() {
        assert_eq!(
            system_time_to_iso8601(UNIX_EPOCH),
            "1970-01-01T00:00:00Z"
        );
    }

    #[test]
    fn test_known_date() {
        // 2024-01-15T10:30:00Z
        // 1970-01-01 to 2024-01-01 = 54 years * 365 + 13 leap days = 19723 days
        // + 14 days to Jan 15 = 19737 days * 86400 + 10*3600 + 30*60 = 1_705_314_600
        let t = UNIX_EPOCH + Duration::from_secs(1_705_314_600);
        assert_eq!(
            system_time_to_iso8601(t),
            "2024-01-15T10:30:00Z"
        );
    }

    #[test]
    fn test_leap_day() {
        // 2024-02-29T12:00:00Z
        // 2024-01-01 = 1704067200
        // + 31 (Jan) + 28 (Feb 1-28) = 59 days
        // 1704067200 + 59*86400 + 12*3600 = 1704067200 + 5097600 + 43200 = 1709208000
        let t = UNIX_EPOCH + Duration::from_secs(1_709_208_000);
        assert_eq!(
            system_time_to_iso8601(t),
            "2024-02-29T12:00:00Z"
        );
    }

    #[test]
    fn test_pre_epoch_fallback() {
        // 1969-12-31T23:00:00Z — before epoch, should fall back
        let t = UNIX_EPOCH - Duration::from_secs(3600);
        assert_eq!(
            system_time_to_iso8601(t),
            "1970-01-01T00:00:00Z"
        );
    }

    #[test]
    fn test_mtime_to_iso8601_some() {
        // 2024-01-15T10:30:00Z = 1_705_314_600 seconds since epoch
        let result = mtime_to_iso8601(Some(1_705_314_600));
        assert_eq!(result, Some("2024-01-15T10:30:00Z".to_string()));
    }

    #[test]
    fn test_mtime_to_iso8601_none() {
        assert_eq!(mtime_to_iso8601(None), None);
    }

    #[test]
    fn test_year_boundary() {
        // 2023-12-31T23:59:59Z
        // 1970-01-01 to 2024-01-01 = 19723 days
        // 2023-12-31T23:59:59Z = 2024-01-01T00:00:00Z minus 1 second
        // 19723*86400 - 1 = 1_704_067_199
        let t = UNIX_EPOCH + Duration::from_secs(1_704_067_199);
        assert_eq!(
            system_time_to_iso8601(t),
            "2023-12-31T23:59:59Z"
        );
    }

    #[test]
    fn test_month_boundary() {
        // 2024-02-01T00:00:00Z
        // 1970-01-01 to 2024-01-01 = 19723 days + 31 (Jan) = 19754 days
        // 19754 * 86400 = 1_706_745_600
        let t = UNIX_EPOCH + Duration::from_secs(1_706_745_600);
        assert_eq!(
            system_time_to_iso8601(t),
            "2024-02-01T00:00:00Z"
        );
    }

    #[test]
    fn test_epoch_plus_one_second() {
        let t = UNIX_EPOCH + Duration::from_secs(1);
        assert_eq!(
            system_time_to_iso8601(t),
            "1970-01-01T00:00:01Z"
        );
    }
}
