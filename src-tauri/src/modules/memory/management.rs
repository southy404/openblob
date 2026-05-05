use chrono::{Datelike, TimeZone, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::modules::memory::sqlite_store::open_memory_database;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryMutationReport {
    pub events: usize,
    pub facts: usize,
    pub summaries: usize,
    pub embeddings: usize,
}

pub fn export_memory_json() -> Result<String, String> {
    let conn = open_memory_database()?;
    let export = json!({
        "version": 1,
        "exported_at": Utc::now().to_rfc3339(),
        "events": export_rows(&conn, "memory_events")?,
        "facts": export_rows(&conn, "memory_facts")?,
        "summaries": export_rows(&conn, "memory_summaries")?,
        "embeddings": export_rows(&conn, "memory_embeddings")?,
    });

    serde_json::to_string_pretty(&export)
        .map_err(|e| format!("Could not serialize memory export: {e}"))
}

pub fn wipe_memory() -> Result<MemoryMutationReport, String> {
    let conn = open_memory_database()?;
    wipe_memory_with_connection(&conn)
}

pub fn forget_memory(query: &str) -> Result<MemoryMutationReport, String> {
    let conn = open_memory_database()?;
    forget_memory_with_connection(&conn, query)
}

pub fn wipe_memory_with_connection(conn: &Connection) -> Result<MemoryMutationReport, String> {
    let embeddings = delete_all(conn, "memory_embeddings")?;
    let _ = delete_all(conn, "memory_events_fts")?;
    let summaries = delete_all(conn, "memory_summaries")?;
    let facts = delete_all(conn, "memory_facts")?;
    let events = delete_all(conn, "memory_events")?;

    Ok(MemoryMutationReport {
        events,
        facts,
        summaries,
        embeddings,
    })
}

pub fn forget_memory_with_connection(
    conn: &Connection,
    query: &str,
) -> Result<MemoryMutationReport, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(MemoryMutationReport::default());
    }

    if asks_for_today(query) {
        return forget_today(conn);
    }

    let pattern = format!("%{}%", query.to_lowercase());
    let event_ids = matching_event_ids(conn, &pattern)?;
    let event_count = delete_events_by_ids(conn, &event_ids)?;

    let facts = conn
        .execute(
            r#"
            DELETE FROM memory_facts
            WHERE lower(subject) LIKE ?1
               OR lower(predicate) LIKE ?1
               OR lower(object) LIKE ?1
               OR lower(source) LIKE ?1
            "#,
            [&pattern],
        )
        .map_err(|e| format!("Could not forget matching memory facts: {e}"))?;

    let summaries = conn
        .execute(
            "DELETE FROM memory_summaries WHERE lower(summary) LIKE ?1 OR lower(scope) LIKE ?1",
            [&pattern],
        )
        .map_err(|e| format!("Could not forget matching memory summaries: {e}"))?;

    let embeddings = delete_orphan_embeddings(conn)?;

    Ok(MemoryMutationReport {
        events: event_count,
        facts,
        summaries,
        embeddings,
    })
}

fn forget_today(conn: &Connection) -> Result<MemoryMutationReport, String> {
    let now = Utc::now();
    let start = Utc
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| "Could not calculate today's memory boundary".to_string())?
        .to_rfc3339();

    let ids = event_ids_since(conn, &start)?;
    let events = delete_events_by_ids(conn, &ids)?;
    let summaries = conn
        .execute("DELETE FROM memory_summaries WHERE created_at >= ?1", [&start])
        .map_err(|e| format!("Could not forget today's memory summaries: {e}"))?;
    let embeddings = delete_orphan_embeddings(conn)?;

    Ok(MemoryMutationReport {
        events,
        facts: 0,
        summaries,
        embeddings,
    })
}

fn export_rows(conn: &Connection, table: &str) -> Result<Vec<Value>, String> {
    let sql = match table {
        "memory_events" => {
            "SELECT json_object('id', id, 'version', version, 'created_at', created_at, 'kind', kind, 'source', source, 'privacy_tier', privacy_tier, 'app_name', app_name, 'context_domain', context_domain, 'user_input', user_input, 'summary', summary, 'outcome', outcome, 'importance', importance, 'metadata', metadata_json) FROM memory_events ORDER BY created_at"
        }
        "memory_facts" => {
            "SELECT json_object('id', id, 'source_key', source_key, 'subject', subject, 'predicate', predicate, 'object', object, 'confidence', confidence, 'valid_from', valid_from, 'valid_to', valid_to, 'superseded_by', superseded_by, 'source', source, 'metadata', metadata_json) FROM memory_facts ORDER BY valid_from"
        }
        "memory_summaries" => {
            "SELECT json_object('id', id, 'scope', scope, 'period_start', period_start, 'period_end', period_end, 'summary', summary, 'source', source, 'created_at', created_at, 'metadata', metadata_json) FROM memory_summaries ORDER BY period_end"
        }
        "memory_embeddings" => {
            "SELECT json_object('target_id', target_id, 'target_kind', target_kind, 'model', model, 'dimensions', dimensions, 'created_at', created_at) FROM memory_embeddings ORDER BY created_at"
        }
        _ => return Err(format!("Unsupported memory export table: {table}")),
    };

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Could not prepare memory export for {table}: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Could not read memory export rows for {table}: {e}"))?;

    let mut values = Vec::new();
    for row in rows {
        let raw = row.map_err(|e| format!("Could not decode memory export row: {e}"))?;
        values.push(serde_json::from_str(&raw).unwrap_or_else(|_| json!({ "raw": raw })));
    }

    Ok(values)
}

fn asks_for_today(query: &str) -> bool {
    let query = query.to_lowercase();
    query.contains("today") || query.contains("heute")
}

fn matching_event_ids(conn: &Connection, pattern: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id
            FROM memory_events
            WHERE lower(kind) LIKE ?1
               OR lower(source) LIKE ?1
               OR lower(coalesce(app_name, '')) LIKE ?1
               OR lower(coalesce(context_domain, '')) LIKE ?1
               OR lower(coalesce(user_input, '')) LIKE ?1
               OR lower(coalesce(summary, '')) LIKE ?1
               OR lower(coalesce(outcome, '')) LIKE ?1
            "#,
        )
        .map_err(|e| format!("Could not prepare forget event query: {e}"))?;

    let rows = stmt
        .query_map([pattern], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Could not read forget event rows: {e}"))?;

    collect_ids(rows)
}

fn event_ids_since(conn: &Connection, since: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT id FROM memory_events WHERE created_at >= ?1")
        .map_err(|e| format!("Could not prepare forget-today query: {e}"))?;
    let rows = stmt
        .query_map([since], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Could not read forget-today rows: {e}"))?;

    collect_ids(rows)
}

fn collect_ids<F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<String>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<String>,
{
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|e| format!("Could not decode memory id row: {e}"))?);
    }
    Ok(ids)
}

fn delete_events_by_ids(conn: &Connection, ids: &[String]) -> Result<usize, String> {
    let mut count = 0;
    for id in ids {
        conn.execute("DELETE FROM memory_embeddings WHERE target_id = ?1", [id])
            .map_err(|e| format!("Could not delete event embedding '{id}': {e}"))?;
        conn.execute("DELETE FROM memory_events_fts WHERE event_id = ?1", [id])
            .map_err(|e| format!("Could not delete event FTS row '{id}': {e}"))?;
        count += conn
            .execute("DELETE FROM memory_events WHERE id = ?1", [id])
            .map_err(|e| format!("Could not forget memory event '{id}': {e}"))?;
    }

    Ok(count)
}

fn delete_orphan_embeddings(conn: &Connection) -> Result<usize, String> {
    conn.execute(
        r#"
        DELETE FROM memory_embeddings
        WHERE target_kind = 'event'
          AND target_id NOT IN (SELECT id FROM memory_events)
        "#,
        [],
    )
    .map_err(|e| format!("Could not delete orphan memory embeddings: {e}"))
}

fn delete_all(conn: &Connection, table: &str) -> Result<usize, String> {
    conn.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| format!("Could not wipe {table}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::{MemoryEvent, MemoryEventKind, PrivacyTier};
    use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database_in_memory};

    #[test]
    fn forget_matching_events_deletes_indexes() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_summary("Worked on project alpha.");
        let id = event.id.clone();
        insert_memory_event(&conn, &event).expect("event inserts");

        let report = forget_memory_with_connection(&conn, "alpha").expect("forget succeeds");

        assert_eq!(report.events, 1);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_events WHERE id = ?1", [id], |row| row.get(0))
            .expect("event count");
        assert_eq!(count, 0);
    }

    #[test]
    fn wipe_memory_removes_all_primary_memory_tables() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_summary("Remember this.");
        insert_memory_event(&conn, &event).expect("event inserts");

        let report = wipe_memory_with_connection(&conn).expect("wipe succeeds");

        assert_eq!(report.events, 1);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_events", [], |row| row.get(0))
            .expect("event count");
        assert_eq!(count, 0);
    }
}
