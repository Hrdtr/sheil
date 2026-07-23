use crate::crypto::{self, MASTER_KEY_SIZE, NONCE_SIZE};
use sqlx::SqlitePool;

/// Store an encrypted credential in `SQLite`.
pub async fn store(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    service: &str,
    value: &str,
) -> Result<(), String> {
    let (ciphertext, nonce) = crypto::encrypt(master_key, value.as_bytes());

    sqlx::query(
        r#"INSERT OR REPLACE INTO credential
           ("service", "encrypted_value", "nonce", "updated_at")
           VALUES (?, ?, ?, datetime('now'))"#,
    )
    .bind(service)
    .bind(&ciphertext)
    .bind(nonce.as_slice())
    .execute(pool)
    .await
    .map_err(|e| format!("database error: {e}"))?;

    Ok(())
}

/// Retrieve and decrypt a credential from `SQLite`.
pub async fn retrieve(
    pool: &SqlitePool,
    master_key: &[u8; MASTER_KEY_SIZE],
    service: &str,
) -> Result<String, String> {
    let row: Option<(Vec<u8>, Vec<u8>)> =
        sqlx::query_as(r#"SELECT "encrypted_value", "nonce" FROM credential WHERE "service" = ?"#)
            .bind(service)
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

/// Delete a credential from `SQLite`.  No-op when the row does not exist.
pub async fn delete(pool: &SqlitePool, service: &str) -> Result<(), String> {
    sqlx::query(r#"DELETE FROM credential WHERE "service" = ?"#)
        .bind(service)
        .execute(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(())
}

/// Return `true` when an encrypted credential exists for the given service.
pub async fn exists(pool: &SqlitePool, service: &str) -> Result<bool, String> {
    let count: (i64,) = sqlx::query_as(r#"SELECT COUNT(*) FROM credential WHERE "service" = ?"#)
        .bind(service)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("database error: {e}"))?;
    Ok(count.0 > 0)
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
    async fn store_and_retrieve_roundtrip() {
        let (pool, key) = setup().await;
        store(&pool, &key, "test.svc", "s3cret").await.unwrap();
        let val = retrieve(&pool, &key, "test.svc").await.unwrap();
        assert_eq!(val, "s3cret");
    }

    #[tokio::test]
    async fn retrieve_nonexistent_fails() {
        let (pool, key) = setup().await;
        let result = retrieve(&pool, &key, "no.svc").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn exists_returns_correctly() {
        let (pool, key) = setup().await;
        assert!(!exists(&pool, "test.svc").await.unwrap());
        store(&pool, &key, "test.svc", "pw").await.unwrap();
        assert!(exists(&pool, "test.svc").await.unwrap());
    }

    #[tokio::test]
    async fn delete_removes_credential() {
        let (pool, key) = setup().await;
        store(&pool, &key, "test.svc", "pw").await.unwrap();
        delete(&pool, "test.svc").await.unwrap();
        assert!(!exists(&pool, "test.svc").await.unwrap());
    }

    #[tokio::test]
    async fn overwrite_updates_value() {
        let (pool, key) = setup().await;
        store(&pool, &key, "test.svc", "old").await.unwrap();
        store(&pool, &key, "test.svc", "new").await.unwrap();
        let val = retrieve(&pool, &key, "test.svc").await.unwrap();
        assert_eq!(val, "new");
    }
}
