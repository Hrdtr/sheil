use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::command;

use super::hosts::SharedPool;

#[derive(Debug, thiserror::Error)]
enum SnippetError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("snippet not found: {0}")]
    NotFound(String),
}

impl From<SnippetError> for String {
    fn from(e: SnippetError) -> Self {
        e.to_string()
    }
}

fn map_snippet_err<E: std::fmt::Display>(e: E) -> SnippetError {
    SnippetError::Validation(e.to_string())
}

/// Deserializes a JSON `null` as `Some(None)` (explicit clear) while an absent
/// field still deserializes as `None` (keep). Same double-option pattern as
/// `commands::hosts`.
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
pub struct SnippetInput {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub group: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Host scope — snippet is associated with a single host.
    pub host_id: Option<String>,
    /// Host-group scope — snippet applies to every host in the group.
    pub host_group: Option<String>,
}

impl SnippetInput {
    fn validate(&self) -> Result<(), SnippetError> {
        if self.name.trim().is_empty() {
            return Err(SnippetError::Validation("name must not be empty".into()));
        }
        if self.command.trim().is_empty() {
            return Err(SnippetError::Validation(
                "command must not be empty".into(),
            ));
        }
        if self.host_id.is_some() && self.host_group.is_some() {
            return Err(SnippetError::Validation(
                "a snippet can be scoped to either a host or a host group, not both".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_field_names)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub group: Option<String>,
    pub tags: Vec<String>,
    pub host_id: Option<String>,
    pub host_group: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::option_option)]
pub struct SnippetUpdate {
    pub name: Option<String>,
    pub command: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub group: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub host_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub host_group: Option<Option<String>>,
}

// ── Row mapping ─────────────────────────────────────────────────────────────

fn row_to_snippet(row: &sqlx::sqlite::SqliteRow) -> Snippet {
    let tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

    Snippet {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        command: row.try_get("command").unwrap_or_default(),
        description: row.try_get("description").ok().flatten(),
        group: row.try_get("group").ok().flatten(),
        tags,
        host_id: row.try_get("host_id").ok().flatten(),
        host_group: row.try_get("host_group").ok().flatten(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[command]
pub async fn snippet_create(
    db: tauri::State<'_, SharedPool>,
    input: SnippetInput,
) -> Result<Snippet, String> {
    snippet_create_inner(db.inner(), input)
        .await
        .map_err(Into::into)
}

async fn snippet_create_inner(pool: &SqlitePool, input: SnippetInput) -> Result<Snippet, SnippetError> {
    input.validate()?;

    let id = crate::db::new_table_row_id();
    let tags_json = serde_json::to_string(&input.tags).map_err(map_snippet_err)?;

    sqlx::query(
        r#"INSERT INTO snippet ("id", "name", "command", "description", "group", "tags", "host_id", "host_group")
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.command)
    .bind(&input.description)
    .bind(&input.group)
    .bind(&tags_json)
    .bind(&input.host_id)
    .bind(&input.host_group)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT * FROM snippet WHERE \"id\" = ?")
        .bind(&id)
        .fetch_one(pool)
        .await?;

    log::info!("Snippet '{}' created (id: {})", input.name, id);
    Ok(row_to_snippet(&row))
}

#[command]
pub async fn snippet_list(db: tauri::State<'_, SharedPool>) -> Result<Vec<Snippet>, String> {
    snippet_list_inner(db.inner()).await.map_err(Into::into)
}

async fn snippet_list_inner(pool: &SqlitePool) -> Result<Vec<Snippet>, SnippetError> {
    let rows = sqlx::query("SELECT * FROM snippet ORDER BY \"group\", \"name\"")
        .fetch_all(pool)
        .await?;

    Ok(rows.iter().map(row_to_snippet).collect())
}

#[command]
pub async fn snippet_resolve(
    db: tauri::State<'_, SharedPool>,
    id: String,
) -> Result<Snippet, String> {
    snippet_resolve_inner(db.inner(), &id)
        .await
        .map_err(Into::into)
}

async fn snippet_resolve_inner(pool: &SqlitePool, id: &str) -> Result<Snippet, SnippetError> {
    let row = sqlx::query("SELECT * FROM snippet WHERE \"id\" = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => Ok(row_to_snippet(&row)),
        None => Err(SnippetError::NotFound(id.to_string())),
    }
}

#[command]
pub async fn snippet_update(
    db: tauri::State<'_, SharedPool>,
    id: String,
    update: SnippetUpdate,
) -> Result<Snippet, String> {
    snippet_update_inner(db.inner(), &id, update)
        .await
        .map_err(Into::into)
}

#[allow(clippy::too_many_lines)]
async fn snippet_update_inner(
    pool: &SqlitePool,
    id: &str,
    update: SnippetUpdate,
) -> Result<Snippet, SnippetError> {
    let existing = sqlx::query("SELECT * FROM snippet WHERE \"id\" = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;

    let row = existing.ok_or_else(|| SnippetError::NotFound(id.to_string()))?;

    let current_name: String = row.try_get("name").unwrap_or_default();
    let current_command: String = row.try_get("command").unwrap_or_default();
    let current_description: Option<String> = row.try_get("description").ok().flatten();
    let current_group: Option<String> = row.try_get("group").ok().flatten();
    let current_tags_json: String = row.try_get("tags").unwrap_or_else(|_| "[]".to_string());
    let current_host_id: Option<String> = row.try_get("host_id").ok().flatten();
    let current_host_group: Option<String> = row.try_get("host_group").ok().flatten();

    let new_name = update.name.unwrap_or(current_name);
    let new_command = update.command.unwrap_or(current_command);
    let new_description = update.description.unwrap_or(current_description);
    let new_group = update.group.unwrap_or(current_group);
    let new_tags_json = if let Some(tags) = update.tags {
        serde_json::to_string(&tags).map_err(map_snippet_err)?
    } else {
        current_tags_json
    };
    // Setting one scope explicitly clears the other — a snippet is scoped to
    // at most one of host / host group.
    let host_id_explicitly_set = matches!(update.host_id, Some(Some(_)));
    let host_group_explicitly_set = matches!(update.host_group, Some(Some(_)));

    let mut new_host_id = update.host_id.unwrap_or(current_host_id);
    let mut new_host_group = update.host_group.unwrap_or(current_host_group);

    if new_name.trim().is_empty() {
        return Err(SnippetError::Validation("name must not be empty".into()));
    }
    if new_command.trim().is_empty() {
        return Err(SnippetError::Validation(
            "command must not be empty".into(),
        ));
    }

    if host_id_explicitly_set {
        new_host_group = None;
    } else if host_group_explicitly_set {
        new_host_id = None;
    }

    sqlx::query(
        r#"UPDATE snippet
           SET "name" = ?, "command" = ?, "description" = ?, "group" = ?, "tags" = ?,
               "host_id" = ?, "host_group" = ?, "updated_at" = datetime('now')
           WHERE "id" = ?"#,
    )
    .bind(&new_name)
    .bind(&new_command)
    .bind(&new_description)
    .bind(&new_group)
    .bind(&new_tags_json)
    .bind(&new_host_id)
    .bind(&new_host_group)
    .bind(id)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT * FROM snippet WHERE \"id\" = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;

    log::info!("Snippet '{new_name}' updated (id: {id})");
    Ok(row_to_snippet(&row))
}

#[command]
pub async fn snippet_delete(db: tauri::State<'_, SharedPool>, id: String) -> Result<(), String> {
    snippet_delete_inner(db.inner(), &id)
        .await
        .map_err(Into::into)
}

async fn snippet_delete_inner(pool: &SqlitePool, id: &str) -> Result<(), SnippetError> {
    sqlx::query("DELETE FROM snippet WHERE \"id\" = ?")
        .bind(id)
        .execute(pool)
        .await?;

    log::info!("Snippet deleted (id: {id})");
    Ok(())
}

/// Clear the host scope of every snippet referencing `host_id`. Called when a
/// host is deleted so snippets fall back to global instead of dangling.
pub async fn clear_host_scope(pool: &SqlitePool, host_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE snippet SET \"host_id\" = NULL WHERE \"host_id\" = ?")
        .bind(host_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn sample_input() -> SnippetInput {
        SnippetInput {
            name: "disk usage".into(),
            command: "df -h".into(),
            description: Some("Show disk usage".into()),
            group: Some("Monitoring".into()),
            tags: vec!["disk".into(), "ops".into()],
            host_id: None,
            host_group: None,
        }
    }

    async fn insert_test_host(pool: &SqlitePool, id: &str) {
        sqlx::query(
            r#"INSERT INTO host ("id", "name", "host", "port", "username")
               VALUES (?, ?, '10.0.0.1', 22, 'admin')"#,
        )
        .bind(id)
        .bind(format!("test-{id}"))
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_and_list_snippet() {
        let pool = db::test_pool().await;
        let snippet = snippet_create_inner(&pool, sample_input()).await.unwrap();
        assert_eq!(snippet.name, "disk usage");
        assert_eq!(snippet.command, "df -h");
        assert_eq!(snippet.tags, vec!["disk".to_string(), "ops".to_string()]);
        assert_eq!(snippet.host_id, None);
        assert_eq!(snippet.host_group, None);

        let fetched = snippet_resolve_inner(&pool, &snippet.id).await.unwrap();
        assert_eq!(fetched.id, snippet.id);
        assert_eq!(fetched.name, "disk usage");

        let all = snippet_list_inner(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn create_with_host_scope() {
        let pool = db::test_pool().await;
        insert_test_host(&pool, "host-1").await;
        let mut input = sample_input();
        input.host_id = Some("host-1".into());
        let snippet = snippet_create_inner(&pool, input).await.unwrap();
        assert_eq!(snippet.host_id.as_deref(), Some("host-1"));
        assert_eq!(snippet.host_group, None);
    }

    #[tokio::test]
    async fn create_rejects_unknown_host_scope() {
        let pool = db::test_pool().await;
        let mut input = sample_input();
        input.host_id = Some("missing-host".into());
        assert!(snippet_create_inner(&pool, input).await.is_err());
    }

    #[tokio::test]
    async fn delete_removes_snippet() {
        let pool = db::test_pool().await;
        let snippet = snippet_create_inner(&pool, sample_input()).await.unwrap();

        snippet_delete_inner(&pool, &snippet.id).await.unwrap();

        let result = snippet_resolve_inner(&pool, &snippet.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_changes_fields() {
        let pool = db::test_pool().await;
        let snippet = snippet_create_inner(&pool, sample_input()).await.unwrap();

        let updated = snippet_update_inner(
            &pool,
            &snippet.id,
            SnippetUpdate {
                name: Some("renamed".into()),
                command: Some("df -h /var".into()),
                description: None,
                group: None,
                tags: None,
                host_id: None,
                host_group: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.command, "df -h /var");
        assert_eq!(updated.group.as_deref(), Some("Monitoring"));
    }

    #[tokio::test]
    async fn update_null_clears_description_and_group() {
        let pool = db::test_pool().await;
        let snippet = snippet_create_inner(&pool, sample_input()).await.unwrap();

        let updated = snippet_update_inner(
            &pool,
            &snippet.id,
            SnippetUpdate {
                name: None,
                command: None,
                description: Some(None),
                group: Some(None),
                tags: None,
                host_id: None,
                host_group: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.description, None);
        assert_eq!(updated.group, None);
    }

    #[tokio::test]
    async fn update_host_scope_clears_host_group() {
        let pool = db::test_pool().await;
        insert_test_host(&pool, "host-1").await;
        let mut input = sample_input();
        input.host_group = Some("Production".into());
        let snippet = snippet_create_inner(&pool, input).await.unwrap();

        let updated = snippet_update_inner(
            &pool,
            &snippet.id,
            SnippetUpdate {
                name: None,
                command: None,
                description: None,
                group: None,
                tags: None,
                host_id: Some(Some("host-1".into())),
                host_group: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.host_id.as_deref(), Some("host-1"));
        assert_eq!(updated.host_group, None);
    }

    #[tokio::test]
    async fn clear_host_scope_nulls_matching_snippets() {
        let pool = db::test_pool().await;
        insert_test_host(&pool, "host-1").await;
        let mut input = sample_input();
        input.host_id = Some("host-1".into());
        let scoped = snippet_create_inner(&pool, input).await.unwrap();
        let global = snippet_create_inner(&pool, sample_input()).await.unwrap();

        clear_host_scope(&pool, "host-1").await.unwrap();

        let scoped = snippet_resolve_inner(&pool, &scoped.id).await.unwrap();
        assert_eq!(scoped.host_id, None);
        let global = snippet_resolve_inner(&pool, &global.id).await.unwrap();
        assert_eq!(global.host_id, None);
    }

    #[test]
    fn snippet_update_deserializes_null_as_clear() {
        let update: SnippetUpdate =
            serde_json::from_str(r#"{"description":null,"group":null,"hostId":null}"#).unwrap();
        assert_eq!(update.description, Some(None));
        assert_eq!(update.group, Some(None));
        assert_eq!(update.host_id, Some(None));
        assert_eq!(update.host_group, None);
    }

    #[test]
    fn validation_rejects_empty_name() {
        let mut input = sample_input();
        input.name = String::new();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_empty_command() {
        let mut input = sample_input();
        input.command = "   ".into();
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_rejects_both_scopes() {
        let mut input = sample_input();
        input.host_id = Some("host-1".into());
        input.host_group = Some("Production".into());
        assert!(input.validate().is_err());
    }

    #[test]
    fn validation_accepts_valid_input() {
        assert!(sample_input().validate().is_ok());
    }
}
