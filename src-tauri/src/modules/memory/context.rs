use rusqlite::Connection;
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryContextEvent {
    created_at: String,
    kind: String,
    source: String,
    privacy_tier: PrivacyTier,
    app_name: Option<String>,
    context_domain: Option<String>,
    user_input: Option<String>,
    summary: Option<String>,
    outcome: Option<String>,
}

pub fn build_memory_context(limit: Option<usize>) -> Result<MemoryContext, String> {
    let conn = open_memory_database()?;
    build_memory_context_from_connection(&conn, limit)
}

pub fn build_memory_context_from_connection(
    conn: &Connection,
    limit: Option<usize>,
) -> Result<MemoryContext, String> {
    let events = load_recent_events(conn, normalized_limit(limit))?;
    Ok(format_memory_context(&events))
}

fn normalized_limit(limit: Option<usize>) -> usize {
    limit
        .unwrap_or(DEFAULT_MEMORY_CONTEXT_LIMIT)
        .clamp(1, MAX_MEMORY_CONTEXT_LIMIT)
}

fn load_recent_events(conn: &Connection, limit: usize) -> Result<Vec<MemoryContextEvent>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                created_at,
                kind,
                source,
                privacy_tier,
                app_name,
                context_domain,
                user_input,
                summary,
                outcome
            FROM memory_events
            WHERE privacy_tier != 'transient'
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Could not prepare memory context query: {e}"))?;

    let rows = stmt
        .query_map([limit as i64], |row| {
            let privacy_tier: String = row.get(3)?;

            Ok(MemoryContextEvent {
                created_at: row.get(0)?,
                kind: row.get(1)?,
                source: row.get(2)?,
                privacy_tier: PrivacyTier::from_str(&privacy_tier),
                app_name: row.get(4)?,
                context_domain: row.get(5)?,
                user_input: row.get(6)?,
                summary: row.get(7)?,
                outcome: row.get(8)?,
            })
        })
        .map_err(|e| format!("Could not read memory context rows: {e}"))?;

    let mut events = Vec::new();
    for row in rows {
        events.push(row.map_err(|e| format!("Could not decode memory context row: {e}"))?);
    }

    Ok(events)
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
            build_memory_context_from_connection(&conn, None).expect("context builds");

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
            build_memory_context_from_connection(&conn, Some(10)).expect("context builds");

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
            build_memory_context_from_connection(&conn, Some(10)).expect("context builds");

        assert!(context.memory.contains("snip event from screen"));
        assert!(!context.memory.contains("secret visible text"));
    }
}
