use crate::crypto::{self, MASTER_KEY_SIZE, NONCE_SIZE};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

/// The two credential kinds backed by the `credential` table.
///
/// Serialized as lowercase `"key"` / `"password"` for the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CredentialKind {
    Key,
    Password,
}

impl CredentialKind {
    /// The `kind` column value for this variant.
    pub fn as_str(self) -> &'static str {
        match self {
            CredentialKind::Key => "key",
            CredentialKind::Password => "password",
        }
    }
}

/// A raw `credential` row. `encrypted_value`/`nonce` stay encrypted here; the
/// command layer decrypts on demand. The optional passphrase columns are read
/// via `retrieve_key_passphrase_value`, so they are not mirrored here.
#[derive(Debug, Clone)]
pub struct CredentialRow {
    pub id: String,
    pub name: String,
    pub kind: CredentialKind,
    pub encrypted_value: Vec<u8>,
    pub nonce: Vec<u8>,
    pub group: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn parse_kind(value: &str) -> CredentialKind {
    match value {
        "key" => CredentialKind::Key,
        _ => CredentialKind::Password,
    }
}

fn parse_tags(value: Option<&str>) -> Vec<String> {
    value
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

fn row_to_credential(row: &sqlx::sqlite::SqliteRow) -> CredentialRow {
    let kind: String = row.try_get("kind").unwrap_or_default();
    let tags_json: Option<String> = row.try_get("tags").ok();
    CredentialRow {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        kind: parse_kind(&kind),
        encrypted_value: row.try_get("encrypted_value").unwrap_or_default(),
        nonce: row.try_get("nonce").unwrap_or_default(),
        group: row.try_get("group").ok().flatten(),
        tags: parse_tags(tags_json.as_deref()),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

/// Create a new credential, returning its generated `UUIDv7` id.
///
/// `value` is always encrypted. `key_passphrase_value` is encrypted only when
/// `Some` (and may be empty, which is stored as `NULL`).
#[allow(clippy::too_many_arguments)]
pub async fn create(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    name: &str,
    kind: CredentialKind,
    value: &str,
    key_passphrase_value: Option<&str>,
    group: Option<&str>,
    tags: &[String],
) -> Result<String, String> {
    let id = crate::db::new_table_row_id();
    let (ciphertext, nonce) = crypto::encrypt(master_key, value.as_bytes());

    let (passphrase_ciphertext, passphrase_nonce) =
        match key_passphrase_value.filter(|p| !p.is_empty()) {
            Some(passphrase) => {
                let (ct, nonce) = crypto::encrypt(master_key, passphrase.as_bytes());
                (Some(ct), Some(nonce.to_vec()))
            }
            None => (None, None),
        };

    let tags_json = serde_json::to_string(tags).map_err(|e| e.to_string())?;

    sqlx::query(
        r#"INSERT INTO credential
           ("id", "name", "kind", "encrypted_value", "nonce",
            "key_passphrase_encrypted_value", "key_passphrase_nonce",
            "group", "tags")
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(name)
    .bind(kind.as_str())
    .bind(&ciphertext)
    .bind(nonce.as_slice())
    .bind(passphrase_ciphertext.as_deref())
    .bind(passphrase_nonce.as_deref())
    .bind(group)
    .bind(&tags_json)
    .execute(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(id)
}

/// Update an existing credential by id.
///
/// `None` fields are left unchanged; `Some` fields are applied (re-encrypting
/// `value` / `key_passphrase_value` when present). `group`/`tags` use the same
/// `Option<Option>` / `Option<Vec>` semantics as the host update command.
#[allow(clippy::too_many_arguments, clippy::option_option)]
pub async fn update(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    id: &str,
    name: Option<&str>,
    value: Option<&str>,
    key_passphrase_value: Option<&str>,
    group: Option<Option<String>>,
    tags: Option<Vec<String>>,
) -> Result<(), String> {
    let existing = sqlx::query("SELECT * FROM credential WHERE \"id\" = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?
        .ok_or_else(|| "credential not found".to_string())?;

    let current_name: String = existing
        .try_get("name")
        .map_err(|e| format!("database error: {e}"))?;
    let current_encrypted: Vec<u8> = existing
        .try_get("encrypted_value")
        .map_err(|e| format!("database error: {e}"))?;
    let current_nonce: Vec<u8> = existing
        .try_get("nonce")
        .map_err(|e| format!("database error: {e}"))?;
    let current_passphrase_encrypted: Option<Vec<u8>> = existing
        .try_get("key_passphrase_encrypted_value")
        .ok()
        .flatten();
    let current_passphrase_nonce: Option<Vec<u8>> =
        existing.try_get("key_passphrase_nonce").ok().flatten();
    let current_group: Option<String> = existing.try_get("group").ok().flatten();
    let current_tags_json: String = existing
        .try_get("tags")
        .unwrap_or_else(|_| "[]".to_string());

    let new_name = name.unwrap_or(&current_name);

    let (new_encrypted, new_nonce) = match value {
        Some(v) => {
            let (ct, nonce) = crypto::encrypt(master_key, v.as_bytes());
            (ct, nonce.to_vec())
        }
        None => (current_encrypted.clone(), current_nonce.clone()),
    };

    let (new_passphrase_encrypted, new_passphrase_nonce) = match key_passphrase_value {
        Some(passphrase) if !passphrase.is_empty() => {
            let (ct, nonce) = crypto::encrypt(master_key, passphrase.as_bytes());
            (Some(ct), Some(nonce.to_vec()))
        }
        Some(_) => (None, None),
        // When the key material itself is replaced, a blank passphrase means the
        // new key has no passphrase — do not carry the old one over.
        None if value.is_some() => (None, None),
        None => (
            current_passphrase_encrypted.clone(),
            current_passphrase_nonce.clone(),
        ),
    };

    let new_group = group.unwrap_or(current_group);
    let new_tags_json = if let Some(tags) = tags {
        serde_json::to_string(&tags).map_err(|e| e.to_string())?
    } else {
        current_tags_json
    };

    sqlx::query(
        r#"UPDATE credential
           SET "name" = ?, "encrypted_value" = ?, "nonce" = ?,
               "key_passphrase_encrypted_value" = ?, "key_passphrase_nonce" = ?,
               "group" = ?, "tags" = ?, "updated_at" = datetime('now')
           WHERE "id" = ?"#,
    )
    .bind(new_name)
    .bind(&new_encrypted)
    .bind(new_nonce.as_slice())
    .bind(new_passphrase_encrypted.as_deref())
    .bind(new_passphrase_nonce.as_deref())
    .bind(&new_group)
    .bind(&new_tags_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(())
}

/// Decrypt and return the credential value (key material or password).
pub async fn retrieve_value(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    id: &str,
) -> Result<String, String> {
    let row: Option<(Vec<u8>, Vec<u8>)> =
        sqlx::query_as(r#"SELECT "encrypted_value", "nonce" FROM credential WHERE "id" = ?"#)
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("database error: {e}"))?;

    let (ciphertext, nonce_bytes) = row.ok_or_else(|| "credential not found".to_string())?;

    let nonce: [u8; NONCE_SIZE] = nonce_bytes
        .try_into()
        .map_err(|_| "stored nonce has wrong length".to_string())?;

    let plaintext = crypto::decrypt(master_key, &ciphertext, &nonce)?;
    String::from_utf8(plaintext).map_err(|e| format!("invalid UTF-8 in credential: {e}"))
}

/// Decrypt and return the optional key passphrase for a credential.
/// Returns `Ok(None)` when the credential has no passphrase stored.
pub async fn retrieve_key_passphrase_value(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    id: &str,
) -> Result<Option<String>, String> {
    type PassphraseColumns = (Option<Vec<u8>>, Option<Vec<u8>>);

    let row: Option<PassphraseColumns> = sqlx::query_as(
        r#"SELECT "key_passphrase_encrypted_value", "key_passphrase_nonce"
           FROM credential WHERE "id" = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    let Some((Some(ciphertext), Some(nonce_bytes))) = row else {
        return Ok(None);
    };

    let nonce: [u8; NONCE_SIZE] = nonce_bytes
        .try_into()
        .map_err(|_| "stored nonce has wrong length".to_string())?;

    let plaintext = crypto::decrypt(master_key, &ciphertext, &nonce)?;
    String::from_utf8(plaintext)
        .map(Some)
        .map_err(|e| format!("invalid UTF-8 in credential: {e}"))
}

/// Fetch a single credential row by id, if it exists.
pub async fn get_row(pool: &SqlitePool, id: &str) -> Result<Option<CredentialRow>, String> {
    let row = sqlx::query(
        r#"SELECT "id", "name", "kind", "encrypted_value", "nonce",
                  "group", "tags", "created_at", "updated_at"
           FROM credential WHERE "id" = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(row.as_ref().map(row_to_credential))
}

/// List credentials, optionally filtered by `kind`.
pub async fn list(
    pool: &SqlitePool,
    kind: Option<CredentialKind>,
) -> Result<Vec<CredentialRow>, String> {
    let rows = match kind {
        Some(kind) => sqlx::query(
            r#"SELECT "id", "name", "kind", "encrypted_value", "nonce",
                          "group", "tags", "created_at", "updated_at"
                   FROM credential WHERE "kind" = ? ORDER BY "group", "name""#,
        )
        .bind(kind.as_str())
        .fetch_all(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?,
        None => sqlx::query(
            r#"SELECT "id", "name", "kind", "encrypted_value", "nonce",
                          "group", "tags", "created_at", "updated_at"
                   FROM credential ORDER BY "group", "name""#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?,
    };

    Ok(rows.iter().map(row_to_credential).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rand::Rng;

    async fn setup() -> (SqlitePool, [u8; MASTER_KEY_SIZE]) {
        let pool = db::test_pool().await;
        let mut key = [0u8; MASTER_KEY_SIZE];
        rand::rng().fill_bytes(&mut key);
        (pool, key)
    }

    #[tokio::test]
    async fn create_and_retrieve_password_roundtrip() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "db-password",
            CredentialKind::Password,
            "hunter2",
            None,
            Some("Production"),
            &["db".into()],
        )
        .await
        .unwrap();
        assert!(!id.is_empty());

        let value = retrieve_value(&pool, &key, &id).await.unwrap();
        assert_eq!(value, "hunter2");

        let passphrase = retrieve_key_passphrase_value(&pool, &key, &id)
            .await
            .unwrap();
        assert_eq!(passphrase, None);
    }

    #[tokio::test]
    async fn create_and_retrieve_key_with_passphrase() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "laptop-ed25519",
            CredentialKind::Key,
            "key-data",
            Some("passphrase"),
            None,
            &[],
        )
        .await
        .unwrap();

        let value = retrieve_value(&pool, &key, &id).await.unwrap();
        assert_eq!(value, "key-data");

        let passphrase = retrieve_key_passphrase_value(&pool, &key, &id)
            .await
            .unwrap();
        assert_eq!(passphrase.as_deref(), Some("passphrase"));
    }

    #[tokio::test]
    async fn list_filters_by_kind() {
        let (pool, key) = setup().await;
        create(
            &pool,
            &key,
            "a-key",
            CredentialKind::Key,
            "key-data",
            None,
            None,
            &[],
        )
        .await
        .unwrap();
        create(
            &pool,
            &key,
            "a-password",
            CredentialKind::Password,
            "pw",
            None,
            None,
            &[],
        )
        .await
        .unwrap();

        let all = list(&pool, None).await.unwrap();
        assert_eq!(all.len(), 2);

        let keys = list(&pool, Some(CredentialKind::Key)).await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].kind, CredentialKind::Key);

        let passwords = list(&pool, Some(CredentialKind::Password)).await.unwrap();
        assert_eq!(passwords.len(), 1);
        assert_eq!(passwords[0].kind, CredentialKind::Password);
    }

    #[tokio::test]
    async fn update_changes_fields_and_reencrypts() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "old-name",
            CredentialKind::Password,
            "old",
            None,
            None,
            &[],
        )
        .await
        .unwrap();

        update(
            &pool,
            &key,
            &id,
            Some("new-name"),
            Some("new-pw"),
            None,
            Some(Some("group".into())),
            Some(vec!["tag".into()]),
        )
        .await
        .unwrap();

        let value = retrieve_value(&pool, &key, &id).await.unwrap();
        assert_eq!(value, "new-pw");

        let rows = list(&pool, Some(CredentialKind::Password)).await.unwrap();
        let row = rows.iter().find(|r| r.id == id).unwrap();
        assert_eq!(row.name, "new-name");
        assert_eq!(row.group.as_deref(), Some("group"));
        assert_eq!(row.tags, vec!["tag".to_string()]);
    }

    #[tokio::test]
    async fn update_clears_passphrase_on_empty() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "key",
            CredentialKind::Key,
            "key-data",
            Some("pw"),
            None,
            &[],
        )
        .await
        .unwrap();

        update(&pool, &key, &id, None, None, Some(""), None, None)
            .await
            .unwrap();

        let passphrase = retrieve_key_passphrase_value(&pool, &key, &id)
            .await
            .unwrap();
        assert_eq!(passphrase, None);
    }

    #[tokio::test]
    async fn update_replacing_value_drops_old_passphrase() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "key",
            CredentialKind::Key,
            "old-key-data",
            Some("pw"),
            None,
            &[],
        )
        .await
        .unwrap();

        // Replace the key material while leaving the passphrase blank (None).
        update(
            &pool,
            &key,
            &id,
            None,
            Some("new-key-data"),
            None,
            None,
            None,
        )
        .await
        .unwrap();

        let value = retrieve_value(&pool, &key, &id).await.unwrap();
        assert_eq!(value, "new-key-data");

        let passphrase = retrieve_key_passphrase_value(&pool, &key, &id)
            .await
            .unwrap();
        assert_eq!(passphrase, None);
    }

    #[tokio::test]
    async fn update_keeping_value_preserves_passphrase() {
        let (pool, key) = setup().await;
        let id = create(
            &pool,
            &key,
            "key",
            CredentialKind::Key,
            "key-data",
            Some("pw"),
            None,
            &[],
        )
        .await
        .unwrap();

        // Leave both value and passphrase unchanged (None) — passphrase kept.
        update(&pool, &key, &id, Some("renamed"), None, None, None, None)
            .await
            .unwrap();

        let passphrase = retrieve_key_passphrase_value(&pool, &key, &id)
            .await
            .unwrap();
        assert_eq!(passphrase.as_deref(), Some("pw"));
    }

    #[tokio::test]
    async fn retrieve_nonexistent_fails() {
        let (pool, key) = setup().await;
        let result = retrieve_value(&pool, &key, "missing").await;
        assert!(result.is_err());
    }
}
