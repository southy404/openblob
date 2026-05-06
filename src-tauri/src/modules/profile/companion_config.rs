use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::companion_config_path;

pub const CURRENT_COMPANION_CONFIG_VERSION: u32 = 1;
pub const DEFAULT_PRIMARY_LANGUAGE: &str = "en";
pub const SUPPORTED_LANGUAGES: &[&str] = &["en", "de"];
pub const DEFAULT_CHAT_MODEL: &str = "llama3.1:8b";
pub const DEFAULT_EMBEDDING_MODEL: &str = "nomic-embed-text";
pub const DEFAULT_WAKE_WORD_PHRASE: &str = "hey blob";
pub const DEFAULT_WAKE_WORD_PROVIDER: &str = "none";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    pub theme: String,
    pub style_variant: String,
    pub face_variant: String,
    pub accent_color: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "glass-blue".into(),
            style_variant: "classic".into(),
            face_variant: "soft".into(),
            accent_color: "#75A3FF".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub proactive_level: f32,
    pub expressiveness: f32,
    pub playfulness: f32,
    pub english_first: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            proactive_level: 0.35,
            expressiveness: 0.70,
            playfulness: 0.50,
            english_first: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub store_episodic_memory: bool,
    pub store_semantic_memory: bool,
    pub allow_screen_history: bool,
    pub allow_voice_history: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            store_episodic_memory: true,
            store_semantic_memory: true,
            allow_screen_history: false,
            allow_voice_history: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_backend")]
    pub backend: String,
    #[serde(default = "default_memory_prompt_enabled")]
    pub prompt_context_enabled: bool,
    #[serde(default = "default_memory_context_limit")]
    pub prompt_context_limit: usize,
    #[serde(default = "default_memory_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_summary_retention_days")]
    pub summary_retention_days: u32,
    #[serde(default = "default_decay_half_life_days")]
    pub decay_half_life_days: u32,
    #[serde(default = "default_max_events")]
    pub max_events: u32,
    #[serde(default = "default_vector_backend")]
    pub vector_backend: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_memory_backend(),
            prompt_context_enabled: default_memory_prompt_enabled(),
            prompt_context_limit: default_memory_context_limit(),
            retention_days: default_memory_retention_days(),
            summary_retention_days: default_summary_retention_days(),
            decay_half_life_days: default_decay_half_life_days(),
            max_events: default_max_events(),
            vector_backend: default_vector_backend(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionConfig {
    pub version: u32,
    pub blob_name: String,
    pub preferred_language: String,
    pub fallback_language: String,
    pub supported_languages: Vec<String>,
    #[serde(default = "default_chat_model")]
    pub chat_model: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    pub voice_enabled: bool,
    pub voice_id: String,
    #[serde(default)]
    pub wake_word_enabled: bool,
    #[serde(default = "default_wake_word_phrase")]
    pub wake_word_phrase: String,
    #[serde(default = "default_wake_word_sensitivity")]
    pub wake_word_sensitivity: f32,
    #[serde(default = "default_wake_word_provider")]
    pub wake_word_provider: String,
    #[serde(default)]
    pub wake_word_model_path: Option<String>,
    #[serde(default)]
    pub wake_word_auto_listen_enabled: bool,
    pub appearance: AppearanceConfig,
    pub behavior: BehaviorConfig,
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_COMPANION_CONFIG_VERSION,
            blob_name: "OpenBlob".into(),
            preferred_language: DEFAULT_PRIMARY_LANGUAGE.into(),
            fallback_language: "de".into(),
            supported_languages: SUPPORTED_LANGUAGES.iter().map(|v| v.to_string()).collect(),
            chat_model: default_chat_model(),
            embedding_model: default_embedding_model(),
            voice_enabled: true,
            voice_id: "default".into(),
            wake_word_enabled: false,
            wake_word_phrase: default_wake_word_phrase(),
            wake_word_sensitivity: default_wake_word_sensitivity(),
            wake_word_provider: default_wake_word_provider(),
            wake_word_model_path: None,
            wake_word_auto_listen_enabled: false,
            appearance: AppearanceConfig::default(),
            behavior: BehaviorConfig::default(),
            privacy: PrivacyConfig::default(),
            memory: MemoryConfig::default(),
        }
    }
}

impl CompanionConfig {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_COMPANION_CONFIG_VERSION;

        self.preferred_language = normalize_lang(&self.preferred_language);
        self.fallback_language = normalize_lang(&self.fallback_language);

        if self.supported_languages.is_empty() {
            self.supported_languages = SUPPORTED_LANGUAGES.iter().map(|v| v.to_string()).collect();
        } else {
            self.supported_languages = self
                .supported_languages
                .iter()
                .map(|lang| normalize_lang(lang))
                .collect();
            self.supported_languages.sort();
            self.supported_languages.dedup();
        }

        clamp_unit(&mut self.behavior.proactive_level);
        clamp_unit(&mut self.behavior.expressiveness);
        clamp_unit(&mut self.behavior.playfulness);
        self.chat_model = normalize_model_name(&self.chat_model, DEFAULT_CHAT_MODEL);
        self.embedding_model = normalize_model_name(&self.embedding_model, DEFAULT_EMBEDDING_MODEL);
        self.memory.backend = normalize_memory_backend(&self.memory.backend);
        self.memory.prompt_context_limit = self.memory.prompt_context_limit.clamp(1, 50);
        self.memory.retention_days = self.memory.retention_days.clamp(1, 3650);
        self.memory.summary_retention_days = self.memory.summary_retention_days.clamp(1, 3650);
        self.memory.decay_half_life_days = self.memory.decay_half_life_days.clamp(1, 3650);
        self.memory.max_events = self.memory.max_events.clamp(100, 1_000_000);
        self.memory.vector_backend = normalize_vector_backend(&self.memory.vector_backend);

        if self.blob_name.trim().is_empty() {
            self.blob_name = "OpenBlob".into();
        }

        if self.voice_id.trim().is_empty() {
            self.voice_id = "default".into();
        }

        self.wake_word_phrase =
            normalize_wake_word_phrase(&self.wake_word_phrase, DEFAULT_WAKE_WORD_PHRASE);
        self.wake_word_sensitivity = self.wake_word_sensitivity.clamp(0.0, 1.0);
        self.wake_word_provider = normalize_wake_word_provider(&self.wake_word_provider);
        self.wake_word_model_path = normalize_optional_path(self.wake_word_model_path);

        self
    }

    pub fn supports_language(&self, lang: &str) -> bool {
        let lang = normalize_lang(lang);
        self.supported_languages.iter().any(|v| v == &lang)
    }
}

pub fn load_companion_config() -> Result<CompanionConfig, String> {
    let path = companion_config_path()?;
    let config = load_json_or_default::<CompanionConfig>(&path)?.normalized();
    Ok(config)
}

pub fn save_companion_config(config: &CompanionConfig) -> Result<(), String> {
    let path = companion_config_path()?;
    save_json(&path, &config.clone().normalized())
}

pub fn load_or_create_companion_config() -> Result<CompanionConfig, String> {
    let config = load_companion_config()?;
    save_companion_config(&config)?;
    Ok(config)
}

pub fn normalize_lang(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    match lower.as_str() {
        "en-us" | "en-gb" | "english" => "en".into(),
        "de-de" | "german" | "deutsch" => "de".into(),
        "en" | "de" => lower,
        _ if lower.starts_with("en") => "en".into(),
        _ if lower.starts_with("de") => "de".into(),
        _ => DEFAULT_PRIMARY_LANGUAGE.into(),
    }
}

fn clamp_unit(value: &mut f32) {
    *value = value.clamp(0.0, 1.0);
}

fn default_memory_backend() -> String {
    "dual_write".into()
}

fn default_chat_model() -> String {
    DEFAULT_CHAT_MODEL.into()
}

fn default_embedding_model() -> String {
    DEFAULT_EMBEDDING_MODEL.into()
}

pub fn default_wake_word_phrase() -> String {
    DEFAULT_WAKE_WORD_PHRASE.into()
}

pub fn default_wake_word_sensitivity() -> f32 {
    0.5
}

pub fn default_wake_word_provider() -> String {
    DEFAULT_WAKE_WORD_PROVIDER.into()
}

fn default_memory_prompt_enabled() -> bool {
    true
}

fn default_memory_context_limit() -> usize {
    12
}

fn default_memory_retention_days() -> u32 {
    365
}

fn default_summary_retention_days() -> u32 {
    730
}

fn default_decay_half_life_days() -> u32 {
    30
}

fn default_max_events() -> u32 {
    50_000
}

fn default_vector_backend() -> String {
    "sqlite_vec".into()
}

fn normalize_memory_backend(input: &str) -> String {
    match input.trim().to_lowercase().as_str() {
        "legacy" => "legacy".into(),
        "sqlite" => "sqlite".into(),
        _ => "dual_write".into(),
    }
}

fn normalize_vector_backend(input: &str) -> String {
    match input.trim().to_lowercase().as_str() {
        "json" | "json_fallback" => "json".into(),
        _ => "sqlite_vec".into(),
    }
}

fn normalize_model_name(input: &str, fallback: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        fallback.into()
    } else {
        trimmed.into()
    }
}

fn normalize_wake_word_phrase(input: &str, fallback: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        fallback.into()
    } else {
        trimmed.into()
    }
}

pub fn normalize_wake_word_provider(input: &str) -> String {
    match input.trim().to_lowercase().as_str() {
        "local-openwakeword" | "local_openwakeword" | "openwakeword" | "open-wakeword"
        | "porcupine" => "local-openwakeword".into(),
        "local-wakeword" | "local_wakeword" | "local" | "wakeword" => "local-wakeword".into(),
        "mic-test" | "mictest" | "mic_test" | "dev-mic-test" => "mic-test".into(),
        "mock" => "mock".into(),
        "disabled" => "disabled".into(),
        _ => DEFAULT_WAKE_WORD_PROVIDER.into(),
    }
}

fn normalize_optional_path(input: Option<String>) -> Option<String> {
    input.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}
