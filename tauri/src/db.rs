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
}
