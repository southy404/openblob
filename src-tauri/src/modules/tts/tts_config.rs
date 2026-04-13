use crate::modules::profile::user_profile::load_or_create_user_profile;

#[derive(Debug, Clone)]
pub enum TtsProvider {
    Piper,
    Kokoro,
}

#[derive(Debug, Clone)]
pub struct TtsConfig {
    pub default_lang: String,

    pub de_provider: TtsProvider,
    pub en_provider: TtsProvider,

    pub de_voice: String,
    pub en_voice: String,

    pub piper_exe: String,
    pub piper_models_dir: String,

    pub kokoro_base_url: String,
    pub enabled: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            default_lang: "de".into(),

            de_provider: TtsProvider::Kokoro,
            en_provider: TtsProvider::Kokoro,

            de_voice: "af_heart".into(),
            en_voice: "af_heart".into(),

            piper_exe: "piper".into(),
            piper_models_dir: "models".into(),

            kokoro_base_url: "http://127.0.0.1:8880".into(),
            enabled: true,
        }
    }
}

pub fn detect_lang_from_text(text: &str) -> String {
    let lower = format!(" {} ", text.to_lowercase());

    let german_markers = [
        " ich ", " und ", " oder ", " nicht ", " bitte ", " heute ", " wetter ",
        " danke ", " hallo ", "ü", "ö", "ä", "ß",
    ];

    if german_markers.iter().any(|m| lower.contains(m)) {
        return "de".into();
    }

    "en".into()
}

pub fn preferred_lang() -> String {
    if let Ok(profile) = load_or_create_user_profile() {
        if let Some(first) = profile.languages.first() {
            let lang = first.trim().to_lowercase();
            if lang == "de" || lang.starts_with("de-") {
                return "de".into();
            }
            if lang == "en" || lang.starts_with("en-") {
                return "en".into();
            }
        }
    }

    TtsConfig::default().default_lang
}