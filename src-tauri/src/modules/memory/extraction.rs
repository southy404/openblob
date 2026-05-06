use std::time::Duration;

use reqwest::blocking::Client;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::json;

use crate::modules::memory::events::{MemoryEvent, MemoryEventKind, PrivacyTier};
use crate::modules::memory::facts::{insert_memory_fact_superseding, ExtractedMemoryFact};
#[cfg(not(test))]
use crate::modules::profile::companion_config::load_or_create_companion_config;

#[derive(Debug, Deserialize)]
struct OllamaFactResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

pub fn extract_and_store_facts_for_event(
    conn: &Connection,
    event: &MemoryEvent,
) -> Result<usize, String> {
    #[cfg(not(test))]
    {
        if !load_or_create_companion_config()
            .map(|config| config.privacy.store_semantic_memory)
            .unwrap_or(true)
        {
            return Ok(0);
        }
    }

    if !event_allows_fact_extraction(event) {
        return Ok(0);
    }

    let mut facts = deterministic_facts_for_event(event);
    if facts.is_empty() {
        facts = ollama_facts_for_event(event).unwrap_or_default();
    }

    let mut inserted = 0;
    for (index, fact) in facts.into_iter().enumerate() {
        if let Some(fact) = fact.into_memory_fact(&event.id, "semantic_extraction", index) {
            if insert_memory_fact_superseding(conn, &fact)? {
                inserted += 1;
            }
        }
    }

    Ok(inserted)
}

pub fn deterministic_facts_for_event(event: &MemoryEvent) -> Vec<ExtractedMemoryFact> {
    let Some(text) = fact_source_text(event) else {
        return Vec::new();
    };

    deterministic_facts_from_text(&text)
}

fn deterministic_facts_from_text(text: &str) -> Vec<ExtractedMemoryFact> {
    let mut facts = Vec::new();
    let text = text.trim();
    let lower = text.to_lowercase();

    for prefix in ["my name is ", "i am called ", "call me "] {
        if let Some(value) = strip_prefix_value(text, &lower, prefix) {
            facts.push(fact("user", "name", value, 0.88));
        }
    }

    if let Some(value) = strip_prefix_value(text, &lower, "my preferred language is ") {
        facts.push(fact("user", "preferred_language", value, 0.82));
    }

    for prefix in ["i am working on ", "i'm working on ", "currently working on "] {
        if let Some(value) = strip_prefix_value(text, &lower, prefix) {
            facts.push(fact("user", "working_on", value, 0.78));
        }
    }

    for prefix in ["my project is ", "my main project is ", "i own project "] {
        if let Some(value) = strip_prefix_value(text, &lower, prefix) {
            facts.push(fact("user", "owns_project", value, 0.80));
        }
    }

    if let Some((thing, name)) = parse_named_possession(text, &lower) {
        facts.push(fact(format!("user.{thing}"), "name", name, 0.84));
    }

    if let Some((thing, value)) = parse_my_thing_is(text, &lower) {
        facts.push(fact(format!("user.{thing}"), "value", value, 0.72));
    }

    facts
}

fn ollama_facts_for_event(event: &MemoryEvent) -> Result<Vec<ExtractedMemoryFact>, String> {
    let Some(text) = fact_source_text(event) else {
        return Ok(Vec::new());
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("Could not create fact extraction client: {e}"))?;

    let prompt = format!(
        r#"Extract durable user memory facts from this event.
Return ONLY a JSON array. Each item must have subject, predicate, object, confidence.
Use simple subjects like "user", "user.project", "user.pet".
Use predicates like "name", "preferred_language", "owns_project", "working_on", "likes".
Ignore temporary commands, private content, secrets, and uncertain details.

EVENT:
{}"#,
        text
    );

    let response = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&json!({
            "model": "llama3.1:8b",
            "stream": false,
            "keep_alive": "10m",
            "messages": [
                { "role": "system", "content": "You extract concise durable private memory facts for a local desktop companion." },
                { "role": "user", "content": prompt }
            ]
        }))
        .send()
        .map_err(|e| format!("Could not call Ollama fact extraction: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Ollama fact extraction failed with {}", response.status()));
    }

    let parsed = response
        .json::<OllamaFactResponse>()
        .map_err(|e| format!("Could not decode fact extraction response: {e}"))?;

    parse_fact_json(&parsed.message.content)
}

fn event_allows_fact_extraction(event: &MemoryEvent) -> bool {
    event.privacy_tier != PrivacyTier::Transient
        && event.privacy_tier != PrivacyTier::MetadataOnly
        && !matches!(
            event.kind,
            MemoryEventKind::Snip | MemoryEventKind::TranscriptSegment
        )
}

fn fact_source_text(event: &MemoryEvent) -> Option<String> {
    let text = [event.user_input.as_deref(), event.summary.as_deref()]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn parse_fact_json(raw: &str) -> Result<Vec<ExtractedMemoryFact>, String> {
    let raw = raw.trim();
    let without_prefix = raw
        .strip_prefix("```json")
        .or_else(|| raw.strip_prefix("```"))
        .unwrap_or(raw)
        .trim();
    let json = without_prefix
        .strip_suffix("```")
        .unwrap_or(without_prefix)
        .trim();

    serde_json::from_str::<Vec<ExtractedMemoryFact>>(json)
        .map_err(|e| format!("Could not parse extracted fact JSON: {e}"))
}

fn strip_prefix_value<'a>(text: &'a str, lower: &str, prefix: &str) -> Option<String> {
    if !lower.starts_with(prefix) {
        return None;
    }

    clean_value(&text[prefix.len()..])
}

fn parse_named_possession(text: &str, lower: &str) -> Option<(String, String)> {
    if !lower.starts_with("my ") {
        return None;
    }

    let rest = &text[3..];
    let lower_rest = &lower[3..];
    let marker = " is named ";
    let index = lower_rest.find(marker)?;
    let thing = normalized_thing(&rest[..index])?;
    let name = clean_value(&rest[index + marker.len()..])?;

    Some((thing, name))
}

fn parse_my_thing_is(text: &str, lower: &str) -> Option<(String, String)> {
    if !lower.starts_with("my ") {
        return None;
    }

    let rest = &text[3..];
    let lower_rest = &lower[3..];
    if lower_rest.contains(" is named ") {
        return None;
    }

    let marker = " is ";
    let index = lower_rest.find(marker)?;
    let thing = normalized_thing(&rest[..index])?;
    let value = clean_value(&rest[index + marker.len()..])?;

    if matches!(thing.as_str(), "name" | "project" | "main_project") || value.len() > 80 {
        return None;
    }

    Some((thing, value))
}

fn fact(
    subject: impl Into<String>,
    predicate: impl Into<String>,
    object: impl Into<String>,
    confidence: f32,
) -> ExtractedMemoryFact {
    ExtractedMemoryFact {
        subject: subject.into(),
        predicate: predicate.into(),
        object: object.into(),
        confidence,
    }
}

fn clean_value(value: &str) -> Option<String> {
    let value = value
        .trim()
        .trim_matches(|ch: char| ch == '.' || ch == '!' || ch == '?' || ch == '"' || ch == '\'')
        .trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalized_thing(value: &str) -> Option<String> {
    let value = value
        .trim()
        .to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::{MemoryEvent, MemoryEventKind};
    use crate::modules::memory::sqlite_store::open_memory_database_in_memory;

    #[test]
    fn deterministic_extractor_catches_named_pet() {
        let facts = deterministic_facts_from_text("my cat is named Ash");

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "user.cat");
        assert_eq!(facts[0].predicate, "name");
        assert_eq!(facts[0].object, "Ash");
    }

    #[test]
    fn deterministic_extractor_catches_project() {
        let facts = deterministic_facts_from_text("I am working on NeuralScript");

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "user");
        assert_eq!(facts[0].predicate, "working_on");
        assert_eq!(facts[0].object, "NeuralScript");
    }

    #[test]
    fn stores_extracted_fact_for_event() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let event = MemoryEvent::new(MemoryEventKind::ChatTurn, "test", PrivacyTier::Redacted)
            .with_user_input("my cat is named Ash");

        let inserted =
            extract_and_store_facts_for_event(&conn, &event).expect("facts extracted");

        assert_eq!(inserted, 1);
        let object: String = conn
            .query_row("SELECT object FROM memory_facts WHERE subject = 'user.cat'", [], |row| {
                row.get(0)
            })
            .expect("stored fact");
        assert_eq!(object, "Ash");
    }
}
