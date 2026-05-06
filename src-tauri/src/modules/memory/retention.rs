use chrono::{Duration, Utc};

use crate::modules::memory::management::MemoryMutationReport;
use crate::modules::memory::sqlite_store::open_memory_database;
use crate::modules::profile::companion_config::MemoryConfig;

pub fn apply_memory_retention(config: &MemoryConfig) -> Result<MemoryMutationReport, String> {
    let conn = open_memory_database()?;
    apply_memory_retention_with_connection(&conn, config)
}

pub fn apply_memory_retention_with_connection(
    conn: &rusqlite::Connection,
    config: &MemoryConfig,
) -> Result<MemoryMutationReport, String> {
    let event_cutoff = (Utc::now() - Duration::days(config.retention_days as i64)).to_rfc3339();
    let summary_cutoff =
        (Utc::now() - Duration::days(config.summary_retention_days as i64)).to_rfc3339();

    let old_ids = ids_for_query(
        conn,
        "SELECT id FROM memory_events WHERE created_at < ?1",
        &event_cutoff,
    )?;
    let mut events = delete_event_ids(conn, &old_ids)?;

    let overflow_ids = ids_for_overflow(conn, config.max_events as usize)?;
    events += delete_event_ids(conn, &overflow_ids)?;

    let summaries = conn
        .execute("DELETE FROM memory_summaries WHERE created_at < ?1", [&summary_cutoff])
        .map_err(|e| format!("Could not delete expired memory summaries: {e}"))?;

    let facts = conn
        .execute(
            "DELETE FROM memory_facts WHERE valid_to IS NOT NULL AND valid_to < ?1",
            [&event_cutoff],
        )
        .map_err(|e| format!("Could not delete expired memory facts: {e}"))?;

    let embeddings = conn
        .execute(
            r#"
            DELETE FROM memory_embeddings
            WHERE target_kind = 'event'
              AND target_id NOT IN (SELECT id FROM memory_events)
            "#,
            [],
        )
        .map_err(|e| format!("Could not delete expired memory embeddings: {e}"))?;
    let _ = conn.execute(
        r#"
        DELETE FROM memory_embedding_vec
        WHERE target_id NOT IN (SELECT id FROM memory_events)
        "#,
        [],
    );

    Ok(MemoryMutationReport {
        events,
        facts,
        summaries,
        embeddings,
    })
}

pub fn decayed_importance(
    importance: f32,
    created_at: &str,
    half_life_days: u32,
) -> f64 {
    let Ok(created_at) = chrono::DateTime::parse_from_rfc3339(created_at) else {
        return importance.clamp(0.0, 1.0) as f64;
    };

    let age_days = Utc::now()
        .signed_duration_since(created_at.with_timezone(&Utc))
        .num_hours()
        .max(0) as f64
        / 24.0;
    let half_life_days = half_life_days.max(1) as f64;
    let multiplier = 0.5_f64.powf(age_days / half_life_days);

    let importance = importance.clamp(0.0, 1.0) as f64;
    (importance * multiplier).min(importance)
}

fn ids_for_query(
    conn: &rusqlite::Connection,
    sql: &str,
    param: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Could not prepare retention id query: {e}"))?;
    let rows = stmt
        .query_map([param], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Could not read retention id rows: {e}"))?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|e| format!("Could not decode retention id row: {e}"))?);
    }
    Ok(ids)
}

fn ids_for_overflow(
    conn: &rusqlite::Connection,
    max_events: usize,
) -> Result<Vec<String>, String> {
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM memory_events", [], |row| row.get(0))
        .map_err(|e| format!("Could not count memory events for retention: {e}"))?;

    if total <= max_events as i64 {
        return Ok(Vec::new());
    }

    let offset = max_events as i64;
    let mut stmt = conn
        .prepare("SELECT id FROM memory_events ORDER BY created_at DESC LIMIT -1 OFFSET ?1")
        .map_err(|e| format!("Could not prepare max-events retention query: {e}"))?;
    let rows = stmt
        .query_map([offset], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Could not read max-events retention rows: {e}"))?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|e| format!("Could not decode max-events retention row: {e}"))?);
    }
    Ok(ids)
}

fn delete_event_ids(conn: &rusqlite::Connection, ids: &[String]) -> Result<usize, String> {
    let mut count = 0;
    for id in ids {
        conn.execute("DELETE FROM memory_embeddings WHERE target_id = ?1", [id])
            .map_err(|e| format!("Could not delete retained embedding '{id}': {e}"))?;
        let _ = conn.execute("DELETE FROM memory_embedding_vec WHERE target_id = ?1", [id]);
        conn.execute("DELETE FROM memory_events_fts WHERE event_id = ?1", [id])
            .map_err(|e| format!("Could not delete retained FTS row '{id}': {e}"))?;
        count += conn
            .execute("DELETE FROM memory_events WHERE id = ?1", [id])
            .map_err(|e| format!("Could not delete retained event '{id}': {e}"))?;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::{MemoryEvent, MemoryEventKind, PrivacyTier};
    use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database_in_memory};

    #[test]
    fn max_events_retention_keeps_newest_events() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let old = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_summary("old event");
        let newest = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_summary("new event");
        let newest_id = newest.id.clone();

        insert_memory_event(&conn, &old).expect("old inserts");
        insert_memory_event(&conn, &newest).expect("new inserts");

        let config = MemoryConfig {
            max_events: 1,
            ..MemoryConfig::default()
        };
        let report = apply_memory_retention_with_connection(&conn, &config).expect("retention");

        assert_eq!(report.events, 1);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_events WHERE id = ?1",
                [newest_id],
                |row| row.get(0),
            )
            .expect("newest count");
        assert_eq!(count, 1);
    }

    #[test]
    fn decayed_importance_never_increases_importance() {
        let score = decayed_importance(0.8, &Utc::now().to_rfc3339(), 30);
        assert!(score <= 0.800001);
        assert!(score > 0.0);
    }
}
