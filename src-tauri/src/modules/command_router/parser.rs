use super::extract::*;
use super::intents::best_intent;
use super::media::parse_media_command;
use super::normalize::{normalize, tokens};
use super::utilities::parse_utility_command;
use super::types::{CompanionAction, IntentKind};
use crate::modules::i18n::command_locale::command_locale;
use strsim::jaro_winkler;

fn matches_any(input: &str, phrases: &[String]) -> bool {
    phrases.iter().any(|p| input.contains(p))
}

fn equals_any(input: &str, phrases: &[String]) -> bool {
    phrases.iter().any(|p| input.trim() == p.trim())
}

fn fuzzy_has_any_strings(tokens: &[&str], words: &[String], threshold: f32) -> bool {
    tokens.iter().any(|t| {
        words.iter().any(|w| jaro_winkler(t, w) >= threshold as f64)
    })
}

fn contains_locale_words(normalized: &str, words: &[String], threshold: f32) -> bool {
    let toks = tokens(normalized);
    fuzzy_has_any_strings(&toks, words, threshold)
}

fn strip_first_prefix_str(input: &str, prefixes: &[String]) -> String {
    let trimmed = input.trim();

    for prefix in prefixes {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim().to_string();
        }
    }

    trimmed.to_string()
}

fn remove_words(mut text: String, words: &[String]) -> String {
    for word in words {
        text = text.replace(word, "");
    }
    text
}

fn trim_leading_search_fillers(mut text: String) -> String {
    loop {
        let trimmed = text.trim().to_string();

        let next = if let Some(rest) = trimmed.strip_prefix("nach ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("for ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("auf ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("on ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("in ") {
            rest.trim().to_string()
        } else if let Some(rest) = trimmed.strip_prefix("im ") {
            rest.trim().to_string()
        } else {
            break;
        };

        if next == trimmed {
            break;
        }

        text = next;
    }

    text.trim().to_string()
}

fn trim_search_service_suffix(mut text: String, service_words: &[String]) -> String {
    let original = text.clone();

    for service_word in service_words {
        for suffix in [
            format!(" auf {service_word}"),
            format!(" on {service_word}"),
            format!(" in {service_word}"),
            format!(" im {service_word}"),
        ] {
            if text.ends_with(&suffix) {
                text = text[..text.len() - suffix.len()].trim().to_string();
                break;
            }
        }
    }

    for dangling in [" auf", " on", " in", " im", " nach", " for"] {
        if text.ends_with(dangling) {
            text = text[..text.len() - dangling.len()].trim().to_string();
            break;
        }
    }

    text = trim_leading_search_fillers(text);

    if text.is_empty() {
        original.trim().to_string()
    } else {
        text
    }
}

fn parse_explicit_search_command(normalized: &str) -> Option<CompanionAction> {
    let locale = command_locale();

    let has_google = contains_locale_words(normalized, &locale.google_words, 0.86);
    let has_youtube = contains_locale_words(normalized, &locale.youtube_words, 0.86);
    let has_search = contains_locale_words(normalized, &locale.search_words, 0.86)
        || contains_locale_words(normalized, &locale.find_words, 0.86);

    if has_google && has_search {
        let mut prefixes = Vec::new();

        for search_word in &locale.search_words {
            for google_word in &locale.google_words {
                prefixes.push(format!("{search_word} {google_word} "));
                prefixes.push(format!("{google_word} {search_word} "));
                prefixes.push(format!("{search_word} on {google_word} "));
                prefixes.push(format!("{search_word} auf {google_word} "));
                prefixes.push(format!("{search_word} auf {google_word} nach "));
                prefixes.push(format!("{search_word} on {google_word} for "));
                prefixes.push(format!("{search_word} {google_word} nach "));
                prefixes.push(format!("{search_word} {google_word} for "));
            }
        }

        let query = trim_search_service_suffix(
            strip_first_prefix_str(normalized, &prefixes),
            &locale.google_words,
        );

        if !query.is_empty() && query != normalized {
            return Some(CompanionAction::GoogleSearch { query });
        }
    }

    if has_youtube && has_search {
        let mut prefixes = Vec::new();

    for search_word in &locale.search_words {
        for youtube_word in &locale.youtube_words {
            prefixes.push(format!("{search_word} {youtube_word} "));
            prefixes.push(format!("{youtube_word} {search_word} "));
            prefixes.push(format!("{search_word} on {youtube_word} "));
            prefixes.push(format!("{search_word} auf {youtube_word} "));
            prefixes.push(format!("{search_word} auf {youtube_word} nach "));
            prefixes.push(format!("{search_word} on {youtube_word} for "));
            prefixes.push(format!("{search_word} {youtube_word} nach "));
            prefixes.push(format!("{search_word} {youtube_word} for "));
        }
    }

        let query = trim_search_service_suffix(
            strip_first_prefix_str(normalized, &prefixes),
            &locale.youtube_words,
        );

        if !query.is_empty() && query != normalized {
            return Some(CompanionAction::YouTubeSearch { query });
        }
    }

    None
}

fn parse_explicit_browser_window_command(normalized: &str) -> Option<CompanionAction> {
    let locale = command_locale();

    let has_open = contains_locale_words(normalized, &locale.open_words, 0.86);
    let has_close = contains_locale_words(normalized, &locale.close_words, 0.86);

    let has_tab = contains_locale_words(normalized, &locale.tab_words, 0.82)
        || contains_locale_words(normalized, &locale.tab_new_words, 0.82)
        || contains_locale_words(normalized, &locale.tab_close_words, 0.82);

    let has_window = contains_locale_words(normalized, &locale.window_words, 0.82)
        || contains_locale_words(normalized, &locale.window_new_words, 0.82);

    if has_open && has_tab {
        return Some(CompanionAction::NewTab);
    }

    if has_close && has_tab {
        return Some(CompanionAction::CloseTab);
    }

    if has_open && has_window {
        return Some(CompanionAction::NewWindow);
    }

    if has_close && has_window {
        return Some(CompanionAction::Close);
    }

    None
}

pub fn parse_voice_command_with_context(
    input: &str,
    app_name: &str,
    window_title: &str,
    domain: &str,
) -> CompanionAction {
    let normalized = normalize(input);
    let locale = command_locale();

    if let Some(action) = parse_utility_command(&normalized) {
        return action;
    }

    if equals_any(&normalized, &locale.current_time_phrases) {
        return CompanionAction::CurrentTime;
    }

    if equals_any(&normalized, &locale.current_date_phrases) {
        return CompanionAction::CurrentDate;
    }

    if matches_any(&normalized, &locale.coin_flip_phrases) {
        return CompanionAction::CoinFlip;
    }

    if matches_any(&normalized, &locale.roll_dice_phrases) {
        return CompanionAction::RollDice;
    }

    if matches_any(&normalized, &locale.timer_phrases) {
        let seconds = extract_timer_seconds(&normalized).unwrap_or(5 * 60);
        return CompanionAction::SetTimer { seconds };
    }

    if matches_any(&normalized, &locale.screenshot_words) {
        return CompanionAction::TakeScreenshot;
    }

    let app = app_name.to_lowercase();
    let title = window_title.to_lowercase();
    let dom = domain.to_lowercase();

    let on_youtube = app.contains("youtube") || title.contains("youtube");

    let in_browser = dom == "browser"
        || app.contains("chrome")
        || app.contains("edge")
        || app.contains("firefox")
        || app.contains("brave")
        || title.contains("google")
        || title.contains("youtube");

    let parsed = parse_voice_command(input);

    if !matches!(parsed, CompanionAction::None) {
        return parsed;
    }

    if on_youtube {
        let has_play = contains_locale_words(&normalized, &locale.play_words, 0.84);
        let has_pause = contains_locale_words(&normalized, &locale.pause_words, 0.84);

        if has_play {
            return CompanionAction::YouTubePlay;
        }

        if has_pause {
            return CompanionAction::YouTubePause;
        }

        let has_skip = contains_locale_words(&normalized, &locale.skip_words, 0.84);
        let has_ad = contains_locale_words(&normalized, &locale.ad_words, 0.84);

        if has_skip && has_ad {
            return CompanionAction::YouTubeSkipAd;
        }

        let has_first = contains_locale_words(&normalized, &locale.first_words, 0.84);
        let has_video = contains_locale_words(&normalized, &locale.video_words, 0.84);

        if has_first && has_video {
            return CompanionAction::BrowserClickFirstResult;
        }

        if let Some(query) = extract_generic_search_query(input) {
            let cleaned = trim_search_service_suffix(query, &locale.youtube_words);
            if !cleaned.trim().is_empty() {
                return CompanionAction::YouTubeSearch { query: cleaned };
            }
        }

        if let Some(text) = extract_quoted_text(input) {
            if !text.trim().is_empty() {
                return CompanionAction::BrowserClickBestMatch {
                    text: text.trim().to_string(),
                };
            }
        }
    }

    if let Some(query) = extract_generic_search_query(input) {
        let cleaned = trim_search_service_suffix(query, &locale.google_words);
        if !cleaned.trim().is_empty() {
            return CompanionAction::GoogleSearch { query: cleaned };
        }
    }

    CompanionAction::None
}

pub fn parse_voice_command(input: &str) -> CompanionAction {
    let normalized = normalize(input);
    let locale = command_locale();

    if normalized.is_empty() {
        return CompanionAction::None;
    }

    if let Some(action) = parse_explicit_search_command(&normalized) {
        return action;
    }

    if let Some(action) = parse_explicit_browser_window_command(&normalized) {
        return action;
    }

    match normalized.trim() {
        "insert my email" | "paste my email" => {
            return CompanionAction::InsertSnippet {
                key: "email".to_string(),
            };
        }
        "insert my github" | "paste my github" | "insert github" | "paste github" => {
            return CompanionAction::InsertSnippet {
                key: "github".to_string(),
            };
        }
        "insert my discord" | "paste my discord" | "insert discord" | "paste discord" => {
            return CompanionAction::InsertSnippet {
                key: "discord".to_string(),
            };
        }
        "insert my signature" | "paste my signature" | "insert signature" | "paste signature" => {
            return CompanionAction::InsertSnippet {
                key: "signature".to_string(),
            };
        }
        _ => {}
    }

    if let Some(action) = parse_utility_command(&normalized) {
        return action;
    }

    if let Some(action) = parse_media_command(&normalized) {
        return action;
    }

    let toks = tokens(&normalized);

    match best_intent(&normalized, &toks) {
        IntentKind::SetVolume => extract_percent(&normalized)
            .map(|percent| CompanionAction::SetVolume { percent })
            .unwrap_or(CompanionAction::None),

        IntentKind::VolumeUp => CompanionAction::VolumeUp,
        IntentKind::VolumeDown => CompanionAction::VolumeDown,
        IntentKind::Mute => CompanionAction::Mute,
        IntentKind::Unmute => CompanionAction::Unmute,
        IntentKind::MediaPlayPause => CompanionAction::MediaPlayPause,
        IntentKind::MediaNext => CompanionAction::MediaNext,
        IntentKind::MediaPrev => CompanionAction::MediaPrev,

        IntentKind::GoogleSearch => {
            let query = trim_search_service_suffix(
                extract_search_query(
                    &normalized,
                    &toks,
                    &["google", "googel", "gogle", "search", "suche", "such", "find"],
                ),
                &locale.google_words,
            );

            if query.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::GoogleSearch { query }
            }
        },

        IntentKind::YouTubeSearch => {
            let query = trim_search_service_suffix(
                extract_search_query(
                    &normalized,
                    &toks,
                    &["youtube", "youtub", "jutube", "jutub", "yt", "search", "suche", "such", "find"],
                ),
                &locale.youtube_words,
            );

            if query.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::YouTubeSearch { query }
            }
        },

        IntentKind::YouTubePlayTitle => {
            let mut title = extract_quoted_text(input).unwrap_or_else(|| normalized.clone());

            title = remove_words(title, &locale.play_words);
            title = remove_words(title, &locale.youtube_words);
            title = title.replace("video", "").trim().to_string();

            if title.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::YouTubePlayTitle { title }
            }
        }

        IntentKind::OpenApp => {
            let (target, prefer_browser) = extract_open_target(&normalized, &toks);
            CompanionAction::OpenApp { target, prefer_browser }
        }

        IntentKind::Save => CompanionAction::Save,
        IntentKind::SaveAs => CompanionAction::SaveAs,
        IntentKind::OpenFile => CompanionAction::OpenFile,
        IntentKind::NewFile => CompanionAction::NewFile,
        IntentKind::Close => CompanionAction::Close,
        IntentKind::NewTab => CompanionAction::NewTab,
        IntentKind::CloseTab => CompanionAction::CloseTab,
        IntentKind::CloseTabByIndex => CompanionAction::CloseTabByIndex {
            index: extract_number(&toks).unwrap_or(1).saturating_sub(1),
        },
        IntentKind::NewWindow => CompanionAction::NewWindow,
        IntentKind::Incognito => CompanionAction::Incognito,
        IntentKind::Reload => CompanionAction::Reload,
        IntentKind::Undo => CompanionAction::Undo,
        IntentKind::Redo => CompanionAction::Redo,

        IntentKind::BrowserBack => CompanionAction::BrowserBack,
        IntentKind::BrowserForward => CompanionAction::BrowserForward,
        IntentKind::BrowserScrollDown => CompanionAction::BrowserScrollDown,
        IntentKind::BrowserScrollUp => CompanionAction::BrowserScrollUp,

        IntentKind::BrowserTypeText => {
            let text = extract_quoted_text(input).unwrap_or_else(|| {
                extract_after_command(&normalized, &["tippe ", "type ", "schreibe ", "enter text "])
                    .unwrap_or_default()
            });

            if text.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::BrowserTypeText { text }
            }
        }

        IntentKind::BrowserSubmit => CompanionAction::BrowserSubmit,

        IntentKind::BrowserClickBestMatch => {
            let mut text = extract_quoted_text(input).unwrap_or_else(|| normalized.clone());

            text = remove_words(text, &locale.click_words);
            text = remove_words(text, &locale.open_words);
            text = text.replace("link", "");
            text = text.replace("button", "");
            text = text.trim().to_string();

            if text.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::BrowserClickBestMatch { text }
            }
        }

        IntentKind::BrowserContext => CompanionAction::BrowserContext,

        IntentKind::BrowserClickLinkByText => {
            let mut text = extract_quoted_text(input).unwrap_or_else(|| normalized.clone());

            text = remove_words(text, &locale.click_words);
            text = remove_words(text, &locale.link_words);
            text = text.trim().to_string();

            if text.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::BrowserClickLinkByText {
                    text,
                    new_tab: wants_new_tab(&normalized, &toks),
                }
            }
        }

        IntentKind::BrowserClickFirstResult => CompanionAction::BrowserClickFirstResult,

        IntentKind::BrowserClickNthResult => CompanionAction::BrowserClickNthResult {
            index: extract_number(&toks).unwrap_or(1).saturating_sub(1),
        },

        IntentKind::BrowserOpenUrl => {
            let mut raw = extract_quoted_text(input).unwrap_or_else(|| normalized.clone());

            raw = remove_words(raw, &locale.open_words);
            raw = remove_words(raw, &locale.go_to_words);
            raw = remove_words(raw, &locale.navigate_to_words);
            raw = raw.trim().to_string();

            if raw.is_empty() {
                CompanionAction::None
            } else {
                let url = if raw.starts_with("http://") || raw.starts_with("https://") {
                    raw
                } else if raw.contains('.') && !raw.contains(' ') {
                    format!("https://{}", raw)
                } else {
                    format!("https://www.google.com/search?q={}", urlencoding::encode(&raw))
                };

                CompanionAction::BrowserOpenUrl {
                    url,
                    new_tab: wants_new_tab(&normalized, &toks),
                    new_window: wants_new_window(&normalized, &toks),
                    incognito: wants_incognito(&normalized, &toks),
                }
            }
        }

        IntentKind::YouTubeNextVideo => CompanionAction::YouTubeNextVideo,
        IntentKind::YouTubeSeekForward => CompanionAction::YouTubeSeekForward,
        IntentKind::YouTubeSeekBackward => CompanionAction::YouTubeSeekBackward,

        IntentKind::CurrentTime => CompanionAction::CurrentTime,
        IntentKind::CurrentDate => CompanionAction::CurrentDate,

        IntentKind::WeatherToday => CompanionAction::WeatherToday {
            location: extract_weather_location(&normalized),
        },

        IntentKind::ExplainSelection => CompanionAction::ExplainSelection,
        IntentKind::TakeScreenshot => CompanionAction::TakeScreenshot,
        IntentKind::CoinFlip => CompanionAction::CoinFlip,
        IntentKind::RollDice => CompanionAction::RollDice,
        IntentKind::CancelTimer => CompanionAction::CancelTimer,
        IntentKind::SetTimer { seconds } => CompanionAction::SetTimer { seconds },
        IntentKind::None => CompanionAction::None,
    }
}