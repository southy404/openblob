use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::semantic_memory_path;

pub const CURRENT_SEMANTIC_MEMORY_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    pub version: u32,
    pub preferred_languages: Vec<String>,
    pub favorite_apps: Vec<String>,
    pub recurring_topics: Vec<String>,
    pub inferred_user_style: Option<String>,
    pub notes: Vec<String>,
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self {
            version: CURRENT_SEMANTIC_MEMORY_VERSION,
            preferred_languages: vec!["en".into(), "de".into()],
            favorite_apps: Vec::new(),
            recurring_topics: Vec::new(),
            inferred_user_style: Some("direct".into()),
            notes: Vec::new(),
        }
    }
}

impl SemanticMemory {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_SEMANTIC_MEMORY_VERSION;

        dedup_trimmed_lowercase(&mut self.preferred_languages);
        dedup_trimmed_preserve_case(&mut self.favorite_apps);
        dedup_trimmed_preserve_case(&mut self.recurring_topics);
        dedup_trimmed_preserve_case(&mut self.notes);

        if self.preferred_languages.is_empty() {
            self.preferred_languages = vec!["en".into(), "de".into()];
        }

        if let Some(style) = &self.inferred_user_style {
            let trimmed = style.trim();
            self.inferred_user_style = if trimmed.is_empty() {
                Some("direct".into())
            } else {
                Some(trimmed.to_lowercase())
            };
        } else {
            self.inferred_user_style = Some("direct".into());
        }

        self
    }

    pub fn register_app(&mut self, app_name: &str) {
        let app = app_name.trim();
        if app.is_empty() || app.eq_ignore_ascii_case("unknown") {
            return;
        }

        self.favorite_apps.push(app.to_string());
        dedup_trimmed_preserve_case(&mut self.favorite_apps);
    }

    pub fn register_topic(&mut self, topic: &str) {
        let topic = topic.trim();
        if topic.is_empty() {
            return;
        }

        self.recurring_topics.push(topic.to_string());
        dedup_trimmed_preserve_case(&mut self.recurring_topics);
    }

    pub fn add_note(&mut self, note: &str) {
        let note = note.trim();
        if note.is_empty() {
            return;
        }

        self.notes.push(note.to_string());
        dedup_trimmed_preserve_case(&mut self.notes);
    }
}

pub fn load_semantic_memory() -> Result<SemanticMemory, String> {
    let path = semantic_memory_path()?;
    let memory = load_json_or_default::<SemanticMemory>(&path)?.normalized();
    Ok(memory)
}

pub fn save_semantic_memory(memory: &SemanticMemory) -> Result<(), String> {
    let path = semantic_memory_path()?;
    save_json(&path, &memory.clone().normalized())
}

pub fn load_or_create_semantic_memory() -> Result<SemanticMemory, String> {
    let memory = load_semantic_memory()?;
    save_semantic_memory(&memory)?;
    Ok(memory)
}

fn dedup_trimmed_lowercase(values: &mut Vec<String>) {
    let mut cleaned: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();

    cleaned.sort();
    cleaned.dedup();

    *values = cleaned;
}

fn dedup_trimmed_preserve_case(values: &mut Vec<String>) {
    let mut cleaned: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    cleaned.sort_by_key(|v| v.to_lowercase());
    cleaned.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    *values = cleaned;
}