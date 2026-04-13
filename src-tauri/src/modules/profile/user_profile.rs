use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::user_profile_path;

pub const CURRENT_USER_PROFILE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub version: u32,
    pub display_name: Option<String>,
    pub languages: Vec<String>,
    pub preferred_response_style: Option<String>,

    pub email_address: Option<String>,
    pub github_url: Option<String>,
    pub discord_url: Option<String>,
    pub signature: Option<String>,

    pub favorite_apps: Vec<String>,
    pub recurring_topics: Vec<String>,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            version: CURRENT_USER_PROFILE_VERSION,
            display_name: None,
            languages: vec!["en".into(), "de".into()],
            preferred_response_style: Some("balanced".into()),

            email_address: None,
            github_url: None,
            discord_url: None,
            signature: None,

            favorite_apps: Vec::new(),
            recurring_topics: Vec::new(),
        }
    }
}

impl UserProfile {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_USER_PROFILE_VERSION;

        self.languages = self
            .languages
            .iter()
            .map(|lang| normalize_lang(lang))
            .collect();
        self.languages.sort();
        self.languages.dedup();

        if self.languages.is_empty() {
            self.languages = vec!["en".into(), "de".into()];
        }

        if let Some(name) = &self.display_name {
            let trimmed = name.trim();
            self.display_name = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(value) = &self.email_address {
            let trimmed = value.trim();
            self.email_address = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(value) = &self.github_url {
            let trimmed = value.trim();
            self.github_url = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(value) = &self.discord_url {
            let trimmed = value.trim();
            self.discord_url = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(value) = &self.signature {
            let trimmed = value.trim();
            self.signature = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }

        if let Some(style) = &self.preferred_response_style {
            let trimmed = style.trim();
            self.preferred_response_style = if trimmed.is_empty() {
                Some("balanced".into())
            } else {
                Some(trimmed.to_lowercase())
            };
        } else {
            self.preferred_response_style = Some("balanced".into());
        }

        dedup_trimmed_lowercase(&mut self.favorite_apps);
        dedup_trimmed_lowercase(&mut self.recurring_topics);

        self
    }

    pub fn register_app(&mut self, app_name: &str) {
        let app = app_name.trim();
        if app.is_empty() || app.eq_ignore_ascii_case("unknown") {
            return;
        }

        self.favorite_apps.push(app.to_string());
        dedup_trimmed_lowercase(&mut self.favorite_apps);
    }

    pub fn register_topic(&mut self, topic: &str) {
        let topic = topic.trim();
        if topic.is_empty() {
            return;
        }

        self.recurring_topics.push(topic.to_string());
        dedup_trimmed_lowercase(&mut self.recurring_topics);
    }
}

pub fn load_user_profile() -> Result<UserProfile, String> {
    let path = user_profile_path()?;
    let profile = load_json_or_default::<UserProfile>(&path)?.normalized();
    Ok(profile)
}

pub fn save_user_profile(profile: &UserProfile) -> Result<(), String> {
    let path = user_profile_path()?;
    save_json(&path, &profile.clone().normalized())
}

pub fn load_or_create_user_profile() -> Result<UserProfile, String> {
    let profile = load_user_profile()?;
    save_user_profile(&profile)?;
    Ok(profile)
}

fn normalize_lang(input: &str) -> String {
    let lower = input.trim().to_lowercase();

    match lower.as_str() {
        "en-us" | "en-gb" | "english" => "en".into(),
        "de-de" | "german" | "deutsch" => "de".into(),
        "en" | "de" => lower,
        _ if lower.starts_with("en") => "en".into(),
        _ if lower.starts_with("de") => "de".into(),
        _ => "en".into(),
    }
}

fn dedup_trimmed_lowercase(values: &mut Vec<String>) {
    let mut cleaned: Vec<String> = values
        .iter()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect();

    cleaned.sort_by_key(|v| v.to_lowercase());
    cleaned.dedup_by(|a, b| a.eq_ignore_ascii_case(b));

    *values = cleaned;
}