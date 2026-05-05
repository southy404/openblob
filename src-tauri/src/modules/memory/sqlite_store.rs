use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::modules::memory::events::{MemoryEvent, PrivacyTier};
use crate::modules::storage::paths::memory_database_path;

pub const CURRENT_MEMORY_SCHEMA_VERSION: i64 = 1;

pub fn open_memory_database() -> Result<Connection, String> {
    let path = memory_database_path()?;
    open_memory_database_at(&path)
}

pub fn open_memory_database_at(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not create memory database directory '{}': {e}",
                parent.display()
            )
        })?;
    }

    let conn = Connection::open(path)
        .map_err(|e| format!("Could not open memory database '{}': {e}", path.display()))?;
    run_migrations(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_memory_database_in_memory() -> Result<Connection, String> {
    let conn = Connection::open_in_memory()
        .map_err(|e| format!("Could not open in-memory memory database: {e}"))?;
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    let current_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("Could not read memory schema version: {e}"))?;

    if current_version > CURRENT_MEMORY_SCHEMA_VERSION {
        return Err(format!(
            "Memory database schema version {current_version} is newer than supported version {CURRENT_MEMORY_SCHEMA_VERSION}"
        ));
    }

    if current_version < 1 {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS memory_events (
                id TEXT PRIMARY KEY,
                version INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                kind TEXT NOT NULL,
                source TEXT NOT NULL,
                privacy_tier TEXT NOT NULL,
                app_name TEXT,
                context_domain TEXT,
                user_input TEXT,
                summary TEXT,
                outcome TEXT,
                importance REAL NOT NULL DEFAULT 0.5,
                metadata_json TEXT NOT NULL DEFAULT '{}'
            );

            CREATE INDEX IF NOT EXISTS idx_memory_events_created_at
                ON memory_events(created_at);

            CREATE INDEX IF NOT EXISTS idx_memory_events_kind
                ON memory_events(kind);

            CREATE INDEX IF NOT EXISTS idx_memory_events_context
                ON memory_events(context_domain, app_name);

            PRAGMA user_version = 1;
            "#,
        )
        .map_err(|e| format!("Could not initialize memory database schema: {e}"))?;
    }

    Ok(())
}

pub fn insert_memory_event(conn: &Connection, event: &MemoryEvent) -> Result<(), String> {
    if event.privacy_tier == PrivacyTier::Transient {
        return Ok(());
    }

    let metadata_json = serde_json::to_string(&event.metadata)
        .map_err(|e| format!("Could not serialize memory event metadata: {e}"))?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO memory_events (
            id,
            version,
            created_at,
            kind,
            source,
            privacy_tier,
            app_name,
            context_domain,
            user_input,
            summary,
            outcome,
            importance,
            metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        "#,
        params![
            event.id.as_str(),
            event.version,
            event.timestamp.as_str(),
            event.kind.as_str(),
            event.source.as_str(),
            event.privacy_tier.as_str(),
            event.app_name.as_deref(),
            event.context_domain.as_deref(),
            event.user_input.as_deref(),
            event.summary.as_deref(),
            event.outcome.as_deref(),
            event.importance,
            metadata_json,
        ],
    )
    .map_err(|e| format!("Could not insert memory event '{}': {e}", event.id))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::MemoryEventKind;
    use crate::modules::profile::companion_config::PrivacyConfig;

    #[test]
    fn migration_creates_memory_events_table() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("schema version");

        assert_eq!(version, CURRENT_MEMORY_SCHEMA_VERSION);

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'memory_events'",
                [],
                |row| row.get(0),
            )
            .expect("table count");

        assert_eq!(table_count, 1);
    }

    #[test]
    fn insert_skips_transient_events() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Transient);

        insert_memory_event(&conn, &event).expect("insert succeeds");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_events", [], |row| row.get(0))
            .expect("event count");

        assert_eq!(count, 0);
    }

    #[test]
    fn insert_persists_non_transient_event() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::successful_command(
            "Spotify",
            "desktop",
            "open spotify",
            "Opened Spotify.",
            "success",
            &PrivacyConfig::default(),
        );

        insert_memory_event(&conn, &event).expect("insert succeeds");

        let stored_kind: String = conn
            .query_row(
                "SELECT kind FROM memory_events WHERE id = ?1",
                [event.id.as_str()],
                |row| row.get(0),
            )
            .expect("stored kind");

        assert_eq!(stored_kind, "command");
    }
}
