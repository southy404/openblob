use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::modules::memory::sqlite_store::open_memory_database;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectiveSummary {
    pub id: String,
    pub scope: String,
    pub period_start: String,
    pub period_end: String,
    pub summary: String,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

pub fn reflect_memory(scope: &str) -> Result<ReflectiveSummary, String> {
    let conn = open_memory_database()?;
    reflect_memory_with_connection(&conn, scope)
}

pub fn reflect_memory_with_connection(
    conn: &Connection,
    scope: &str,
) -> Result<ReflectiveSummary, String> {
    let scope = normalize_scope(scope);
    let (period_start, period_end) = period_for_scope(scope);
    let source_text = load_reflection_source(conn, &period_start, &period_end, 60)?;

    let (summary, source) = if source_text.trim().is_empty() {
        ("No notable memory activity in this period.".to_string(), "deterministic".to_string())
    } else {
        match summarize_with_ollama(scope, &source_text) {
            Ok(summary) if !summary.trim().is_empty() => (summary, "ollama".to_string()),
            _ => (fallback_summary(scope, &source_text), "deterministic".to_string()),
        }
    };

    let summary = ReflectiveSummary {
        id: format!("summary_{}", Uuid::now_v7()),
        scope: scope.to_string(),
        period_start,
        period_end,
        summary,
        source,
        created_at: Utc::now().to_rfc3339(),
    };

    insert_reflective_summary(conn, &summary)?;
    Ok(summary)
}

pub fn insert_reflective_summary(
    conn: &Connection,
    summary: &ReflectiveSummary,
) -> Result<(), String> {
    conn.execute(
        r#"
        INSERT OR REPLACE INTO memory_summaries (
            id,
            scope,
            period_start,
            period_end,
            summary,
            source,
            created_at,
            metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            summary.id.as_str(),
            summary.scope.as_str(),
            summary.period_start.as_str(),
            summary.period_end.as_str(),
            summary.summary.as_str(),
            summary.source.as_str(),
            summary.created_at.as_str(),
            json!({ "generator": summary.source }).to_string(),
        ],
    )
    .map_err(|e| format!("Could not insert reflective memory summary: {e}"))?;

    Ok(())
}

pub fn load_recent_reflective_summaries(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<ReflectiveSummary>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, scope, period_start, period_end, summary, source, created_at
            FROM memory_summaries
            ORDER BY period_end DESC, created_at DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Could not prepare reflective summaries query: {e}"))?;

    let rows = stmt
        .query_map([limit as i64], |row| {
            Ok(ReflectiveSummary {
                id: row.get(0)?,
                scope: row.get(1)?,
                period_start: row.get(2)?,
                period_end: row.get(3)?,
                summary: row.get(4)?,
                source: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("Could not read reflective summary rows: {e}"))?;

    let mut summaries = Vec::new();
    for row in rows {
        summaries.push(row.map_err(|e| format!("Could not decode reflective summary row: {e}"))?);
    }

    Ok(summaries)
}

fn load_reflection_source(
    conn: &Connection,
    period_start: &str,
    period_end: &str,
    limit: usize,
) -> Result<String, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT created_at, coalesce(app_name, source), coalesce(summary, user_input, outcome, kind)
            FROM memory_events
            WHERE created_at >= ?1
              AND created_at <= ?2
              AND privacy_tier != 'transient'
            ORDER BY created_at DESC
            LIMIT ?3
            "#,
        )
        .map_err(|e| format!("Could not prepare reflection source query: {e}"))?;

    let rows = stmt
        .query_map(params![period_start, period_end, limit as i64], |row| {
            let created_at: String = row.get(0)?;
            let source: String = row.get(1)?;
            let text: String = row.get(2)?;
            Ok(format!("- {created_at} [{source}]: {text}"))
        })
        .map_err(|e| format!("Could not read reflection source rows: {e}"))?;

    let mut lines = Vec::new();
    for row in rows {
        lines.push(row.map_err(|e| format!("Could not decode reflection source row: {e}"))?);
    }

    Ok(lines.join("\n"))
}

fn summarize_with_ollama(scope: &str, source_text: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("Could not create Ollama reflection client: {e}"))?;

    let prompt = format!(
        "Summarize this local memory activity as a concise {scope} companion memory. Keep durable facts, projects, preferences, and unresolved work. Do not invent details.\n\n{source_text}"
    );

    let response = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({
            "model": "llama3.1:8b",
            "stream": false,
            "keep_alive": "10m",
            "messages": [
                { "role": "system", "content": "You write concise private memory summaries for a local desktop companion." },
                { "role": "user", "content": prompt }
            ]
        }))
        .send()
        .map_err(|e| format!("Could not call Ollama reflection: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Ollama reflection failed with {}", response.status()));
    }

    let parsed = response
        .json::<OllamaChatResponse>()
        .map_err(|e| format!("Could not decode Ollama reflection response: {e}"))?;

    Ok(parsed.message.content.trim().to_string())
}

fn fallback_summary(scope: &str, source_text: &str) -> String {
    let mut lines = source_text
        .lines()
        .take(8)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if lines.is_empty() {
        return "No notable memory activity in this period.".into();
    }

    lines.insert(0, match scope {
        "weekly" => "This week:",
        "daily" => "Today:",
        _ => "This session:",
    });

    lines.join("\n")
}

fn normalize_scope(scope: &str) -> &'static str {
    match scope.trim().to_lowercase().as_str() {
        "weekly" | "week" => "weekly",
        "session" => "session",
        _ => "daily",
    }
}

fn period_for_scope(scope: &str) -> (String, String) {
    let end = Utc::now();
    let start = match scope {
        "weekly" => end - ChronoDuration::days(7),
        "session" => end - ChronoDuration::hours(6),
        _ => end - ChronoDuration::days(1),
    };

    (start.to_rfc3339(), end.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::{MemoryEvent, MemoryEventKind, PrivacyTier};
    use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database_in_memory};

    #[test]
    fn stores_deterministic_summary_when_source_is_empty() {
        let conn = open_memory_database_in_memory().expect("database opens");

        let summary = reflect_memory_with_connection(&conn, "daily").expect("summary");

        assert_eq!(summary.scope, "daily");
        assert_eq!(summary.source, "deterministic");
        assert!(summary.summary.contains("No notable"));
    }

    #[test]
    fn recent_summaries_are_loaded_for_context() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::Command, "desktop", PrivacyTier::Redacted)
            .with_summary("User worked on long-term memory.");
        insert_memory_event(&conn, &event).expect("event inserts");

        let summary = reflect_memory_with_connection(&conn, "session").expect("summary");
        let summaries = load_recent_reflective_summaries(&conn, 3).expect("summaries");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, summary.id);
    }
}
