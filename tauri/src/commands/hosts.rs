use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::command;

/// Type alias used by the command signatures — mirrors `commands::SharedPool`.
pub type SharedPool = SqlitePool;

#[derive(Debug, thiserror::Error)]
enum HostError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("host not found: {0}")]
    NotFound(String),
}

impl From<HostError> for String {
    fn from(e: HostError) -> Self {
        e.to_string()
    }
}

fn map_host_err<E: std::fmt::Display>(e: E) -> HostError {
    HostError::Validation(e.to_string())
}

/// Deserializes a JSON `null` as `Some(None)` (explicit clear) while an absent
/// field still deserializes as `None` (keep). This is the standard
/// "double option" pattern — plain `Option<Option<T>>` collapses both `null`
/// and absent to `None`, so `null` never reaches the inner option.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
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
    /// Credential id of the SSH key, required when `auth_method` is `key`.
    pub key_id: Option<String>,
    /// Credential id of the password, required when `auth_method` is `password`.
    pub password_id: Option<String>,
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
        if !matches!(self.auth_method.as_str(), "none" | "password" | "key") {
            return Err(HostError::Validation(format!(
                "invalid auth method: {} (expected 'none', 'password' or 'key')",
                self.auth_method
            )));
        }
        if self.auth_method == "key" && self.key_id.as_deref().map_or(true, str::is_empty) {
            return Err(HostError::Validation(
                "key_id is required when auth_method is 'key'".into(),
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
    pub key_id: Option<String>,
    pub password_id: Option<String>,
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
    #[serde(default, deserialize_with = "deserialize_some")]
    pub group: Option<Option<String>>,
    pub auth_method: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub key_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub password_id: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

// ── Row mapping ─────────────────────────────────────────────────────────────

fn row_to_host(row: &sqlx::sqlite::SqliteRow) -> Host {
    let tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let password_id: Option<String> = row.try_get("password_id").ok().flatten();
    let has_password = password_id.is_some();

    Host {
        id: row.try_get("id").unwrap_or_default(),
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
        key_id: row.try_get("key_id").ok().flatten(),
        password_id,
        tags,
        has_password,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[command]
pub async fn host_create(
    db: tauri::State<'_, SharedPool>,
    input: HostInput,
) -> Result<Host, String> {
    host_create_inner(db.inner(), input)
        .await
        .map_err(Into::into)
}

async fn host_create_inner(pool: &SqlitePool, input: HostInput) -> Result<Host, HostError> {
    input.validate()?;

    let id = crate::db::new_table_row_id();
    let tags_json = serde_json::to_string(&input.tags).map_err(map_host_err)?;

    sqlx::query(
        r#"INSERT INTO host ("id", "name", "host", "port", "username", "protocol", "group", "auth_method", "key_id", "password_id", "tags")
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.host)
    .bind(i64::from(input.port))
    .bind(&input.username)
    .bind(&input.protocol)
    .bind(&input.group)
    .bind(&input.auth_method)
    .bind(&input.key_id)
    .bind(&input.password_id)
    .bind(&tags_json)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    log::info!("Host '{}' created (id: {})", input.name, id);
    Ok(row_to_host(&row))
}

#[command]
pub async fn host_list(db: tauri::State<'_, SharedPool>) -> Result<Vec<Host>, String> {
    host_list_inner(db.inner()).await.map_err(Into::into)
}

async fn host_list_inner(pool: &SqlitePool) -> Result<Vec<Host>, HostError> {
    let rows = sqlx::query("SELECT * FROM host ORDER BY \"group\", \"name\"")
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_host).collect())
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
        Some(row) => Ok(row_to_host(&row)),
        None => Err(HostError::NotFound(id.to_string())),
    }
}

#[command]
pub async fn host_update(
    db: tauri::State<'_, SharedPool>,
    id: String,
    update: HostUpdate,
) -> Result<Host, String> {
    host_update_inner(db.inner(), &id, update)
        .await
        .map_err(Into::into)
}

async fn host_update_inner(
    pool: &SqlitePool,
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
    let current_key: Option<String> = row.try_get("key_id").ok().flatten();
    let current_password: Option<String> = row.try_get("password_id").ok().flatten();
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
    let new_key = update.key_id.unwrap_or(current_key);
    let new_password = update.password_id.unwrap_or(current_password);
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
    if new_auth == "key" && new_key.as_deref().map_or(true, str::is_empty) {
        return Err(HostError::Validation(
            "key_id is required when auth_method is 'key'".into(),
        ));
    }

    sqlx::query(
        r#"UPDATE host
           SET "name" = ?, "host" = ?, "port" = ?, "username" = ?, "protocol" = ?,
               "group" = ?, "auth_method" = ?, "key_id" = ?, "password_id" = ?, "tags" = ?,
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
    .bind(&new_password)
    .bind(&new_tags_json)
    .bind(id)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT * FROM host WHERE \"id\" = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    log::info!("Host '{new_name}' updated (id: {id})");
    Ok(row_to_host(&row))
}

#[command]
pub async fn host_delete(db: tauri::State<'_, SharedPool>, id: String) -> Result<(), String> {
    host_delete_inner(db.inner(), &id).await.map_err(Into::into)
}

async fn host_delete_inner(pool: &SqlitePool, id: &str) -> Result<(), HostError> {
    sqlx::query("DELETE FROM host WHERE \"id\" = ?")
        .bind(id)
        .execute(pool)
        .await?;

    log::info!("Host deleted (id: {id})");
    Ok(())
}

// ── Import / Export ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportData {
    version: u32,
    hosts: Vec<Host>,
}

/// Export all hosts as JSON (passwords excluded). Credential references are
/// exported verbatim as `key_id` / `password_id`.
#[command]
pub async fn host_export(db: tauri::State<'_, SharedPool>) -> Result<String, String> {
    let hosts = host_list_inner(db.inner())
        .await
        .map_err(|e| e.to_string())?;
    let data = ExportData { version: 1, hosts };
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

/// Host shape as read from an import file. Carries both `key_id`/`password_id`
/// (new exports) and `key_name` (legacy exports) so the importer can detect the
/// shape without a version bump.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportedHost {
    name: String,
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    username: String,
    #[serde(default = "default_protocol")]
    protocol: String,
    group: Option<String>,
    #[serde(default = "default_auth_method")]
    auth_method: String,
    key_id: Option<String>,
    password_id: Option<String>,
    key_name: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportData {
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    hosts: Vec<ImportedHost>,
}

/// Resolve a credential id that must exist in this DB, returning `None` when it
/// does not (dangling references import as `NULL`).
async fn existing_credential_id(
    pool: &SqlitePool,
    id: Option<&str>,
) -> Result<Option<String>, HostError> {
    let Some(id) = id else {
        return Ok(None);
    };
    let count: i64 = sqlx::query_scalar(r#"SELECT COUNT(*) FROM credential WHERE "id" = ?"#)
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok((count > 0).then(|| id.to_string()))
}

/// Resolve a legacy key name to a credential id (keys only). `None` when no
/// key credential with that name exists.
async fn key_id_by_name(pool: &SqlitePool, name: &str) -> Result<Option<String>, HostError> {
    let id: Option<String> =
        sqlx::query_scalar(r#"SELECT "id" FROM credential WHERE "kind" = 'key' AND "name" = ?"#)
            .bind(name)
            .fetch_optional(pool)
            .await?;
    Ok(id)
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
    json: String,
) -> Result<ImportResult, String> {
    let data: ImportData = serde_json::from_str(&json).map_err(|e| format!("invalid JSON: {e}"))?;

    let pool = db.inner();
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for host in data.hosts {
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM host WHERE \"host\" = ? AND \"port\" = ? AND \"username\" = ?",
        )
        .bind(&host.host)
        .bind(i64::from(host.port))
        .bind(&host.username)
        .fetch_one(pool)
        .await;

        match existing {
            Ok(count) if count > 0 => {
                skipped += 1;
                continue;
            }
            Err(e) => {
                failed.push(format!("{}: {e}", host.name));
                continue;
            }
            _ => {}
        }

        // Shape detection: legacy exports carry `key_name`; new exports carry
        // `key_id`/`password_id`.
        let key_id = match host.key_name.as_deref() {
            Some(key_name) => key_id_by_name(pool, key_name).await,
            None => existing_credential_id(pool, host.key_id.as_deref()).await,
        };
        let password_id = existing_credential_id(pool, host.password_id.as_deref()).await;

        let (key_id, password_id) = match (key_id, password_id) {
            (Ok(key_id), Ok(password_id)) => (key_id, password_id),
            (Err(e), _) | (_, Err(e)) => {
                failed.push(format!("{}: {e}", host.name));
                continue;
            }
        };

        let host_input = HostInput {
            name: host.name.clone(),
            host: host.host.clone(),
            port: host.port,
            username: host.username.clone(),
            protocol: host.protocol.clone(),
            group: host.group.clone(),
            auth_method: host.auth_method.clone(),
            key_id,
            password_id,
            tags: host.tags.clone(),
        };

        match host_create_inner(pool, host_input).await {
            Ok(_) => imported += 1,
            Err(e) => failed.push(format!("{}: {e}", host.name)),
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
            key_id: Some("credential-key-id".into()),
            password_id: None,
            tags: vec!["web".into(), "nginx".into()],
        }
    }

    #[tokio::test]
    async fn create_and_list_host() {
        let pool = db::test_pool().await;
        let host = host_create_inner(&pool, sample_input()).await.unwrap();
        assert_eq!(host.name, "test-host");
        assert_eq!(host.port, 22);
        assert_eq!(host.protocol, "ssh");
        assert_eq!(host.tags, vec!["web".to_string(), "nginx".to_string()]);
        assert_eq!(host.key_id.as_deref(), Some("credential-key-id"));
        assert!(!host.has_password);

        let fetched = host_resolve_inner(&pool, &host.id).await.unwrap();
        assert_eq!(fetched.id, host.id);
        assert_eq!(fetched.name, "test-host");
    }

    #[tokio::test]
    async fn delete_removes_host() {
        let pool = db::test_pool().await;
        let host = host_create_inner(&pool, sample_input()).await.unwrap();

        host_delete_inner(&pool, &host.id).await.unwrap();

        let result = host_resolve_inner(&pool, &host.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let pool = db::test_pool().await;
        let host = host_create_inner(&pool, sample_input()).await.unwrap();

        let updated = host_update_inner(
            &pool,
            &host.id,
            HostUpdate {
                name: Some("renamed".into()),
                port: Some(2222),
                host: None,
                username: None,
                protocol: None,
                group: None,
                auth_method: None,
                key_id: None,
                password_id: None,
                tags: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.port, 2222);
        assert_eq!(updated.host, "10.0.0.1");
    }

    #[tokio::test]
    async fn update_switches_auth_and_clears_other_reference() {
        let pool = db::test_pool().await;
        let host = host_create_inner(&pool, sample_input()).await.unwrap();

        let updated = host_update_inner(
            &pool,
            &host.id,
            HostUpdate {
                name: None,
                host: None,
                port: None,
                username: None,
                protocol: None,
                group: None,
                auth_method: Some("password".into()),
                key_id: Some(None),
                password_id: Some(Some("credential-pw-id".into())),
                tags: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.auth_method, "password");
        assert_eq!(updated.key_id, None);
        assert_eq!(updated.password_id.as_deref(), Some("credential-pw-id"));
        assert!(updated.has_password);
    }

    #[tokio::test]
    async fn update_none_clears_both_credential_refs() {
        let pool = db::test_pool().await;
        let mut input = sample_input();
        input.auth_method = "password".into();
        input.key_id = None;
        input.password_id = Some("credential-pw-id".into());
        let host = host_create_inner(&pool, input).await.unwrap();
        assert_eq!(host.password_id.as_deref(), Some("credential-pw-id"));

        let updated = host_update_inner(
            &pool,
            &host.id,
            HostUpdate {
                name: None,
                host: None,
                port: None,
                username: None,
                protocol: None,
                group: None,
                auth_method: Some("none".into()),
                key_id: Some(None),
                password_id: Some(None),
                tags: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.auth_method, "none");
        assert_eq!(updated.key_id, None);
        assert_eq!(updated.password_id, None);
        assert!(!updated.has_password);
    }

    #[test]
    fn host_update_deserializes_null_as_clear() {
        // Mirrors the exact JSON the frontend sends when switching a host to
        // "none" — `null` must become `Some(None)` (clear), not `None` (keep).
        let update: HostUpdate =
            serde_json::from_str(r#"{"authMethod":"none","keyId":null,"passwordId":null}"#)
                .unwrap();
        assert_eq!(update.auth_method.as_deref(), Some("none"));
        assert_eq!(update.key_id, Some(None));
        assert_eq!(update.password_id, Some(None));
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
    fn validation_rejects_key_auth_without_key_id() {
        let mut input = sample_input();
        input.auth_method = "key".into();
        input.key_id = None;
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_unsupported_protocol() {
        let mut input = sample_input();
        input.protocol = "telnet".into();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_accepts_none_auth_method() {
        let mut input = sample_input();
        input.auth_method = "none".into();
        input.key_id = None;
        input.password_id = None;
        assert!(input.validate().is_ok());
    }

    #[test]
    fn validation_accepts_valid_input() {
        assert!(sample_input().validate().is_ok());
    }

    #[test]
    fn new_id_is_unique() {
        let a = crate::db::new_table_row_id();
        let b = crate::db::new_table_row_id();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn import_shape_detection_and_dangling_id_resolution() {
        let pool = db::test_pool().await;

        let mut key = [0u8; crate::crypto::MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        let key_id = crate::credentials::create(
            &pool,
            &key,
            "laptop-ed25519",
            crate::credentials::CredentialKind::Key,
            "key-data",
            None,
            None,
            &[],
        )
        .await
        .unwrap();

        // Legacy shape: key_name resolves to the seeded credential id.
        assert_eq!(
            key_id_by_name(&pool, "laptop-ed25519")
                .await
                .unwrap()
                .as_deref(),
            Some(key_id.as_str())
        );
        assert_eq!(key_id_by_name(&pool, "missing").await.unwrap(), None);

        // Dangling new-shape id resolves to NULL.
        assert_eq!(
            existing_credential_id(&pool, Some(&key_id))
                .await
                .unwrap()
                .as_deref(),
            Some(key_id.as_str())
        );
        assert_eq!(
            existing_credential_id(&pool, Some("does-not-exist"))
                .await
                .unwrap(),
            None
        );
        assert_eq!(existing_credential_id(&pool, None).await.unwrap(), None);
    }
}
