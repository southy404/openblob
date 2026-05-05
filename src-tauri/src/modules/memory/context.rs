use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Serialize;

use crate::modules::memory::events::PrivacyTier;
use crate::modules::memory::sqlite_store::open_memory_database;

const DEFAULT_MEMORY_CONTEXT_LIMIT: usize = 12;
const MAX_MEMORY_CONTEXT_LIMIT: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemoryContext {
    pub memory: String,
    pub event_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct MemoryContextEvent {
    id: String,
    created_at: String,
    kind: String,
    source: String,
    privacy_tier: PrivacyTier,
    app_name: Option<String>,
    context_domain: Option<String>,
    user_input: Option<String>,
    summary: Option<String>,
    outcome: Option<String>,
    importance: f32,
    search_rank: Option<f64>,
}

pub fn build_memory_context(limit: Option<usize>) -> Result<MemoryContext, String> {
    build_memory_context_for_query(None, limit)
}

pub fn build_memory_context_for_query(
    query: Option<&str>,
    limit: Option<usize>,
) -> Result<MemoryContext, String> {
    let conn = open_memory_database()?;
    build_memory_context_from_connection(&conn, query, limit)
}

pub fn build_memory_context_from_connection(
    conn: &Connection,
    query: Option<&str>,
    limit: Option<usize>,
) -> Result<MemoryContext, String> {
    let events = load_ranked_events(conn, query, normalized_limit(limit))?;
    Ok(format_memory_context(&events))
}

fn normalized_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_MEMORY_CONTEXT_LIMIT)
        .clamp(1, MAX_MEMORY_CONTEXT_LIMIT)
}

fn load_ranked_events(
    conn: &Connection,
    query: Option<&str>,
    limit: usize,
) -> Result<Vec<MemoryContextEvent>, String> {
    let query = query.map(str::trim).filter(|value| !value.is_empty());
    let mut events = if let Some(query) = query {
        load_fts_events(conn, query, limit.saturating_mul(3))?
    } else {
        Vec::new()
    };

    if events.len() < limit {
        for event in load_recent_events(conn, limit.saturating_mul(2))? {
            if !events.iter().any(|existing| existing.id == event.id) {
                events.push(event);
            }
        }
    }

    events.sort_by(|a, b| {
        event_score(b)
            .partial_cmp(&event_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    events.truncate(limit);

    Ok(events)
}

fn load_fts_events(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<MemoryContextEvent>, String> {
    let query = escape_fts_query(query);
    if query.is_empty() {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                e.id,
                e.created_at,
                e.kind,
                e.source,
                e.privacy_tier,
                e.app_name,
                e.context_domain,
                e.user_input,
                e.summary,
                e.outcome,
                e.importance,
                bm25(memory_events_fts) AS rank
            FROM memory_events_fts
            JOIN memory_events e ON e.id = memory_events_fts.event_id
            WHERE memory_events_fts MATCH ?1
              AND e.privacy_tier != 'transient'
            ORDER BY rank
            LIMIT ?2
            "#,
        )
        .map_err(|e| format!("Could not prepare memory FTS query: {e}"))?;

    let rows = stmt
        .query_map(params![query, limit as i64], decode_context_row)
        .map_err(|e| format!("Could not read memory FTS rows: {e}"))?;

    collect_context_rows(rows)
}

fn load_recent_events(conn: &Connection, limit: usize) -> Result<Vec<MemoryContextEvent>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                id,
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
                NULL AS rank
            FROM memory_events
            WHERE privacy_tier != 'transient'
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Could not prepare memory context query: {e}"))?;

    let rows = stmt
        .query_map([limit as i64], decode_context_row)
        .map_err(|e| format!("Could not read memory context rows: {e}"))?;

    collect_context_rows(rows)
}

fn collect_context_rows<F>(rows: rusqlite::MappedRows<'_, F>) -> Result<Vec<MemoryContextEvent>, String>
where
    F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<MemoryContextEvent>,
{
    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|e| format!("Could not decode memory context row: {e}"))?);
    }

    Ok(events)
}

fn decode_context_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryContextEvent> {
    let privacy_tier: String = row.get(4)?;

    Ok(MemoryContextEvent {
        id: row.get(0)?,
        created_at: row.get(1)?,
        kind: row.get(2)?,
        source: row.get(3)?,
        privacy_tier: PrivacyTier::from_str(&privacy_tier),
        app_name: row.get(5)?,
        context_domain: row.get(6)?,
        user_input: row.get(7)?,
        summary: row.get(8)?,
        outcome: row.get(9)?,
        importance: row.get(10)?,
        search_rank: row.get(11)?,
    })
}

fn event_score(event: &MemoryContextEvent) -> f64 {
    let text_score = event
        .search_rank
        .map(|rank| 1.0 / (1.0 + rank.abs()))
        .unwrap_or(0.0);
    let recency_score = recency_score(&event.created_at);
    let importance_score = event.importance.clamp(0.0, 1.0) as f64;

    (text_score * 0.55) + (recency_score * 0.25) + (importance_score * 0.20)
}

fn recency_score(created_at: &str) -> f64 {
    let Ok(created_at) = DateTime::parse_from_rfc3339(created_at) else {
        return 0.0;
    };

    let age_hours = Utc::now()
        .signed_duration_since(created_at.with_timezone(&Utc))
        .num_hours()
        .max(0) as f64;

    1.0 / (1.0 + (age_hours / 24.0))
}

fn escape_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|term| {
            let escaped = term.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn format_memory_context(events: &[MemoryContextEvent]) -> MemoryContext {
    if events.is_empty() {
        return MemoryContext {
            memory: String::new(),
            event_count: 0,
        };
    }

    let mut lines = vec!["<memory>".to_string(), "## Recent activity".to_string()];

    for event in events {
        if let Some(line) = format_event_line(event) {
            lines.push(line);
        }
    }

    lines.push("</memory>".to_string());

    MemoryContext {
        memory: lines.join("\n"),
        event_count: events.len(),
    }
}

fn format_event_line(event: &MemoryContextEvent) -> Option<String> {
    let text = event_text(event)?;
    let location = event_location(event);

    let mut line = format!("- {}{}: {}", event.created_at, location, text);

    if let Some(outcome) = clean(&event.outcome) {
        line.push_str(&format!(" ({outcome})"));
    }

    Some(line)
}

fn event_text(event: &MemoryContextEvent) -> Option<String> {
    if let Some(summary) = clean(&event.summary) {
        return Some(summary);
    }

    if event.privacy_tier == PrivacyTier::MetadataOnly {
        return Some(format!("{} event from {}", event.kind, event.source));
    }

    clean(&event.user_input)
}

fn event_location(event: &MemoryContextEvent) -> String {
    let app = clean(&event.app_name);
    let domain = clean(&event.context_domain);

    match (app, domain) {
        (Some(app), Some(domain)) if app != domain => format!(" [{domain}/{app}]"),
        (Some(app), _) => format!(" [{app}]"),
        (_, Some(domain)) => format!(" [{domain}]"),
        _ => String::new(),
    }
}

fn clean(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::{MemoryEvent, MemoryEventKind};
    use crate::modules::memory::sqlite_store::{
        insert_memory_event, open_memory_database_in_memory,
    };

    #[test]
    fn empty_database_returns_empty_memory_block() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let context =
            build_memory_context_from_connection(&conn, None, None).expect("context builds");

        assert_eq!(context.memory, "");
        assert_eq!(context.event_count, 0);
    }

    #[test]
    fn recent_events_are_formatted_as_memory_block() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_app_name("Terminal")
            .with_context_domain("desktop")
            .with_user_input("run tests")
            .with_summary("User ran the memory test suite.")
            .with_outcome("success");

        insert_memory_event(&conn, &event).expect("event inserts");

        let context =
            build_memory_context_from_connection(&conn, None, Some(10)).expect("context builds");

        assert_eq!(context.event_count, 1);
        assert!(context.memory.starts_with("<memory>\n## Recent activity\n- "));
        assert!(context
            .memory
            .contains("[desktop/Terminal]: User ran the memory test suite. (success)"));
        assert!(context.memory.ends_with("\n</memory>"));
    }

    #[test]
    fn metadata_only_events_do_not_expose_raw_input_without_summary() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(
            MemoryEventKind::Snip,
            "screen",
            PrivacyTier::MetadataOnly,
        )
        .with_context_domain("screen")
        .with_user_input("secret visible text");

        insert_memory_event(&conn, &event).expect("event inserts");

        let context =
            build_memory_context_from_connection(&conn, None, Some(10)).expect("context builds");

        assert!(context.memory.contains("snip event from screen"));
        assert!(!context.memory.contains("secret visible text"));
    }

    #[test]
    fn query_terms_prioritize_fts_matches_over_recency() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let relevant = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_app_name("Editor")
            .with_context_domain("desktop")
            .with_summary("User worked on the NeuralScript parser.")
            .with_importance(0.9);
        let irrelevant =
            MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
                .with_app_name("Browser")
                .with_context_domain("desktop")
                .with_summary("User opened a music website.")
                .with_importance(0.1);

        insert_memory_event(&conn, &relevant).expect("relevant event inserts");
        insert_memory_event(&conn, &irrelevant).expect("irrelevant event inserts");

        let context = build_memory_context_from_connection(&conn, Some("NeuralScript"), Some(1))
            .expect("context builds");

        assert!(context.memory.contains("NeuralScript parser"));
        assert!(!context.memory.contains("music website"));
    }
}
