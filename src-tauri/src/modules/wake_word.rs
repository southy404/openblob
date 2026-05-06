use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock};

use crate::modules::profile::companion_config::{
    default_wake_word_phrase, default_wake_word_provider, default_wake_word_sensitivity,
    load_or_create_companion_config, normalize_wake_word_provider, save_companion_config,
};

const LOG_PREFIX: &str = "[openblob:wake-word]";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordSettings {
    #[serde(default)]
    pub wake_word_enabled: bool,
    #[serde(default = "default_wake_word_phrase")]
    pub wake_word_phrase: String,
    #[serde(default = "default_wake_word_sensitivity")]
    pub wake_word_sensitivity: f32,
    #[serde(default = "default_wake_word_provider")]
    pub wake_word_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordStatus {
    pub status: String,
    pub message: String,
    pub listening: bool,
    pub detected: bool,
    pub provider_configured: bool,
}

static WAKE_WORD_STATUS: OnceLock<Mutex<WakeWordStatus>> = OnceLock::new();

fn status_store() -> &'static Mutex<WakeWordStatus> {
    WAKE_WORD_STATUS.get_or_init(|| Mutex::new(disabled_status("Wake word is disabled.")))
}

fn log(message: impl AsRef<str>) {
    println!("{LOG_PREFIX} {}", message.as_ref());
}

fn normalize_settings(mut settings: WakeWordSettings) -> WakeWordSettings {
    settings.wake_word_phrase = if settings.wake_word_phrase.trim().is_empty() {
        default_wake_word_phrase()
    } else {
        settings.wake_word_phrase.trim().to_string()
    };
    settings.wake_word_sensitivity = settings.wake_word_sensitivity.clamp(0.0, 1.0);
    settings.wake_word_provider = normalize_wake_word_provider(&settings.wake_word_provider);
    settings
}

fn settings_from_config() -> Result<WakeWordSettings, String> {
    let config = load_or_create_companion_config()?;
    Ok(normalize_settings(WakeWordSettings {
        wake_word_enabled: config.wake_word_enabled,
        wake_word_phrase: config.wake_word_phrase,
        wake_word_sensitivity: config.wake_word_sensitivity,
        wake_word_provider: config.wake_word_provider,
    }))
}

fn disabled_status(message: impl Into<String>) -> WakeWordStatus {
    WakeWordStatus {
        status: "disabled".into(),
        message: message.into(),
        listening: false,
        detected: false,
        provider_configured: false,
    }
}

fn error_status(message: impl Into<String>, provider_configured: bool) -> WakeWordStatus {
    WakeWordStatus {
        status: "error".into(),
        message: message.into(),
        listening: false,
        detected: false,
        provider_configured,
    }
}

fn listening_status(message: impl Into<String>) -> WakeWordStatus {
    WakeWordStatus {
        status: "listening".into(),
        message: message.into(),
        listening: true,
        detected: false,
        provider_configured: true,
    }
}

fn provider_config_message(settings: &WakeWordSettings) -> Option<&'static str> {
    match settings.wake_word_provider.as_str() {
        "none" => Some("Wake word provider not configured yet."),
        "porcupine" if !porcupine_configured() => Some("Wake word provider not configured yet."),
        "porcupine" => None,
        _ => Some("Wake word provider not configured yet."),
    }
}

fn porcupine_configured() -> bool {
    let has_access_key = std::env::var("PICOVOICE_ACCESS_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_keyword_path = std::env::var("OPENBLOB_PORCUPINE_KEYWORD_PATH")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    has_access_key && has_keyword_path
}

fn set_status(status: WakeWordStatus) -> Result<WakeWordStatus, String> {
    let mut guard = status_store()
        .lock()
        .map_err(|_| "Wake word status lock is poisoned".to_string())?;
    *guard = status.clone();
    Ok(status)
}

fn status_for_settings(settings: &WakeWordSettings) -> WakeWordStatus {
    if !settings.wake_word_enabled {
        return disabled_status("Wake word is disabled.");
    }

    if let Some(message) = provider_config_message(settings) {
        return error_status(message, false);
    }

    error_status(
        "Porcupine wake word listener is not implemented in this build yet.",
        true,
    )
}

#[tauri::command]
pub fn get_wake_word_settings() -> Result<WakeWordSettings, String> {
    settings_from_config()
}

#[tauri::command]
pub fn update_wake_word_settings(settings: WakeWordSettings) -> Result<WakeWordSettings, String> {
    let settings = normalize_settings(settings);
    let mut config = load_or_create_companion_config()?;

    config.wake_word_enabled = settings.wake_word_enabled;
    config.wake_word_phrase = settings.wake_word_phrase.clone();
    config.wake_word_sensitivity = settings.wake_word_sensitivity;
    config.wake_word_provider = settings.wake_word_provider.clone();
    save_companion_config(&config)?;

    log(format!(
        "settings updated; enabled={}, provider={}, sensitivity={:.2}",
        settings.wake_word_enabled, settings.wake_word_provider, settings.wake_word_sensitivity
    ));

    let _ = set_status(status_for_settings(&settings));
    Ok(settings)
}

#[tauri::command]
pub fn start_wake_word_listener() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;

    if !settings.wake_word_enabled {
        log("start skipped; wake word is disabled");
        return set_status(disabled_status("Wake word is disabled."));
    }

    if let Some(message) = provider_config_message(&settings) {
        log(format!(
            "start skipped; provider={} is not configured",
            settings.wake_word_provider
        ));
        return set_status(error_status(message, false));
    }

    log("start requested; Porcupine placeholder is stable but not recording yet");
    set_status(listening_status(
        "Wake word listener placeholder is ready for local Porcupine integration.",
    ))
}

#[tauri::command]
pub fn stop_wake_word_listener() -> Result<WakeWordStatus, String> {
    log("listener stopped");
    set_status(disabled_status("Wake word listener stopped."))
}

#[tauri::command]
pub fn get_wake_word_status() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;
    let current = status_store()
        .lock()
        .map_err(|_| "Wake word status lock is poisoned".to_string())?
        .clone();

    if !settings.wake_word_enabled {
        return Ok(disabled_status("Wake word is disabled."));
    }

    if current.listening || current.detected {
        return Ok(current);
    }

    Ok(status_for_settings(&settings))
}
