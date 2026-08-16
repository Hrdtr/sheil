use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::command;

use crate::commands::hosts::SharedPool;
use crate::commands::ssh::{key_fingerprint, parse_private_key, try_parse_key_info};
use crate::credentials::{self, CredentialKind, CredentialRow};
use crate::crypto::{self, MASTER_KEY_SIZE, NONCE_SIZE};
use crate::MasterKey;

/// Deserializes a JSON `null` as `Some(None)` (explicit clear) while an absent
/// field still deserializes as `None` (keep). Mirrors `hosts::deserialize_some`.
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
pub struct CredentialInput {
    pub name: String,
    pub kind: CredentialKind,
    /// Key data or password plaintext.
    pub value: String,
    /// Key passphrase (keys only).
    pub key_passphrase_value: Option<String>,
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub struct CredentialUpdate {
    pub name: Option<String>,
    pub value: Option<String>,
    pub key_passphrase_value: Option<String>,
    /// `null` clears the group.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub group: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialInfo {
    pub id: String,
    pub name: String,
    pub kind: CredentialKind,
    /// Key type (e.g. `ssh-ed25519`); `None` for passwords.
    pub key_type: Option<String>,
    /// Key fingerprint; `None` for passwords.
    pub key_fingerprint: Option<String>,
    pub group: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn decrypt_row_value(
    row: &CredentialRow,
    master_key: &[u8; MASTER_KEY_SIZE],
) -> Result<String, String> {
    let nonce: [u8; NONCE_SIZE] = row
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| "stored nonce has wrong length".to_string())?;
    let plaintext = crypto::decrypt(master_key, &row.encrypted_value, &nonce)?;
    String::from_utf8(plaintext).map_err(|e| format!("invalid UTF-8 in credential: {e}"))
}

fn row_to_info(row: &CredentialRow, master_key: &[u8; MASTER_KEY_SIZE]) -> CredentialInfo {
    let (key_type, key_fingerprint) = if row.kind == CredentialKind::Key {
        decrypt_row_value(row, master_key)
            .ok()
            .and_then(|key_data| try_parse_key_info(&key_data))
            .map_or((None, None), |(t, f)| (Some(t), Some(f)))
    } else {
        (None, None)
    };

    CredentialInfo {
        id: row.id.clone(),
        name: row.name.clone(),
        kind: row.kind,
        key_type,
        key_fingerprint,
        group: row.group.clone(),
        tags: row.tags.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[command]
pub async fn credential_create(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    input: CredentialInput,
) -> Result<CredentialInfo, String> {
    credential_create_inner(db.inner(), &master_key.0, input).await
}

async fn credential_create_inner(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    input: CredentialInput,
) -> Result<CredentialInfo, String> {
    if input.name.trim().is_empty() {
        return Err("name must not be empty".to_string());
    }

    let key_info = match input.kind {
        CredentialKind::Key => {
            let key = parse_private_key(&input.value, input.key_passphrase_value.as_deref())
                .map_err(|e| e.to_string())?;
            Some((key.algorithm().to_string(), key_fingerprint(&key)))
        }
        CredentialKind::Password => {
            if input.value.is_empty() {
                return Err("password must not be empty".to_string());
            }
            None
        }
    };

    let id = credentials::create(
        pool,
        master_key,
        input.name.trim(),
        input.kind,
        &input.value,
        input.key_passphrase_value.as_deref(),
        input.group.as_deref(),
        &input.tags,
    )
    .await?;

    let row = credentials::get_row(pool, &id)
        .await?
        .ok_or_else(|| "credential not found after create".to_string())?;
    let mut info = row_to_info(&row, master_key);
    if let Some((key_type, fingerprint)) = key_info {
        info.key_type = Some(key_type);
        info.key_fingerprint = Some(fingerprint);
    }
    Ok(info)
}

#[command]
pub async fn credential_list(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    kind: Option<CredentialKind>,
) -> Result<Vec<CredentialInfo>, String> {
    let rows = credentials::list(db.inner(), kind).await?;
    Ok(rows.iter().map(|r| row_to_info(r, &master_key.0)).collect())
}

#[command]
pub async fn credential_update(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    id: String,
    update: CredentialUpdate,
) -> Result<CredentialInfo, String> {
    credential_update_inner(db.inner(), &master_key.0, &id, update).await
}

async fn credential_update_inner(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    id: &str,
    update: CredentialUpdate,
) -> Result<CredentialInfo, String> {
    if let Some(name) = &update.name {
        if name.trim().is_empty() {
            return Err("name must not be empty".to_string());
        }
    }

    let existing = credentials::get_row(pool, id)
        .await?
        .ok_or_else(|| "credential not found".to_string())?;

    // Re-validate keys when the key material or passphrase changes.
    let key_info = if existing.kind == CredentialKind::Key
        && (update.value.is_some() || update.key_passphrase_value.is_some())
    {
        let current_value = credentials::retrieve_value(pool, master_key, id).await?;
        let current_passphrase =
            credentials::retrieve_key_passphrase_value(pool, master_key, id).await?;
        let new_value = update.value.clone().unwrap_or(current_value);
        let new_passphrase = match &update.key_passphrase_value {
            Some(p) if !p.is_empty() => Some(p.clone()),
            Some(_) => None,
            // Replacing the key material with a blank passphrase means the new
            // key has no passphrase — don't validate against the old one.
            None if update.value.is_some() => None,
            None => current_passphrase,
        };
        let key =
            parse_private_key(&new_value, new_passphrase.as_deref()).map_err(|e| e.to_string())?;
        Some((key.algorithm().to_string(), key_fingerprint(&key)))
    } else {
        None
    };

    credentials::update(
        pool,
        master_key,
        id,
        update.name.as_deref(),
        update.value.as_deref(),
        update.key_passphrase_value.as_deref(),
        update.group,
        update.tags,
    )
    .await?;

    let row = credentials::get_row(pool, id)
        .await?
        .ok_or_else(|| "credential not found after update".to_string())?;
    let mut info = row_to_info(&row, master_key);
    if let Some((key_type, fingerprint)) = key_info {
        info.key_type = Some(key_type);
        info.key_fingerprint = Some(fingerprint);
    }
    Ok(info)
}

#[command]
pub async fn credential_delete(db: tauri::State<'_, SharedPool>, id: String) -> Result<(), String> {
    credential_delete_inner(db.inner(), &id).await
}

async fn credential_delete_inner(pool: &SqlitePool, id: &str) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("database error: {e}"))?;

    sqlx::query(r#"DELETE FROM credential WHERE "id" = ?"#)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("database error: {e}"))?;

    // Detach the deleted credential from any host using it — the host's auth
    // method resets to 'none' since its credential no longer exists.
    sqlx::query(r#"UPDATE host SET "key_id" = NULL, "auth_method" = 'none' WHERE "key_id" = ?"#)
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("database error: {e}"))?;

    sqlx::query(
        r#"UPDATE host SET "password_id" = NULL, "auth_method" = 'none' WHERE "password_id" = ?"#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    tx.commit()
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

/// Decrypts and returns the credential value (used for password auth; keys are
/// resolved server-side in `ssh_connect`).
#[command]
pub async fn credential_resolve(
    db: tauri::State<'_, SharedPool>,
    master_key: tauri::State<'_, MasterKey>,
    id: String,
) -> Result<String, String> {
    credentials::retrieve_value(db.inner(), &master_key.0, &id).await
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rand::Rng;

    fn ed25519_test_key() -> &'static str {
        include_str!("../../tests/fixtures/test_ed25519_key")
    }

    fn test_master_key() -> [u8; MASTER_KEY_SIZE] {
        let mut key = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        key
    }

    #[tokio::test]
    async fn create_and_resolve_password() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let info = credential_create_inner(
            &pool,
            &key,
            CredentialInput {
                name: "db-password".into(),
                kind: CredentialKind::Password,
                value: "hunter2".into(),
                key_passphrase_value: None,
                group: Some("Production".into()),
                tags: vec!["db".into()],
            },
        )
        .await
        .unwrap();

        assert_eq!(info.name, "db-password");
        assert_eq!(info.kind, CredentialKind::Password);
        assert!(info.key_type.is_none());
        assert!(info.key_fingerprint.is_none());

        let resolved = credentials::retrieve_value(&pool, &key, &info.id)
            .await
            .unwrap();
        assert_eq!(resolved, "hunter2");
    }

    #[tokio::test]
    async fn create_rejects_invalid_key() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let result = credential_create_inner(
            &pool,
            &key,
            CredentialInput {
                name: "bad-key".into(),
                kind: CredentialKind::Key,
                value: "not a key".into(),
                key_passphrase_value: None,
                group: None,
                tags: vec![],
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_rejects_empty_password() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let result = credential_create_inner(
            &pool,
            &key,
            CredentialInput {
                name: "empty".into(),
                kind: CredentialKind::Password,
                value: String::new(),
                key_passphrase_value: None,
                group: None,
                tags: vec![],
            },
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_derives_key_type_and_fingerprint() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let created = credential_create_inner(
            &pool,
            &key,
            CredentialInput {
                name: "laptop-ed25519".into(),
                kind: CredentialKind::Key,
                value: ed25519_test_key().into(),
                key_passphrase_value: None,
                group: None,
                tags: vec![],
            },
        )
        .await
        .unwrap();

        assert_eq!(created.key_type.as_deref(), Some("ssh-ed25519"));
        assert!(created
            .key_fingerprint
            .as_deref()
            .unwrap()
            .starts_with("SHA256:"));

        let rows = credentials::list(&pool, Some(CredentialKind::Key))
            .await
            .unwrap();
        let info = row_to_info(&rows[0], &key);
        assert_eq!(info.key_type.as_deref(), Some("ssh-ed25519"));
        assert!(info
            .key_fingerprint
            .as_deref()
            .unwrap()
            .starts_with("SHA256:"));
    }

    #[tokio::test]
    async fn delete_clears_host_references() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let key_id = credentials::create(
            &pool,
            &key,
            "laptop-ed25519",
            CredentialKind::Key,
            ed25519_test_key(),
            None,
            None,
            &[],
        )
        .await
        .unwrap();

        let host_id = crate::db::new_table_row_id();
        sqlx::query(
            r#"INSERT INTO host ("id","name","host","port","username","auth_method","key_id")
               VALUES (?, ?, ?, ?, ?, 'key', ?)"#,
        )
        .bind(&host_id)
        .bind("test-host")
        .bind("10.0.0.1")
        .bind(22i64)
        .bind("admin")
        .bind(&key_id)
        .execute(&pool)
        .await
        .unwrap();

        credential_delete_inner(&pool, &key_id).await.unwrap();

        let (auth, remaining_key): (String, Option<String>) =
            sqlx::query_as(r#"SELECT "auth_method", "key_id" FROM host WHERE "id" = ?"#)
                .bind(&host_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(auth, "none");
        assert_eq!(remaining_key, None);
    }

    #[tokio::test]
    async fn delete_password_clears_host_auth_method() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let password_id = credentials::create(
            &pool,
            &key,
            "db-password",
            CredentialKind::Password,
            "hunter2",
            None,
            None,
            &[],
        )
        .await
        .unwrap();

        let host_id = crate::db::new_table_row_id();
        sqlx::query(
            r#"INSERT INTO host ("id","name","host","port","username","auth_method","password_id")
               VALUES (?, ?, ?, ?, ?, 'password', ?)"#,
        )
        .bind(&host_id)
        .bind("pw-host")
        .bind("10.0.0.2")
        .bind(22i64)
        .bind("admin")
        .bind(&password_id)
        .execute(&pool)
        .await
        .unwrap();

        credential_delete_inner(&pool, &password_id).await.unwrap();

        let (auth, remaining_password): (String, Option<String>) =
            sqlx::query_as(r#"SELECT "auth_method", "password_id" FROM host WHERE "id" = ?"#)
                .bind(&host_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(auth, "none");
        assert_eq!(remaining_password, None);
    }

    #[tokio::test]
    async fn update_renames_and_revalidates_key() {
        let pool = db::test_pool().await;
        let key = test_master_key();

        let created = credential_create_inner(
            &pool,
            &key,
            CredentialInput {
                name: "laptop-ed25519".into(),
                kind: CredentialKind::Key,
                value: ed25519_test_key().into(),
                key_passphrase_value: None,
                group: None,
                tags: vec![],
            },
        )
        .await
        .unwrap();

        let updated = credential_update_inner(
            &pool,
            &key,
            &created.id,
            CredentialUpdate {
                name: Some("renamed-key".into()),
                value: None,
                key_passphrase_value: None,
                group: Some(Some("Keys".into())),
                tags: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "renamed-key");
        assert_eq!(updated.group.as_deref(), Some("Keys"));

        // Invalid key material must be rejected on update.
        let bad = credential_update_inner(
            &pool,
            &key,
            &created.id,
            CredentialUpdate {
                name: None,
                value: Some("not a key".into()),
                key_passphrase_value: None,
                group: None,
                tags: None,
            },
        )
        .await;
        assert!(bad.is_err());
    }
}
