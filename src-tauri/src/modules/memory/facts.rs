use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryFact {
    pub id: String,
    pub source_key: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub superseded_by: Option<String>,
    pub source: String,
    pub metadata: Value,
}

impl MemoryFact {
    pub fn new(
        source_key: impl Into<String>,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("fact_{}", Uuid::now_v7()),
            source_key: source_key.into(),
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence: 0.75,
            valid_from: Utc::now().to_rfc3339(),
            valid_to: None,
            superseded_by: None,
            source: source.into(),
            metadata: json!({}),
        }
    }

    pub fn with_confidence(mut self, value: f32) -> Self {
        self.confidence = value.clamp(0.0, 1.0);
        self
    }

    pub fn with_metadata(mut self, value: Value) -> Self {
        self.metadata = value;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveMemoryFact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub valid_from: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedMemoryFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    #[serde(default = "default_extracted_confidence")]
    pub confidence: f32,
}

pub fn insert_memory_fact(conn: &Connection, fact: &MemoryFact) -> Result<(), String> {
    let metadata_json = serde_json::to_string(&fact.metadata)
        .map_err(|e| format!("Could not serialize memory fact metadata: {e}"))?;

    conn.execute(
        r#"
        INSERT OR IGNORE INTO memory_facts (
            id,
            source_key,
            subject,
            predicate,
            object,
            confidence,
            valid_from,
            valid_to,
            superseded_by,
            source,
            metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            fact.id.as_str(),
            fact.source_key.as_str(),
            fact.subject.as_str(),
            fact.predicate.as_str(),
            fact.object.as_str(),
            fact.confidence,
            fact.valid_from.as_str(),
            fact.valid_to.as_deref(),
            fact.superseded_by.as_deref(),
            fact.source.as_str(),
            metadata_json,
        ],
    )
    .map_err(|e| format!("Could not insert memory fact '{}': {e}", fact.source_key))?;

    Ok(())
}

pub fn insert_memory_fact_superseding(
    conn: &Connection,
    fact: &MemoryFact,
) -> Result<bool, String> {
    if fact.subject.trim().is_empty()
        || fact.predicate.trim().is_empty()
        || fact.object.trim().is_empty()
    {
        return Ok(false);
    }

    if active_equivalent_fact_exists(conn, fact)? {
        return Ok(false);
    }

    let metadata_json = serde_json::to_string(&fact.metadata)
        .map_err(|e| format!("Could not serialize memory fact metadata: {e}"))?;

    conn.execute(
        r#"
        INSERT OR IGNORE INTO memory_facts (
            id,
            source_key,
            subject,
            predicate,
            object,
            confidence,
            valid_from,
            valid_to,
            superseded_by,
            source,
            metadata_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            fact.id.as_str(),
            fact.source_key.as_str(),
            fact.subject.as_str(),
            fact.predicate.as_str(),
            fact.object.as_str(),
            fact.confidence,
            fact.valid_from.as_str(),
            fact.valid_to.as_deref(),
            fact.superseded_by.as_deref(),
            fact.source.as_str(),
            metadata_json,
        ],
    )
    .map_err(|e| format!("Could not insert superseding memory fact '{}': {e}", fact.source_key))?;

    let inserted = conn.changes() > 0;
    if inserted {
        conn.execute(
            r#"
            UPDATE memory_facts
            SET valid_to = ?1,
                superseded_by = ?2
            WHERE subject = ?3
              AND predicate = ?4
              AND valid_to IS NULL
              AND id != ?2
              AND lower(object) != lower(?5)
            "#,
            params![
                fact.valid_from.as_str(),
                fact.id.as_str(),
                fact.subject.as_str(),
                fact.predicate.as_str(),
                fact.object.as_str(),
            ],
        )
        .map_err(|e| format!("Could not supersede old memory facts: {e}"))?;
    }

    Ok(inserted)
}

pub fn load_active_memory_facts(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<ActiveMemoryFact>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, subject, predicate, object, confidence, valid_from
            FROM memory_facts
            WHERE valid_to IS NULL
            ORDER BY confidence DESC, valid_from DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| format!("Could not prepare memory facts query: {e}"))?;

    let rows = stmt
        .query_map([limit as i64], |row| {
            Ok(ActiveMemoryFact {
                id: row.get(0)?,
                subject: row.get(1)?,
                predicate: row.get(2)?,
                object: row.get(3)?,
                confidence: row.get(4)?,
                valid_from: row.get(5)?,
            })
        })
        .map_err(|e| format!("Could not read memory fact rows: {e}"))?;

    let mut facts = Vec::new();
    for row in rows {
        facts.push(row.map_err(|e| format!("Could not decode memory fact row: {e}"))?);
    }

    Ok(facts)
}

impl ExtractedMemoryFact {
    pub fn into_memory_fact(
        self,
        event_id: &str,
        extractor: &str,
        index: usize,
    ) -> Option<MemoryFact> {
        let subject = normalized_token(&self.subject)?;
        let predicate = normalized_token(&self.predicate)?;
        let object = normalized_object(&self.object)?;
        let source_key = format!(
            "event:{event_id}:{extractor}:{index}:{}:{}:{}",
            subject,
            predicate,
            stable_key(&object)
        );

        Some(
            MemoryFact::new(source_key, subject, predicate, object, extractor)
                .with_confidence(self.confidence)
                .with_metadata(json!({
                    "event_id": event_id,
                    "extractor": extractor
                })),
        )
    }
}

fn active_equivalent_fact_exists(conn: &Connection, fact: &MemoryFact) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM memory_facts
            WHERE subject = ?1
              AND predicate = ?2
              AND lower(object) = lower(?3)
              AND valid_to IS NULL
            "#,
            params![
                fact.subject.as_str(),
                fact.predicate.as_str(),
                fact.object.as_str()
            ],
            |row| row.get(0),
        )
        .map_err(|e| format!("Could not check equivalent memory fact: {e}"))?;

    Ok(count > 0)
}

fn default_extracted_confidence() -> f32 {
    0.72
}

fn normalized_token(value: &str) -> Option<String> {
    let value = value
        .trim()
        .to_lowercase()
        .replace(' ', "_")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '.')
        .collect::<String>();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn normalized_object(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn stable_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::sqlite_store::open_memory_database_in_memory;

    #[test]
    fn inserts_and_loads_active_facts() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let fact = MemoryFact::new(
            "legacy:user:language:en",
            "user",
            "preferred_language",
            "en",
            "legacy_import",
        );

        insert_memory_fact(&conn, &fact).expect("fact inserts");
        insert_memory_fact(&conn, &fact).expect("duplicate ignored");

        let facts = load_active_memory_facts(&conn, 10).expect("facts load");

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].subject, "user");
        assert_eq!(facts[0].predicate, "preferred_language");
        assert_eq!(facts[0].object, "en");
    }

    #[test]
    fn superseding_closes_old_active_fact() {
        let conn = open_memory_database_in_memory().expect("database opens");
        let first = MemoryFact::new("first", "user", "name", "Alex", "test");
        let second = MemoryFact::new("second", "user", "name", "Brandon", "test");

        assert!(insert_memory_fact_superseding(&conn, &first).expect("first inserts"));
        assert!(insert_memory_fact_superseding(&conn, &second).expect("second inserts"));

        let facts = load_active_memory_facts(&conn, 10).expect("facts load");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].object, "Brandon");

        let closed_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_facts WHERE object = 'Alex' AND valid_to IS NOT NULL AND superseded_by = ?1",
                [second.id.as_str()],
                |row| row.get(0),
            )
            .expect("closed count");
        assert_eq!(closed_count, 1);
    }

    #[test]
    fn extracted_fact_builds_stable_memory_fact() {
        let extracted = ExtractedMemoryFact {
            subject: "User".into(),
            predicate: "Owns Project".into(),
            object: "NeuralScript".into(),
            confidence: 0.8,
        };

        let fact = extracted
            .into_memory_fact("mem_1", "deterministic", 0)
            .expect("fact");

        assert_eq!(fact.subject, "user");
        assert_eq!(fact.predicate, "owns_project");
        assert_eq!(fact.object, "NeuralScript");
        assert!(fact.source_key.contains("event:mem_1:deterministic:0"));
    }
}
