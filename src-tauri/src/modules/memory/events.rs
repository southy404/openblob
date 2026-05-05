use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::modules::profile::companion_config::PrivacyConfig;

pub const CURRENT_MEMORY_EVENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEventKind {
    Command,
    ChatTurn,
    Snip,
    BrowserVisit,
    TranscriptSegment,
    ConnectorMessage,
}

impl MemoryEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::ChatTurn => "chat_turn",
            Self::Snip => "snip",
            Self::BrowserVisit => "browser_visit",
            Self::TranscriptSegment => "transcript_segment",
            Self::ConnectorMessage => "connector_message",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyTier {
    Transient,
    MetadataOnly,
    Redacted,
    Full,
}

impl Default for PrivacyTier {
    fn default() -> Self {
        Self::Redacted
    }
}

impl PrivacyTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::MetadataOnly => "metadata_only",
            Self::Redacted => "redacted",
            Self::Full => "full",
        }
    }

    pub fn should_persist(self) -> bool {
        !matches!(self, Self::Transient)
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "transient" => Self::Transient,
            "metadata_only" => Self::MetadataOnly,
            "full" => Self::Full,
            _ => Self::Redacted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub kind: MemoryEventKind,
    pub source: String,
    pub app_name: Option<String>,
    pub context_domain: Option<String>,
    pub user_input: Option<String>,
    pub summary: Option<String>,
    pub outcome: Option<String>,
    pub importance: f32,
    pub privacy_tier: PrivacyTier,
    pub metadata: Value,
}

impl MemoryEvent {
    pub fn new(
        kind: MemoryEventKind,
        source: impl Into<String>,
        privacy_tier: PrivacyTier,
    ) -> Self {
        let now = Utc::now();

        Self {
            version: CURRENT_MEMORY_EVENT_VERSION,
            id: format!("mem_{}", Uuid::now_v7()),
            timestamp: now.to_rfc3339(),
            kind,
            source: source.into(),
            app_name: None,
            context_domain: None,
            user_input: None,
            summary: None,
            outcome: None,
            importance: 0.5,
            privacy_tier,
            metadata: json!({}),
        }
    }

    pub fn successful_command(
        app_name: impl Into<String>,
        context_domain: impl Into<String>,
        user_input: impl Into<String>,
        summary: impl Into<String>,
        outcome: impl Into<String>,
        privacy: &PrivacyConfig,
    ) -> Self {
        Self::new(
            MemoryEventKind::Command,
            "desktop",
            privacy_tier_for_kind(MemoryEventKind::Command, privacy),
        )
        .with_app_name(app_name)
        .with_context_domain(context_domain)
        .with_user_input(user_input)
        .with_summary(summary)
        .with_outcome(outcome)
        .with_importance(0.42)
    }

    pub fn successful_connector_command(
        channel: impl Into<String>,
        user_input: impl Into<String>,
        summary: impl Into<String>,
        outcome: impl Into<String>,
        privacy: &PrivacyConfig,
    ) -> Self {
        let channel = channel.into();

        Self::new(
            MemoryEventKind::ConnectorMessage,
            channel.clone(),
            privacy_tier_for_kind(MemoryEventKind::ConnectorMessage, privacy),
        )
        .with_app_name(channel)
        .with_context_domain("external")
        .with_user_input(user_input)
        .with_summary(summary)
        .with_outcome(outcome)
        .with_importance(0.6)
    }

    pub fn with_app_name(mut self, value: impl Into<String>) -> Self {
        self.app_name = cleaned_optional(value.into());
        self
    }

    pub fn with_context_domain(mut self, value: impl Into<String>) -> Self {
        self.context_domain = cleaned_optional(value.into());
        self
    }

    pub fn with_user_input(mut self, value: impl Into<String>) -> Self {
        self.user_input = cleaned_optional(value.into());
        self
    }

    pub fn with_summary(mut self, value: impl Into<String>) -> Self {
        self.summary = cleaned_optional(value.into());
        self
    }

    pub fn with_outcome(mut self, value: impl Into<String>) -> Self {
        self.outcome = cleaned_optional(value.into());
        self
    }

    pub fn with_importance(mut self, value: f32) -> Self {
        self.importance = value.clamp(0.0, 1.0);
        self
    }

    pub fn with_metadata(mut self, value: Value) -> Self {
        self.metadata = value;
        self
    }
}

pub fn privacy_tier_for_kind(kind: MemoryEventKind, privacy: &PrivacyConfig) -> PrivacyTier {
    if !privacy.store_episodic_memory {
        return PrivacyTier::Transient;
    }

    match kind {
        MemoryEventKind::Snip | MemoryEventKind::BrowserVisit if !privacy.allow_screen_history => {
            PrivacyTier::MetadataOnly
        }
        MemoryEventKind::TranscriptSegment if !privacy.allow_voice_history => {
            PrivacyTier::MetadataOnly
        }
        _ => PrivacyTier::Redacted,
    }
}

pub fn allows_semantic_extraction(kind: MemoryEventKind, privacy: &PrivacyConfig) -> bool {
    privacy.store_semantic_memory
        && privacy_tier_for_kind(kind, privacy).should_persist()
        && !matches!(
            kind,
            MemoryEventKind::Snip | MemoryEventKind::TranscriptSegment
        )
}

fn cleaned_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_event_uses_redacted_privacy_by_default() {
        let privacy = PrivacyConfig::default();
        let event = MemoryEvent::successful_command(
            "Spotify",
            "desktop",
            "open spotify",
            "Opened Spotify.",
            "success",
            &privacy,
        );

        assert_eq!(event.version, CURRENT_MEMORY_EVENT_VERSION);
        assert_eq!(event.kind, MemoryEventKind::Command);
        assert_eq!(event.source, "desktop");
        assert_eq!(event.app_name.as_deref(), Some("Spotify"));
        assert_eq!(event.context_domain.as_deref(), Some("desktop"));
        assert_eq!(event.user_input.as_deref(), Some("open spotify"));
        assert_eq!(event.summary.as_deref(), Some("Opened Spotify."));
        assert_eq!(event.outcome.as_deref(), Some("success"));
        assert_eq!(event.privacy_tier, PrivacyTier::Redacted);
        assert!(event.id.starts_with("mem_"));
    }

    #[test]
    fn privacy_config_can_disable_persisted_events() {
        let privacy = PrivacyConfig {
            store_episodic_memory: false,
            ..PrivacyConfig::default()
        };

        assert_eq!(
            privacy_tier_for_kind(MemoryEventKind::Command, &privacy),
            PrivacyTier::Transient
        );
    }

    #[test]
    fn screen_and_voice_history_use_metadata_only_when_disabled() {
        let privacy = PrivacyConfig::default();

        assert_eq!(
            privacy_tier_for_kind(MemoryEventKind::Snip, &privacy),
            PrivacyTier::MetadataOnly
        );
        assert_eq!(
            privacy_tier_for_kind(MemoryEventKind::TranscriptSegment, &privacy),
            PrivacyTier::MetadataOnly
        );
    }

    #[test]
    fn semantic_extraction_respects_semantic_privacy_flag() {
        let privacy = PrivacyConfig {
            store_semantic_memory: false,
            ..PrivacyConfig::default()
        };

        assert!(!allows_semantic_extraction(
            MemoryEventKind::Command,
            &privacy
        ));
    }

    #[test]
    fn connector_command_records_channel_as_source_and_app() {
        let event = MemoryEvent::successful_connector_command(
            "telegram",
            "open spotify",
            "Opened Spotify.",
            "success",
            &PrivacyConfig::default(),
        );

        assert_eq!(event.kind, MemoryEventKind::ConnectorMessage);
        assert_eq!(event.source, "telegram");
        assert_eq!(event.app_name.as_deref(), Some("telegram"));
        assert_eq!(event.context_domain.as_deref(), Some("external"));
        assert_eq!(event.importance, 0.6);
    }
}
