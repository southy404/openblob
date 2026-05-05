use chrono::Utc;
use rusqlite::{params, Connection};
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
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub valid_from: String,
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

pub fn load_active_memory_facts(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<ActiveMemoryFact>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT subject, predicate, object, confidence, valid_from
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
                subject: row.get(0)?,
                predicate: row.get(1)?,
                object: row.get(2)?,
                confidence: row.get(3)?,
                valid_from: row.get(4)?,
            })
        })
        .map_err(|e| format!("Could not read memory fact rows: {e}"))?;

    let mut facts = Vec::new();
    for row in rows {
        facts.push(row.map_err(|e| format!("Could not decode memory fact row: {e}"))?);
    }

    Ok(facts)
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
}
