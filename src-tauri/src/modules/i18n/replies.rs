use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::modules::i18n::command_locale::command_locale;

static DE_REPLIES: OnceLock<HashMap<String, String>> = OnceLock::new();
static EN_REPLIES: OnceLock<HashMap<String, String>> = OnceLock::new();

fn parse_map(raw: &str) -> HashMap<String, String> {
    let value: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let mut out = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, value) in obj {
            if let Some(text) = value.as_str() {
                out.insert(key.clone(), text.to_string());
            }
        }
    }

    out
}

fn de_replies() -> &'static HashMap<String, String> {
    DE_REPLIES.get_or_init(|| {
        parse_map(include_str!("replies/de.json"))
    })
}

fn en_replies() -> &'static HashMap<String, String> {
    EN_REPLIES.get_or_init(|| {
        parse_map(include_str!("replies/en.json"))
    })
}

fn is_german_locale() -> bool {
    let locale = command_locale();

    locale
        .current_time_phrases
        .iter()
        .any(|p| p.contains("wie viel uhr") || p.contains("uhrzeit"))
}

fn active_replies() -> &'static HashMap<String, String> {
    if is_german_locale() {
        de_replies()
    } else {
        en_replies()
    }
}

pub fn reply(key: &str) -> String {
    active_replies()
        .get(key)
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

pub fn reply_with(key: &str, replacements: &[(&str, String)]) -> String {
    let mut text = reply(key);

    for (name, value) in replacements {
        let needle = format!("{{{name}}}");
        text = text.replace(&needle, value);
    }

    text
}