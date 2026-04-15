use crate::modules::companion::bonding::{load_bonding_state, save_bonding_state};
use crate::modules::memory::episodic_memory::{append_episode, EpisodicMemoryEntry};
use crate::modules::memory::semantic_memory::{load_or_create_semantic_memory, save_semantic_memory};
use crate::modules::profile::user_profile::{load_or_create_user_profile, save_user_profile};

pub fn summarize_success_reply(reply: &str) -> String {
    let trimmed = reply.trim();

    if trimmed.is_empty() {
        return "Action completed.".into();
    }

    trimmed.chars().take(180).collect()
}

pub fn infer_topic_from_input(input: &str) -> Option<String> {
    let lowered = input.trim().to_lowercase();

    if lowered.is_empty() {
        return None;
    }

    if lowered.contains("youtube") {
        return Some("youtube".into());
    }
    if lowered.contains("netflix") {
        return Some("netflix".into());
    }
    if lowered.contains("spotify") {
        return Some("spotify".into());
    }
    if lowered.contains("weather") || lowered.contains("wetter") {
        return Some("weather".into());
    }
    if lowered.contains("screenshot") || lowered.contains("snip") {
        return Some("screenshot".into());
    }
    if lowered.contains("google") {
        return Some("google".into());
    }
    if lowered.contains("tab") || lowered.contains("browser") || lowered.contains("chrome") {
        return Some("browser".into());
    }

    None
}

pub fn register_successful_interaction(
    input: &str,
    app_name: &str,
    context_domain: &str,
    reply: &str,
) {
    if let Ok(mut bonding) = load_bonding_state() {
        bonding.register_helpful_interaction();
        let _ = save_bonding_state(&bonding);
    }

    if let Ok(mut profile) = load_or_create_user_profile() {
        profile.register_app(app_name);

        if let Some(topic) = infer_topic_from_input(input) {
            profile.register_topic(&topic);
        }

        let _ = save_user_profile(&profile);
    }

    if let Ok(mut semantic_memory) = load_or_create_semantic_memory() {
        semantic_memory.register_app(app_name);

        if let Some(topic) = infer_topic_from_input(input) {
            semantic_memory.register_topic(&topic);
        }

        let _ = save_semantic_memory(&semantic_memory);
    }

    let episode = EpisodicMemoryEntry::new(
        "successful_command",
        app_name,
        context_domain,
        input,
        summarize_success_reply(reply),
        "success",
        0.42,
    );

    let _ = append_episode(&episode);
}