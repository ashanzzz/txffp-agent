use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(database_url: &str) -> Result<DbPool, sqlx::Error> {
    // If SQLite file url, ensure parent directory exists
    if database_url.starts_with("sqlite:") {
        let path_str = database_url.trim_start_matches("sqlite://");
        let path = std::path::Path::new(path_str);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
        }
    }

    let connection_options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    // Create required tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kv (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS operations (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            phase TEXT NOT NULL,
            parameters TEXT NOT NULL,
            result TEXT,
            error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS human_actions (
            id TEXT PRIMARY KEY,
            token TEXT UNIQUE NOT NULL,
            type TEXT NOT NULL,
            state TEXT NOT NULL,
            message TEXT NOT NULL,
            operation_id TEXT,
            context_json TEXT,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS downloads (
            id TEXT PRIMARY KEY,
            invoice_id TEXT NOT NULL,
            file_type TEXT NOT NULL,
            file_path TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            sha256 TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}
