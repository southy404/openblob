use std::time::Duration;

use chrono::Utc;
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::modules::memory::events::{MemoryEvent, PrivacyTier};
use crate::modules::memory::sqlite_store::{
    sqlite_vec_available, DEFAULT_EMBEDDING_DIMENSIONS,
};

pub const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";

#[derive(Debug, Clone, PartialEq)]
pub struct StoredEmbedding {
    pub target_id: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorMatch {
    pub target_id: String,
    pub score: f64,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

pub fn embedding_text_for_event(event: &MemoryEvent) -> Option<String> {
    if event.privacy_tier == PrivacyTier::Transient {
        return None;
    }

    let mut parts = vec![
        event.kind.as_str().to_string(),
        event.source.clone(),
    ];

    if let Some(value) = &event.app_name {
        parts.push(value.clone());
    }
    if let Some(value) = &event.context_domain {
        parts.push(value.clone());
    }
    if let Some(value) = &event.summary {
        parts.push(value.clone());
    }
    if event.privacy_tier != PrivacyTier::MetadataOnly {
        if let Some(value) = &event.user_input {
            parts.push(value.clone());
        }
    }
    if let Some(value) = &event.outcome {
        parts.push(value.clone());
    }

    let text = parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn embed_text_with_ollama(text: &str, timeout: Duration) -> Result<Vec<f32>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let client = Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Could not create Ollama embedding client: {e}"))?;

    let response = client
        .post("http://127.0.0.1:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": DEFAULT_EMBEDDING_MODEL,
            "prompt": text
        }))
        .send()
        .map_err(|e| format!("Could not call Ollama embeddings: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Ollama embeddings failed with {}", response.status()));
    }

    let parsed = response
        .json::<OllamaEmbeddingResponse>()
        .map_err(|e| format!("Could not decode Ollama embedding response: {e}"))?;

    Ok(parsed.embedding)
}

pub fn try_embed_event(conn: &Connection, event: &MemoryEvent) -> Result<(), String> {
    let Some(text) = embedding_text_for_event(event) else {
        return Ok(());
    };

    let vector = embed_text_with_ollama(&text, Duration::from_millis(750))?;
    if vector.is_empty() {
        return Ok(());
    }

    insert_embedding(conn, event.id.as_str(), "event", DEFAULT_EMBEDDING_MODEL, &vector)
}

pub fn insert_embedding(
    conn: &Connection,
    target_id: &str,
    target_kind: &str,
    model: &str,
    vector: &[f32],
) -> Result<(), String> {
    if vector.is_empty() {
        return Ok(());
    }

    let vector_json = serde_json::to_string(vector)
        .map_err(|e| format!("Could not serialize memory embedding: {e}"))?;

    conn.execute(
        r#"
        INSERT OR REPLACE INTO memory_embeddings (
            target_id,
            target_kind,
            model,
            dimensions,
            vector_json,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        params![
            target_id,
            target_kind,
            model,
            vector.len() as i64,
            vector_json,
            Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| format!("Could not insert memory embedding '{target_id}': {e}"))?;

    if vector.len() == DEFAULT_EMBEDDING_DIMENSIONS {
        let _ = insert_sqlite_vec_embedding(conn, target_id, vector);
    }

    Ok(())
}

pub fn load_sqlite_vec_event_matches(
    conn: &Connection,
    query_vector: &[f32],
    limit: usize,
) -> Result<Vec<VectorMatch>, String> {
    if query_vector.len() != DEFAULT_EMBEDDING_DIMENSIONS || !sqlite_vec_available(conn) {
        return Ok(Vec::new());
    }

    let query_json = serde_json::to_string(query_vector)
        .map_err(|e| format!("Could not serialize sqlite-vec query vector: {e}"))?;
    let mut stmt = match conn.prepare(
        r#"
        SELECT target_id, distance
        FROM memory_embedding_vec
        WHERE embedding MATCH ?1
          AND k = ?2
        ORDER BY distance
        "#,
    ) {
        Ok(stmt) => stmt,
        Err(_) => return Ok(Vec::new()),
    };

    let rows = stmt
        .query_map(params![query_json, limit as i64], |row| {
            let distance: f64 = row.get(1)?;
            Ok(VectorMatch {
                target_id: row.get(0)?,
                score: 1.0 / (1.0 + distance.max(0.0)),
            })
        })
        .map_err(|e| format!("Could not read sqlite-vec memory matches: {e}"))?;

    let mut matches = Vec::new();
    for row in rows {
        matches.push(row.map_err(|e| format!("Could not decode sqlite-vec row: {e}"))?);
    }

    Ok(matches)
}

fn insert_sqlite_vec_embedding(
    conn: &Connection,
    target_id: &str,
    vector: &[f32],
) -> Result<(), String> {
    if !sqlite_vec_available(conn) {
        return Ok(());
    }

    let vector_json = serde_json::to_string(vector)
        .map_err(|e| format!("Could not serialize sqlite-vec embedding: {e}"))?;

    conn.execute(
        "DELETE FROM memory_embedding_vec WHERE target_id = ?1",
        [target_id],
    )
    .ok();
    conn.execute(
        "INSERT INTO memory_embedding_vec(embedding, target_id) VALUES (?1, ?2)",
        params![vector_json, target_id],
    )
    .map_err(|e| format!("Could not insert sqlite-vec embedding '{target_id}': {e}"))?;

    Ok(())
}

pub fn load_event_embeddings(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<StoredEmbedding>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT target_id, vector_json
            FROM memory_embeddings
            WHERE target_kind = 'event'
              AND model = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .map_err(|e| format!("Could not prepare memory embedding query: {e}"))?;

    let rows = stmt
        .query_map(params![DEFAULT_EMBEDDING_MODEL, limit as i64], |row| {
            let vector_json: String = row.get(1)?;
            let vector = serde_json::from_str::<Vec<f32>>(&vector_json).unwrap_or_default();

            Ok(StoredEmbedding {
                target_id: row.get(0)?,
                vector,
            })
        })
        .map_err(|e| format!("Could not read memory embedding rows: {e}"))?;

    let mut embeddings = Vec::new();
    for row in rows {
        let embedding = row.map_err(|e| format!("Could not decode memory embedding row: {e}"))?;
        if !embedding.vector.is_empty() {
            embeddings.push(embedding);
        }
    }

    Ok(embeddings)
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }

    let mut dot = 0.0_f64;
    let mut a_norm = 0.0_f64;
    let mut b_norm = 0.0_f64;

    for (left, right) in a.iter().zip(b.iter()) {
        let left = *left as f64;
        let right = *right as f64;
        dot += left * right;
        a_norm += left * left;
        b_norm += right * right;
    }

    if a_norm == 0.0 || b_norm == 0.0 {
        0.0
    } else {
        dot / (a_norm.sqrt() * b_norm.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::sqlite_store::open_memory_database_in_memory;

    #[test]
    fn metadata_only_embedding_text_excludes_raw_input() {
        let event = MemoryEvent::new(
            crate::modules::memory::events::MemoryEventKind::Snip,
            "screen",
            PrivacyTier::MetadataOnly,
        )
        .with_summary("Screen was captured.")
        .with_user_input("secret visible text");

        let text = embedding_text_for_event(&event).expect("embedding text");

        assert!(text.contains("Screen was captured."));
        assert!(!text.contains("secret visible text"));
    }

    #[test]
    fn stores_and_loads_event_embeddings() {
        let conn = open_memory_database_in_memory().expect("database opens");

        insert_embedding(&conn, "mem_1", "event", DEFAULT_EMBEDDING_MODEL, &[1.0, 0.0])
            .expect("embedding inserts");

        let embeddings = load_event_embeddings(&conn, 10).expect("embeddings load");

        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings[0].target_id, "mem_1");
        assert_eq!(embeddings[0].vector, vec![1.0, 0.0]);
    }

    #[test]
    fn cosine_similarity_scores_matching_vectors() {
        assert!(cosine_similarity(&[1.0, 0.0], &[1.0, 0.0]) > 0.99);
        assert_eq!(cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]), 0.0);
    }

    #[test]
    fn sqlite_vec_extension_is_available() {
        let conn = open_memory_database_in_memory().expect("database opens");
        assert!(sqlite_vec_available(&conn));
    }
}
