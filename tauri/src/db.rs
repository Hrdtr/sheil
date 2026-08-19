use sqlx::SqlitePool;
use std::path::Path;
use uuid::Uuid;

/// Initialize the `SQLite` database at `app_data_dir/sheil.db`, creating the
/// directory and running all pending migrations.
pub async fn init(app_data_dir: &Path) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(app_data_dir).ok();

    let db_path = app_data_dir.join("sheil.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

/// In-memory `SQLite` pool with all migrations applied. For tests only.
#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations");

    pool
}

/// Generate a new `UUIDv7` identifier, formatted as a hyphenated string.
///
/// `UUIDv7` embeds a Unix timestamp (ms) in the first 48 bits, giving
/// time-ordered, index-friendly primary keys without sacrificing
/// global uniqueness.
pub fn new_table_row_id() -> String {
    Uuid::now_v7().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::Migrate;

    #[test]
    fn new_table_row_id_is_unique() {
        let id1 = new_table_row_id();
        let id2 = new_table_row_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn new_table_row_id_is_uuid_v7() {
        let id = new_table_row_id();
        let uuid = Uuid::parse_str(&id).expect("valid UUID");
        assert_eq!(uuid.get_version_num(), 7);
    }

    /// Seed legacy service-keyed credentials, then run only the credential
    /// refactor migration and assert the id-based re-link preserves data.
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn credential_refactor_migration_links_legacy_data() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let migrator = sqlx::migrate!("./migrations");
        let mut conn = pool.acquire().await.unwrap();

        conn.ensure_migrations_table().await.unwrap();

        // Apply every migration except the credential refactor.
        for migration in migrator.iter() {
            if migration.description.contains("refactor") {
                break;
            }
            if !migration.migration_type.is_down_migration() {
                conn.apply(migration).await.unwrap();
            }
        }

        // Legacy host rows (old schema carries `key_name`, no id columns).
        sqlx::query(
            r#"INSERT INTO host ("id","name","host","port","username","auth_method","key_name")
               VALUES ('h1','key-host','10.0.0.1',22,'admin','key','laptop-ed25519')"#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO host ("id","name","host","port","username","auth_method")
               VALUES ('h2','pw-host','10.0.0.2',22,'admin','password')"#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        // Legacy service-keyed credentials (ciphertext copied verbatim).
        sqlx::query(
            r#"INSERT INTO credential ("service","encrypted_value","nonce")
               VALUES ('sheil.ssh_key.laptop-ed25519', ?, ?)"#,
        )
        .bind(vec![1u8, 2, 3])
        .bind(vec![4u8, 5, 6])
        .execute(&mut *conn)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO credential ("service","encrypted_value","nonce")
               VALUES ('sheil.ssh_key.laptop-ed25519.passphrase', ?, ?)"#,
        )
        .bind(vec![7u8, 8])
        .bind(vec![9u8, 10])
        .execute(&mut *conn)
        .await
        .unwrap();
        sqlx::query(
            r#"INSERT INTO credential ("service","encrypted_value","nonce")
               VALUES ('sheil.host_password.h2', ?, ?)"#,
        )
        .bind(vec![11u8, 12])
        .bind(vec![13u8, 14])
        .execute(&mut *conn)
        .await
        .unwrap();

        // Apply the refactor migration.
        for migration in migrator.iter() {
            if migration.description.contains("refactor") {
                conn.apply(migration).await.unwrap();
            }
        }

        // Exactly one key and one password credential remain (passphrase merged).
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM credential")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(count, 2);

        let (key_id, key_value, key_passphrase): (String, Vec<u8>, Option<Vec<u8>>) =
            sqlx::query_as(
                r#"SELECT "id", "encrypted_value", "key_passphrase_encrypted_value"
                   FROM credential WHERE "kind" = 'key'"#,
            )
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(key_value, vec![1u8, 2, 3]);
        assert!(key_passphrase.is_some());

        let (password_id, password_name, password_value): (String, String, Vec<u8>) =
            sqlx::query_as(
                r#"SELECT "id", "name", "encrypted_value"
                   FROM credential WHERE "kind" = 'password'"#,
            )
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        assert_eq!(password_name, "pw-host (h2)");
        assert_eq!(password_value, vec![11u8, 12]);

        for id in [&key_id, &password_id] {
            let uuid = Uuid::parse_str(id).expect("valid UUID");
            assert_eq!(uuid.get_version_num(), 7);
        }

        // Hosts are re-linked by credential id.
        let (key_host_key_id, key_host_password_id): (Option<String>, Option<String>) =
            sqlx::query_as(r#"SELECT "key_id", "password_id" FROM host WHERE "id" = 'h1'"#)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(key_host_key_id.as_deref(), Some(key_id.as_str()));
        assert_eq!(key_host_password_id, None);

        let (pw_host_key_id, pw_host_password_id): (Option<String>, Option<String>) =
            sqlx::query_as(r#"SELECT "key_id", "password_id" FROM host WHERE "id" = 'h2'"#)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(pw_host_key_id, None);
        assert_eq!(pw_host_password_id.as_deref(), Some(password_id.as_str()));
    }

    /// Seed a legacy `ai.command_palette_enabled` row, run only the rename
    /// migration, and assert the user's value survives under the new key.
    #[tokio::test]
    async fn ai_command_generator_rename_migration_preserves_value() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let migrator = sqlx::migrate!("./migrations");
        let mut conn = pool.acquire().await.unwrap();

        conn.ensure_migrations_table().await.unwrap();

        // Apply every migration except the setting rename. Note: sqlx turns
        // filename underscores into spaces in the description.
        for migration in migrator.iter() {
            if migration.description.contains("rename ai command palette") {
                continue;
            }
            if !migration.migration_type.is_down_migration() {
                conn.apply(migration).await.unwrap();
            }
        }

        // Legacy row carrying a user-customized value.
        sqlx::query(
            r#"INSERT INTO setting ("key", "value", "default_value", "value_type")
               VALUES ('ai.command_palette_enabled', 'false', 'true', 'boolean')"#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        // Apply the rename migration.
        for migration in migrator.iter() {
            if migration.description.contains("rename ai command palette") {
                conn.apply(migration).await.unwrap();
            }
        }

        let renamed: Option<(String, String, String)> = sqlx::query_as(
            r#"SELECT "key", "value", "value_type" FROM setting
               WHERE "key" = 'ai.command_generator_enabled'"#,
        )
        .fetch_optional(&mut *conn)
        .await
        .unwrap();
        assert_eq!(
            renamed,
            Some((
                "ai.command_generator_enabled".to_string(),
                "false".to_string(),
                "boolean".to_string()
            ))
        );

        let legacy: Option<String> = sqlx::query_scalar(
            r#"SELECT "key" FROM setting WHERE "key" = 'ai.command_palette_enabled'"#,
        )
        .fetch_optional(&mut *conn)
        .await
        .unwrap();
        assert!(legacy.is_none());
    }
}
