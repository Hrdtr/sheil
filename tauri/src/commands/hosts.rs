use crate::secrets;
use crate::MasterKey;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::command;

/// Type alias used by the command signatures — mirrors `commands::SharedPool`.
pub type SharedPool = SqlitePool;

/// Service prefix for host password credentials.
const HOST_PASSWORD_SERVICE_PREFIX: &str = "sheil.host_password.";

#[derive(Debug, thiserror::Error)]
enum HostError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("host not found: {0}")]
    NotFound(String),
    #[error("encryption error: {0}")]
    Encryption(String),
}

impl From<HostError> for String {
    fn from(e: HostError) -> Self {
        e.to_string()
    }
}

fn map_host_err<E: std::fmt::Display>(e: E) -> HostError {
    HostError::Validation(e.to_string())
}

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInput {
    pub name: String,
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub group: Option<String>,
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    pub key_name: Option<String>,
    /// Optional plaintext password.  Encrypted and stored in `SQLite` alongside
    /// the host metadata; never stored in plaintext.
    pub password: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_port() -> u16 {
    22
}
fn default_protocol() -> String {
    "ssh".to_string()
}
fn default_auth_method() -> String {
    "password".to_string()
}

impl HostInput {
    fn validate(&self) -> Result<(), HostError> {
        if self.name.trim().is_empty() {
            return Err(HostError::Validation("name must not be empty".into()));
        }
        if self.host.trim().is_empty() {
            return Err(HostError::Validation("host must not be empty".into()));
        }
        if self.username.trim().is_empty() {
            return Err(HostError::Validation("username must not be empty".into()));
        }
        if self.protocol != "ssh" {
            return Err(HostError::Validation(format!(
                "unsupported protocol: {} (only ssh is supported in Phase 1)",
                self.protocol
            )));
        }
        if !matches!(self.auth_method.as_str(), "password" | "key") {
            return Err(HostError::Validation(format!(
                "invalid auth method: {} (expected 'password' or 'key')",
                self.auth_method
            )));
        }
        if self.auth_method == "key" && self.key_name.as_deref().map_or(true, str::is_empty) {
            return Err(HostError::Validation(
                "key_name is required when auth_method is 'key'".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub protocol: String,
    pub group: Option<String>,
    pub auth_method: String,
    pub key_name: Option<String>,
    pub tags: Vec<String>,
    pub has_password: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub struct HostUpdate {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub protocol: Option<String>,
    pub group: Option<Option<String>>,
    pub auth_method: Option<String>,
    pub key_name: Option<Option<String>>,
    pub password: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn host_password_service(id: &str) -> String {
    format!("{HOST_PASSWORD_SERVICE_PREFIX}{id}")
}

async fn store_host_password(
    pool: &SqlitePool,
    master_key: &[u8; crate::crypto::MASTER_KEY_SIZE],
    id: &str,
    password: &str,
) -> Result<(), HostError> {
    secrets::store(pool, master_key, &host_password_service(id), password)
        .await
        .map_err(HostError::Encryption)
}

async fn host_has_password(pool: &SqlitePool, id: &str) -> Result<bool, HostError> {
    secrets::exists(pool, &host_password_service(id))
        .await
        .map_err(HostError::Encryption)
}

async fn delete_host_password(pool: &SqlitePool, id: &str) -> Result<(), HostError> {
    secrets::delete(pool, &host_password_service(id))
        .await
        .map_err(HostError::Encryption)
}

// ── Row mapping ─────────────────────────────────────────────────────────────

async fn row_to_host(
    pool: &SqlitePool,
    row: &sqlx::sqlite::SqliteRow,
    id: &str,
) -> Result<Host, HostError> {
    let tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let has_password = host_has_password(pool, id).await?;

    Ok(Host {
        id: id.to_string(),
        name: row.try_get("name").unwrap_or_default(),
        host: row.try_get("host").unwrap_or_default(),
        port: u16::try_from(row.try_get::<i64, _>("port").unwrap_or(22)).unwrap_or(22),
        username: row.try_get("username").unwrap_or_default(),
        protocol: row
            .try_get("protocol")
            .unwrap_or_else(|_| "ssh".to_string()),
        group: row.try_get("group").ok().flatten(),
        auth_method: row
            .try_get("auth_method")
            .unwrap_or_else(|_| "password".to_string()),
        key_name: row.try_get("key_name").ok().flatten(),
        tags,
        has_password,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    })
}

// ── Commands ────────────────────────────────────────────────────────────────

#[command]
pub async fn host_create(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    input: HostInput,
) -> Result<Host, String> {
    host_create_inner(db.inner(), &master_key.0, input)
        .await
        .map_err(Into::into)
}

async fn host_create_inner(
    pool: &SqlitePool,
    master_key: &[u8; crate::crypto::MASTER_KEY_SIZE],
    input: HostInput,
) -> Result<Host, HostError> {
    input.validate()?;

    let id = crate::db::new_table_row_id();
    let tags_json = serde_json::to_string(&input.tags).map_err(map_host_err)?;

    sqlx::query(
        r#"INSERT INTO host ("id", "name", "host", "port", "username", "protocol", "group", "auth_method", "key_name", "tags")
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.host)
    .bind(i64::from(input.port))
    .bind(&input.username)
    .bind(&input.protocol)
    .bind(&input.group)
    .bind(&input.auth_method)
    .bind(&input.key_name)
    .bind(&tags_json)
    .execute(pool)
    .await?;

    if input.auth_method == "password" {
        if let Some(password) = input.password.as_deref() {
            if !password.is_empty() {
                store_host_password(pool, master_key, &id, password).await?;
            }
        }
    }

    let row = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    log::info!("Host '{}' created (id: {})", input.name, id);
    row_to_host(pool, &row, &id).await
}

#[command]
pub async fn host_list(db: tauri::State<'_, SharedPool>) -> Result<Vec<Host>, String> {
    let pool = db.inner();
    let rows = sqlx::query("SELECT * FROM host ORDER BY \"group\", \"name\"")
        .fetch_all(pool)
        .await
        .map_err(HostError::from)?;

    let mut hosts = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: String = row.try_get("id").unwrap_or_default();
        hosts.push(row_to_host(pool, row, &id).await?);
    }
    Ok(hosts)
}

#[command]
pub async fn host_resolve(db: tauri::State<'_, SharedPool>, id: String) -> Result<Host, String> {
    host_resolve_inner(db.inner(), &id)
        .await
        .map_err(Into::into)
}

async fn host_resolve_inner(pool: &SqlitePool, id: &str) -> Result<Host, HostError> {
    let row = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => row_to_host(pool, &row, id).await,
        None => Err(HostError::NotFound(id.to_string())),
    }
}

#[command]
pub async fn host_update(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    id: String,
    update: HostUpdate,
) -> Result<Host, String> {
    host_update_inner(db.inner(), &master_key.0, &id, update)
        .await
        .map_err(Into::into)
}

async fn host_update_inner(
    pool: &SqlitePool,
    master_key: &[u8; crate::crypto::MASTER_KEY_SIZE],
    id: &str,
    update: HostUpdate,
) -> Result<Host, HostError> {
    let existing = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = existing.ok_or_else(|| HostError::NotFound(id.to_string()))?;

    let current_name: String = row.try_get("name").unwrap_or_default();
    let current_host: String = row.try_get("host").unwrap_or_default();
    let current_port: i64 = row.try_get("port").unwrap_or(22);
    let current_username: String = row.try_get("username").unwrap_or_default();
    let current_protocol: String = row
        .try_get("protocol")
        .unwrap_or_else(|_| "ssh".to_string());
    let current_group: Option<String> = row.try_get("group").ok().flatten();
    let current_auth: String = row
        .try_get("auth_method")
        .unwrap_or_else(|_| "password".to_string());
    let current_key: Option<String> = row.try_get("key_name").ok().flatten();
    let current_tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());

    let new_name = update.name.unwrap_or(current_name);
    let new_host = update.host.unwrap_or(current_host);
    let new_port = update
        .port
        .unwrap_or(u16::try_from(current_port).unwrap_or(22));
    let new_username = update.username.unwrap_or(current_username);
    let new_protocol = update.protocol.unwrap_or(current_protocol);
    let new_group = update.group.unwrap_or(current_group);
    let new_auth = update.auth_method.unwrap_or(current_auth);
    let new_key = update.key_name.unwrap_or(current_key);
    let new_tags_json = if let Some(tags) = update.tags {
        serde_json::to_string(&tags).map_err(map_host_err)?
    } else {
        current_tags_json
    };

    if new_name.trim().is_empty() {
        return Err(HostError::Validation("name must not be empty".into()));
    }
    if new_host.trim().is_empty() {
        return Err(HostError::Validation("host must not be empty".into()));
    }
    if new_username.trim().is_empty() {
        return Err(HostError::Validation("username must not be empty".into()));
    }

    sqlx::query(
        r#"UPDATE host
           SET "name" = ?, "host" = ?, "port" = ?, "username" = ?, "protocol" = ?,
               "group" = ?, "auth_method" = ?, "key_name" = ?, "tags" = ?,
               "updated_at" = datetime('now')
           WHERE "id" = ?"#,
    )
    .bind(&new_name)
    .bind(&new_host)
    .bind(i64::from(new_port))
    .bind(&new_username)
    .bind(&new_protocol)
    .bind(&new_group)
    .bind(&new_auth)
    .bind(&new_key)
    .bind(&new_tags_json)
    .bind(id)
    .execute(pool)
    .await?;

    // Handle password: store new, clear on empty, clear on auth switch.
    // Single-key design — no username migration needed.
    log::info!(
        "Host '{}' update — auth={}, password_present={:?}",
        id,
        new_auth,
        update.password.is_some()
    );
    if new_auth == "password" {
        if let Some(password) = update.password.as_deref() {
            if password.is_empty() {
                log::info!("Host '{id}' — clearing stored password");
                delete_host_password(pool, id).await?;
            } else {
                log::info!("Host '{id}' — storing new password");
                store_host_password(pool, master_key, id, password).await?;
            }
        }
    } else {
        log::info!("Host '{id}' — auth switched to key, clearing password");
        delete_host_password(pool, id).await?;
    }

    let row = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    log::info!("Host '{new_name}' updated (id: {id})");
    row_to_host(pool, &row, id).await
}

#[command]
pub async fn host_delete(db: tauri::State<'_, SharedPool>, id: String) -> Result<(), String> {
    host_delete_inner(db.inner(), &id).await.map_err(Into::into)
}

async fn host_delete_inner(pool: &SqlitePool, id: &str) -> Result<(), HostError> {
    delete_host_password(pool, id).await?;

    sqlx::query("DELETE FROM host WHERE \"id\" = ?")
        .bind(id)
        .execute(pool)
        .await?;

    log::info!("Host deleted (id: {id})");
    Ok(())
}

/// Decrypts and returns a host's stored password.
#[command]
pub async fn host_resolve_password(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    id: String,
) -> Result<String, String> {
    secrets::retrieve(db.inner(), &master_key.0, &host_password_service(&id))
        .await
        .map_err(|_| "no password stored for this host".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportData {
    version: u32,
    hosts: Vec<Host>,
}

/// Export all hosts as JSON (passwords excluded).
#[command]
pub async fn host_export(db: tauri::State<'_, SharedPool>) -> Result<String, String> {
    let hosts = host_list_inner(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let data = ExportData { version: 1, hosts };
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

async fn host_list_inner(pool: &SqlitePool) -> Result<Vec<Host>, HostError> {
    let rows = sqlx::query("SELECT * FROM host ORDER BY \"group\", \"name\"")
        .fetch_all(pool)
        .await?;

    let mut hosts = Vec::with_capacity(rows.len());
    for row in &rows {
        let id: String = row.try_get("id").unwrap_or_default();
        hosts.push(row_to_host(pool, row, &id).await?);
    }
    Ok(hosts)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub failed: Vec<String>,
}

/// Import hosts from JSON. Skips duplicates by (host, port, username).
#[command]
pub async fn host_import(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    json: String,
) -> Result<ImportResult, String> {
    let data: ExportData = serde_json::from_str(&json).map_err(|e| format!("invalid JSON: {e}"))?;

    let pool = db.inner();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for input in data.hosts {
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM host WHERE \"host\" = ? AND \"port\" = ? AND \"username\" = ?",
        )
        .bind(&input.host)
        .bind(i64::from(input.port))
        .bind(&input.username)
        .fetch_one(pool)
        .await;

        match existing {
            Ok(count) if count > 0 => {
                skipped += 1;
                continue;
            }
            Err(e) => {
                failed.push(format!("{}: {e}", input.name));
                continue;
            }
            _ => {}
        }

        let host_input = HostInput {
            name: input.name.clone(),
            host: input.host.clone(),
            port: input.port,
            username: input.username.clone(),
            protocol: input.protocol.clone(),
            group: input.group.clone(),
            auth_method: input.auth_method.clone(),
            key_name: input.key_name.clone(),
            password: None,
            tags: input.tags.clone(),
        };

        match host_create_inner(pool, &master_key.0, host_input).await {
            Ok(_) => imported += 1,
            Err(e) => failed.push(format!("{}: {e}", input.name)),
        }
    }

    log::info!(
        "Import complete: imported={}, skipped={}, failed={}",
        imported,
        skipped,
        failed.len()
    );

    Ok(ImportResult {
        imported,
        skipped,
        failed,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rand::Rng;

    fn sample_input() -> HostInput {
        HostInput {
            name: "test-host".into(),
            host: "10.0.0.1".into(),
            port: 22,
            username: "admin".into(),
            protocol: "ssh".into(),
            group: Some("Production".into()),
            auth_method: "key".into(),
            key_name: Some("laptop-ed25519".into()),
            tags: vec!["web".into(), "nginx".into()],
            password: None,
        }
    }

    fn test_master_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::rng().fill_bytes(&mut key);
        key
    }

    #[tokio::test]
    async fn create_and_list_host() {
        let pool = db::test_pool().await;
        let key = test_master_key();
        let host = host_create_inner(&pool, &key, sample_input())
            .await
            .unwrap();
        assert_eq!(host.name, "test-host");
        assert_eq!(host.port, 22);
        assert_eq!(host.protocol, "ssh");
        assert_eq!(host.tags, vec!["web".to_string(), "nginx".to_string()]);

        let fetched = host_resolve_inner(&pool, &host.id).await.unwrap();
        assert_eq!(fetched.id, host.id);
        assert_eq!(fetched.name, "test-host");
    }

    #[tokio::test]
    async fn delete_removes_host() {
        let pool = db::test_pool().await;
        let key = test_master_key();
        let host = host_create_inner(&pool, &key, sample_input())
            .await
            .unwrap();

        host_delete_inner(&pool, &host.id).await.unwrap();

        let result = host_resolve_inner(&pool, &host.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let pool = db::test_pool().await;
        let key = test_master_key();
        let host = host_create_inner(&pool, &key, sample_input())
            .await
            .unwrap();

        let updated = host_update_inner(
            &pool,
            &key,
            &host.id,
            HostUpdate {
                name: Some("renamed".into()),
                port: Some(2222),
                host: None,
                username: None,
                protocol: None,
                group: None,
                auth_method: None,
                key_name: None,
                tags: None,
                password: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.port, 2222);
        assert_eq!(updated.host, "10.0.0.1");
    }

    #[test]
    fn validation_rejects_empty_name() {
        let mut input = sample_input();
        input.name = String::new();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_empty_host() {
        let mut input = sample_input();
        input.host = "  ".into();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_key_auth_without_key_name() {
        let mut input = sample_input();
        input.auth_method = "key".into();
        input.key_name = None;
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_unsupported_protocol() {
        let mut input = sample_input();
        input.protocol = "telnet".into();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_accepts_valid_input() {
        assert!(sample_input().validate().is_ok());
    }

    #[test]
    fn password_service_is_namespaced() {
        assert_eq!(
            host_password_service("host_abc"),
            "sheil.host_password.host_abc"
        );
    }

    #[test]
    fn new_id_is_unique() {
        let a = crate::db::new_table_row_id();
        let b = crate::db::new_table_row_id();
        assert_ne!(a, b);
    }
}
