use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::append_jsonl;
use crate::modules::storage::paths::episodic_memory_path;

pub const CURRENT_EPISODIC_MEMORY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemoryEntry {
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub kind: String,
    pub app_name: String,
    pub context_domain: String,
    pub user_input: String,
    pub summary: String,
    pub outcome: String,
    pub importance: f32,
}

impl EpisodicMemoryEntry {
    pub fn new(
        kind: impl Into<String>,
        app_name: impl Into<String>,
        context_domain: impl Into<String>,
        user_input: impl Into<String>,
        summary: impl Into<String>,
        outcome: impl Into<String>,
        importance: f32,
    ) -> Self {
        let now = Utc::now();
        let ts = now.to_rfc3339();

        Self {
            version: CURRENT_EPISODIC_MEMORY_VERSION,
            id: format!("ep_{}", now.timestamp_millis()),
            timestamp: ts,
            kind: kind.into(),
            app_name: app_name.into(),
            context_domain: context_domain.into(),
            user_input: user_input.into(),
            summary: summary.into(),
            outcome: outcome.into(),
            importance: importance.clamp(0.0, 1.0),
        }
    }
}

pub fn append_episode(entry: &EpisodicMemoryEntry) -> Result<(), String> {
    let path = episodic_memory_path()?;
    append_jsonl(&path, entry)
}