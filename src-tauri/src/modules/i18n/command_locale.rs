use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Deserialize)]
pub struct CommandLocale {
    pub open_words: Vec<String>,
    pub close_words: Vec<String>,
    pub browser_words: Vec<String>,
    pub google_words: Vec<String>,
    pub youtube_words: Vec<String>,
    pub weather_words: Vec<String>,
    pub time_words: Vec<String>,
    pub date_words: Vec<String>,
    pub explain_words: Vec<String>,
    pub volume_up_words: Vec<String>,
    pub volume_down_words: Vec<String>,
    pub volume_words: Vec<String>,
    pub mute_words: Vec<String>,
    pub unmute_words: Vec<String>,
    pub pause_words: Vec<String>,
    pub next_words: Vec<String>,
    pub prev_words: Vec<String>,
    pub save_words: Vec<String>,
    pub save_as_words: Vec<String>,
    pub open_file_words: Vec<String>,
    pub new_file_words: Vec<String>,
    pub undo_words: Vec<String>,
    pub redo_words: Vec<String>,
    pub tab_close_words: Vec<String>,
    pub tab_new_words: Vec<String>,
    pub window_new_words: Vec<String>,
    pub incognito_words: Vec<String>,
    pub reload_words: Vec<String>,
    pub yt_next_words: Vec<String>,
    pub yt_forward_words: Vec<String>,
    pub yt_back_words: Vec<String>,
    pub click_words: Vec<String>,
    pub play_words: Vec<String>,
    pub result_words: Vec<String>,
    pub back_words: Vec<String>,
    pub forward_words: Vec<String>,
    pub scroll_down_words: Vec<String>,
    pub scroll_up_words: Vec<String>,
    pub type_words: Vec<String>,
    pub submit_words: Vec<String>,
    pub context_words: Vec<String>,
    pub screenshot_words: Vec<String>,

    pub streaming_followup_confirm: Vec<String>,
    pub streaming_more_words: Vec<String>,

    pub known_targets: HashMap<String, Vec<String>>,
    pub streaming_service_aliases: HashMap<String, Vec<String>>,

    pub current_time_phrases: Vec<String>,
    pub current_date_phrases: Vec<String>,
    pub coin_flip_phrases: Vec<String>,
    pub roll_dice_phrases: Vec<String>,
    pub timer_phrases: Vec<String>,
    pub timer_cancel_phrases: Vec<String>,

    pub search_words: Vec<String>,
    pub find_words: Vec<String>,
    pub tab_words: Vec<String>,
    pub window_words: Vec<String>,
    pub skip_words: Vec<String>,
    pub ad_words: Vec<String>,
    pub first_words: Vec<String>,
    pub video_words: Vec<String>,
    pub link_words: Vec<String>,
    pub button_words: Vec<String>,
    pub go_to_words: Vec<String>,
    pub navigate_to_words: Vec<String>,
}

static ACTIVE_LOCALE: OnceLock<CommandLocale> = OnceLock::new();

fn command_locale_path(lang: &str) -> PathBuf {
    PathBuf::from("src").join("i18n").join("commands").join(format!("{lang}.json"))
}

pub fn load_command_locale(lang: &str) -> Result<CommandLocale, String> {
    let path = command_locale_path(lang);
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read locale '{}': {}", path.display(), e))?;

    serde_json::from_str::<CommandLocale>(&raw)
        .map_err(|e| format!("Failed to parse locale '{}': {}", path.display(), e))
}

pub fn init_command_locale(lang: &str) -> Result<(), String> {
    let locale = load_command_locale(lang)
        .or_else(|_| load_command_locale("en"))?;

    let _ = ACTIVE_LOCALE.set(locale);
    Ok(())
}

pub fn command_locale() -> &'static CommandLocale {
    ACTIVE_LOCALE.get().expect("command locale not initialized")
}