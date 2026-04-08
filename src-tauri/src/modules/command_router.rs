use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum CompanionAction {
    VolumeUp,
    VolumeDown,
    SetVolume { percent: u8 },
    Mute,
    Unmute,
    ToggleMute,
    MediaPlayPause,
    MediaNext,
    MediaPrev,

    GoogleSearch { query: String },
    YouTubeSearch { query: String },
    YouTubePlayTitle { title: String },

    OpenApp { target: String, prefer_browser: bool },

    Save,
    SaveAs,
    OpenFile,
    NewFile,
    Close,

    NewTab,
    CloseTab,
    CloseTabByIndex { index: usize },
    NewWindow,
    Incognito,
    Reload,

    BrowserOpenUrl {
        url: String,
        new_tab: bool,
        new_window: bool,
        incognito: bool,
    },
    BrowserClickLinkByText {
        text: String,
        new_tab: bool,
    },
    BrowserClickFirstResult,
    BrowserClickNthResult { index: usize },
    BrowserBack,
    BrowserForward,
    BrowserScrollDown,
    BrowserScrollUp,
    BrowserTypeText { text: String },
    BrowserSubmit,
    BrowserClickBestMatch { text: String },
    BrowserContext,

    InsertText(String),
    KeyCombo(Vec<&'static str>),
    KeyPress(&'static str),

    Confirm,
    Clear,
    CloseApp,
    Undo,
    Redo,
    YouTubeNextVideo,
    YouTubeSeekForward,
    YouTubeSeekBackward,
    WeatherToday { location: Option<String> },
    ExplainSelection,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntentKind {
    VolumeUp,
    VolumeDown,
    SetVolume,
    Mute,
    Unmute,
    MediaPlayPause,
    MediaNext,
    MediaPrev,

    GoogleSearch,
    YouTubeSearch,
    YouTubePlayTitle,

    OpenApp,

    Save,
    SaveAs,
    OpenFile,
    NewFile,
    Close,

    NewTab,
    CloseTab,
    CloseTabByIndex,
    NewWindow,
    Incognito,
    Reload,

    BrowserOpenUrl,
    BrowserClickLinkByText,
    BrowserClickFirstResult,
    BrowserClickNthResult,
    BrowserBack,
    BrowserForward,
    BrowserScrollDown,
    BrowserScrollUp,
    BrowserTypeText,
    BrowserSubmit,
    BrowserClickBestMatch,
    BrowserContext,

    Undo,
    Redo,
    YouTubeNextVideo,
    YouTubeSeekForward,
    YouTubeSeekBackward,
    WeatherToday,
    ExplainSelection,
    None,
}

#[derive(Debug)]
struct IntentScore {
    kind: IntentKind,
    score: f32,
}

const OPEN_WORDS: &[&str] = &["open", "oeffne", "oeffnen", "starte", "start", "launch", "run"];
const CLOSE_WORDS: &[&str] = &["close", "schliess", "schliesse", "beenden", "exit", "quit"];
const BROWSER_WORDS: &[&str] = &["browser", "web", "website", "chrome", "edge", "online"];
const GOOGLE_WORDS: &[&str] = &["google", "googel", "gogle"];
const YOUTUBE_WORDS: &[&str] = &["youtube", "youtub", "jutube", "jutub", "yt"];
const WEATHER_WORDS: &[&str] = &[
    "wetter", "weather", "temperatur", "temperature", "regen", "rain", "sun", "sonne", "forecast",
];
const EXPLAIN_WORDS: &[&str] = &["erklaer", "erklaere", "explain", "meaning", "bedeutet", "mean"];
const VOLUME_UP_WORDS: &[&str] = &["lauter", "louder", "increase", "up", "hoch"];
const VOLUME_DOWN_WORDS: &[&str] = &["leiser", "quieter", "down", "lower", "runter", "reduce"];
const VOLUME_WORDS: &[&str] = &["lautstaerke", "volume", "sound", "ton", "audio"];
const MUTE_WORDS: &[&str] = &["mute", "stumm", "silent", "silence", "aus"];
const UNMUTE_WORDS: &[&str] = &["unmute", "an", "wieder", "restore"];
const PAUSE_WORDS: &[&str] = &["pause", "stop", "pausieren", "hold"];
const NEXT_WORDS: &[&str] = &["next", "naechster", "weiter", "skip"];
const PREV_WORDS: &[&str] = &["previous", "prev", "zurueck", "back", "vorheriger"];
const SAVE_WORDS: &[&str] = &["save", "speichern"];
const SAVE_AS_WORDS: &[&str] = &["saveas", "save as", "speichern unter"];
const OPEN_FILE_WORDS: &[&str] = &["open file", "datei oeffnen", "file open"];
const NEW_FILE_WORDS: &[&str] = &["new file", "neu", "new", "neue datei"];
const UNDO_WORDS: &[&str] = &["undo", "rueckgaengig", "zuruecknehmen"];
const REDO_WORDS: &[&str] = &["redo", "wiederholen"];
const TAB_CLOSE_WORDS: &[&str] = &["close tab", "tab schliessen", "tab close"];
const TAB_NEW_WORDS: &[&str] = &["new tab", "neuer tab"];
const WINDOW_NEW_WORDS: &[&str] = &["new window", "neues fenster"];
const INCOGNITO_WORDS: &[&str] = &["incognito", "inkognito", "private window", "privates fenster"];
const RELOAD_WORDS: &[&str] = &["reload", "neu laden", "refresh"];
const YT_NEXT_WORDS: &[&str] = &["next video", "naechstes video", "video weiter"];
const YT_FORWARD_WORDS: &[&str] = &["vorspulen", "forward", "skip ahead"];
const YT_BACK_WORDS: &[&str] = &["zurueckspulen", "rewind", "backward"];
const CLICK_WORDS: &[&str] = &["click", "klick", "klicke", "oeffne link", "open link"];
const PLAY_WORDS: &[&str] = &["play", "spiele", "abspielen"];
const RESULT_WORDS: &[&str] = &["result", "ergebnis", "suchergebnis", "video"];
const BACK_WORDS: &[&str] = &["zurueck", "go back", "back", "eine seite zurueck"];
const FORWARD_WORDS: &[&str] = &["forward", "weiter", "vor", "go forward"];
const SCROLL_DOWN_WORDS: &[&str] = &["scroll runter", "runter scrollen", "scroll down"];
const SCROLL_UP_WORDS: &[&str] = &["scroll hoch", "hoch scrollen", "scroll up"];
const TYPE_WORDS: &[&str] = &["tippe", "type", "schreibe", "enter text"];
const SUBMIT_WORDS: &[&str] = &["submit", "abschicken", "absenden", "drueck enter"];
const CONTEXT_WORDS: &[&str] = &["wo bin ich", "seitenkontext", "browser context", "was ist auf der seite"];

const KNOWN_TARGETS: &[(&str, &[&str])] = &[
    ("discord", &["discord", "discrod", "discort", "disord"]),
    ("spotify", &["spotify", "spotfy", "spoti"]),
    ("youtube", &["youtube", "youtub", "jutube", "yt"]),
    ("google", &["google", "googel", "gogle"]),
    ("chrome", &["chrome", "chrom"]),
    ("edge", &["edge", "msedge", "microsoftedge"]),
    ("paint", &["paint", "mspaint"]),
    ("notepad", &["notepad", "editor", "texteditor"]),
    ("explorer", &["explorer", "fileexplorer", "dateiexplorer"]),
    ("calc", &["calc", "calculator", "rechner", "taschenrechner"]),
    ("taskmgr", &["taskmanager", "taskmgr"]),
    ("settings", &["settings", "einstellungen"]),
    ("gmail", &["gmail", "googlemail", "mail"]),
];

fn normalize(input: &str) -> String {
    let lower = input.trim().to_lowercase();
    let replaced = lower
        .replace('ä', "ae")
        .replace('ö', "oe")
        .replace('ü', "ue")
        .replace('ß', "ss");

    let mut out = String::with_capacity(replaced.len());
    let mut prev_space = false;

    for ch in replaced.chars() {
        if ch.is_ascii_alphanumeric() || ch == ' ' {
            if ch == ' ' {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            } else {
                out.push(ch);
                prev_space = false;
            }
        } else if !prev_space {
            out.push(' ');
            prev_space = true;
        }
    }

    out.trim().to_string()
}

fn tokens(text: &str) -> Vec<&str> {
    text.split_whitespace().filter(|t| !t.is_empty()).collect()
}

fn best_similarity(token: &str, words: &[&str]) -> f32 {
    words
        .iter()
        .map(|w| jaro_winkler(token, w) as f32)
        .fold(0.0_f32, f32::max)
}

fn fuzzy_has_any(tokens: &[&str], words: &[&str], threshold: f32) -> bool {
    tokens.iter().any(|t| best_similarity(t, words) >= threshold)
}

fn fuzzy_count(tokens: &[&str], words: &[&str], threshold: f32) -> usize {
    tokens.iter().filter(|t| best_similarity(t, words) >= threshold).count()
}

fn extract_percent(normalized: &str) -> Option<u8> {
    for token in normalized.split_whitespace() {
        let cleaned = token.replace('%', "").replace("prozent", "").replace("percent", "");
        if let Ok(value) = cleaned.parse::<u8>() {
            return Some(value.min(100));
        }
    }
    None
}

fn detect_known_target(tokens: &[&str]) -> Option<String> {
    let mut best: Option<(&str, f32)> = None;

    for token in tokens {
        for (canonical, aliases) in KNOWN_TARGETS {
            let score = best_similarity(token, aliases);
            if score >= 0.88 {
                match best {
                    Some((_, current_best)) if current_best >= score => {}
                    _ => best = Some((canonical, score)),
                }
            }
        }
    }

    best.map(|(name, _)| name.to_string())
}

fn extract_after_prefixes(normalized: &str, prefixes: &[&str]) -> Option<String> {
    for prefix in prefixes {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn extract_quoted_text(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut start = None;

    for (i, b) in bytes.iter().enumerate() {
        if *b == b'"' {
            if let Some(s) = start {
                if i > s + 1 {
                    return Some(input[s + 1..i].to_string());
                }
                start = None;
            } else {
                start = Some(i);
            }
        }
    }

    None
}

fn extract_number(tokens: &[&str]) -> Option<usize> {
    for token in tokens {
        if let Ok(n) = token.parse::<usize>() {
            if n > 0 {
                return Some(n);
            }
        }
    }
    None
}

fn extract_search_query(normalized: &str, toks: &[&str], remove_aliases: &[&str]) -> String {
    if let Some(rest) = extract_after_prefixes(
        normalized,
        &[
            "suche auf google ",
            "google ",
            "search google ",
            "search for ",
            "suche auf youtube ",
            "youtube ",
            "search youtube ",
        ],
    ) {
        return rest;
    }

    let filtered: Vec<&str> = toks
        .iter()
        .copied()
        .filter(|token| !remove_aliases.iter().any(|alias| jaro_winkler(token, alias) >= 0.88))
        .collect();

    let joined = filtered.join(" ").trim().to_string();
    if joined.is_empty() {
        normalized.to_string()
    } else {
        joined
    }
}

fn extract_open_target(normalized: &str, toks: &[&str]) -> (String, bool) {
    let prefer_browser = fuzzy_has_any(toks, BROWSER_WORDS, 0.86);

    if let Some(target) = extract_after_prefixes(
        normalized,
        &["oeffne ", "oeffne mal ", "starte ", "start ", "open ", "launch ", "run "],
    ) {
        let cleaned = target
            .replace(" im browser", "")
            .replace(" in browser", "")
            .replace(" im web", "")
            .replace(" in web", "")
            .replace(" in chrome", "")
            .replace(" mit chrome", "")
            .trim()
            .to_string();

        if let Some(canonical) = detect_known_target(&tokens(&cleaned)) {
            return (canonical, prefer_browser);
        }

        return (cleaned, prefer_browser);
    }

    if let Some(canonical) = detect_known_target(toks) {
        return (canonical, prefer_browser);
    }

    (normalized.to_string(), prefer_browser)
}

fn extract_weather_location(normalized: &str) -> Option<String> {
    for prefix in [" in ", " for ", " fuer ", " für "] {
        if let Some(pos) = normalized.find(prefix) {
            let part = normalized[pos + prefix.len()..].trim();
            if !part.is_empty() {
                return Some(part.to_string());
            }
        }
    }
    None
}

fn score(tokens: &[&str], words: &[&str], threshold: f32, weight: f32) -> f32 {
    fuzzy_count(tokens, words, threshold) as f32 * weight
}

fn wants_new_tab(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("new tab")
        || normalized.contains("neuer tab")
        || fuzzy_has_any(toks, TAB_NEW_WORDS, 0.82)
}

fn wants_new_window(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("new window")
        || normalized.contains("neues fenster")
        || fuzzy_has_any(toks, WINDOW_NEW_WORDS, 0.82)
}

fn wants_incognito(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("inkognito")
        || normalized.contains("incognito")
        || fuzzy_has_any(toks, INCOGNITO_WORDS, 0.82)
}

fn extract_after_command(normalized: &str, commands: &[&str]) -> Option<String> {
    for cmd in commands {
        if let Some(rest) = normalized.strip_prefix(cmd) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn best_intent(normalized: &str, toks: &[&str]) -> IntentKind {
    if extract_percent(normalized).is_some() && fuzzy_has_any(toks, VOLUME_WORDS, 0.82) {
        return IntentKind::SetVolume;
    }

    let mut scores = vec![
        IntentScore { kind: IntentKind::VolumeUp, score: score(toks, VOLUME_UP_WORDS, 0.84, 1.8) + score(toks, VOLUME_WORDS, 0.84, 0.8) },
        IntentScore { kind: IntentKind::VolumeDown, score: score(toks, VOLUME_DOWN_WORDS, 0.84, 1.8) + score(toks, VOLUME_WORDS, 0.84, 0.8) },
        IntentScore { kind: IntentKind::Mute, score: score(toks, MUTE_WORDS, 0.84, 1.7) + score(toks, VOLUME_WORDS, 0.82, 0.5) },
        IntentScore { kind: IntentKind::Unmute, score: score(toks, UNMUTE_WORDS, 0.84, 1.7) + score(toks, VOLUME_WORDS, 0.82, 0.4) },
        IntentScore { kind: IntentKind::MediaPlayPause, score: score(toks, PAUSE_WORDS, 0.84, 1.8) + score(toks, &["music", "musik", "song", "track", "video"], 0.84, 0.6) },
        IntentScore { kind: IntentKind::MediaNext, score: score(toks, NEXT_WORDS, 0.84, 1.8) + score(toks, &["music", "musik", "song", "track"], 0.84, 0.5) },
        IntentScore { kind: IntentKind::MediaPrev, score: score(toks, PREV_WORDS, 0.84, 1.8) + score(toks, &["music", "musik", "song", "track"], 0.84, 0.5) },
        IntentScore { kind: IntentKind::GoogleSearch, score: score(toks, GOOGLE_WORDS, 0.88, 2.0) + score(toks, &["search", "suche", "such", "find"], 0.86, 1.0) },
        IntentScore { kind: IntentKind::YouTubeSearch, score: score(toks, YOUTUBE_WORDS, 0.86, 2.0) + score(toks, &["search", "suche", "such", "find"], 0.86, 0.9) },
        IntentScore { kind: IntentKind::YouTubePlayTitle, score: score(toks, PLAY_WORDS, 0.84, 1.7) + score(toks, YOUTUBE_WORDS, 0.84, 0.6) + score(toks, RESULT_WORDS, 0.84, 0.3) },
        IntentScore {
            kind: IntentKind::OpenApp,
            score:
                score(toks, OPEN_WORDS, 0.88, 1.8)
                + if detect_known_target(toks).is_some() { 1.5 } else { 0.0 }
                + if extract_after_prefixes(
                    normalized,
                    &["oeffne ", "oeffne mal ", "starte ", "start ", "open ", "launch ", "run "]
                ).is_some() { 1.2 } else { 0.0 }
                + score(toks, BROWSER_WORDS, 0.88, 0.5),
        },
        IntentScore { kind: IntentKind::Close, score: score(toks, CLOSE_WORDS, 0.86, 2.0) },
        IntentScore { kind: IntentKind::SaveAs, score: score(toks, SAVE_AS_WORDS, 0.82, 2.4) + if normalized.contains("save as") || normalized.contains("speichern unter") { 1.0 } else { 0.0 } },
        IntentScore { kind: IntentKind::Save, score: score(toks, SAVE_WORDS, 0.88, 2.0) },
        IntentScore { kind: IntentKind::OpenFile, score: score(toks, OPEN_FILE_WORDS, 0.82, 1.8) + if normalized == "oeffnen" || normalized == "open" { 0.8 } else { 0.0 } },
        IntentScore { kind: IntentKind::NewFile, score: score(toks, NEW_FILE_WORDS, 0.82, 1.8) },
        IntentScore { kind: IntentKind::Undo, score: score(toks, UNDO_WORDS, 0.86, 2.0) },
        IntentScore { kind: IntentKind::Redo, score: score(toks, REDO_WORDS, 0.86, 2.0) },
        IntentScore { kind: IntentKind::CloseTab, score: score(toks, TAB_CLOSE_WORDS, 0.82, 2.3) + if normalized.contains("close tab") || normalized.contains("tab schliessen") { 0.8 } else { 0.0 } },
        IntentScore { kind: IntentKind::CloseTabByIndex, score: if normalized.contains("tab") && extract_number(toks).is_some() && fuzzy_has_any(toks, CLOSE_WORDS, 0.84) { 2.6 } else { 0.0 } },
        IntentScore { kind: IntentKind::NewTab, score: score(toks, TAB_NEW_WORDS, 0.82, 2.3) + if normalized.contains("new tab") || normalized.contains("neuer tab") { 0.8 } else { 0.0 } },
        IntentScore { kind: IntentKind::NewWindow, score: score(toks, WINDOW_NEW_WORDS, 0.82, 2.3) + if normalized.contains("new window") || normalized.contains("neues fenster") { 0.8 } else { 0.0 } },
        IntentScore { kind: IntentKind::Incognito, score: score(toks, INCOGNITO_WORDS, 0.82, 2.3) },
        IntentScore { kind: IntentKind::Reload, score: score(toks, RELOAD_WORDS, 0.82, 2.0) },
        IntentScore { kind: IntentKind::BrowserClickLinkByText, score: score(toks, CLICK_WORDS, 0.82, 2.2) },
        IntentScore { kind: IntentKind::BrowserClickFirstResult, score: if normalized.contains("erstes ergebnis") || normalized.contains("first result") { 2.5 } else { 0.0 } },
        IntentScore { kind: IntentKind::BrowserClickNthResult, score: if normalized.contains("ergebnis") && extract_number(toks).is_some() { 2.4 } else { 0.0 } },
        IntentScore { kind: IntentKind::BrowserBack, score: score(toks, BACK_WORDS, 0.82, 2.2) },
        IntentScore { kind: IntentKind::BrowserForward, score: score(toks, FORWARD_WORDS, 0.82, 2.2) },
        IntentScore { kind: IntentKind::BrowserScrollDown, score: score(toks, SCROLL_DOWN_WORDS, 0.80, 2.2) },
        IntentScore { kind: IntentKind::BrowserScrollUp, score: score(toks, SCROLL_UP_WORDS, 0.80, 2.2) },
        IntentScore { kind: IntentKind::BrowserTypeText, score: score(toks, TYPE_WORDS, 0.82, 2.1) },
        IntentScore { kind: IntentKind::BrowserSubmit, score: score(toks, SUBMIT_WORDS, 0.82, 2.2) },
        IntentScore { kind: IntentKind::BrowserClickBestMatch, score: score(toks, CLICK_WORDS, 0.82, 1.8) + score(toks, RESULT_WORDS, 0.80, 0.3) },
        IntentScore { kind: IntentKind::BrowserContext, score: score(toks, CONTEXT_WORDS, 0.80, 2.3) },
        IntentScore { kind: IntentKind::YouTubeNextVideo, score: score(toks, YT_NEXT_WORDS, 0.82, 2.2) + if normalized.contains("next video") || normalized.contains("naechstes video") { 0.7 } else { 0.0 } },
        IntentScore { kind: IntentKind::YouTubeSeekForward, score: score(toks, YT_FORWARD_WORDS, 0.84, 2.0) },
        IntentScore { kind: IntentKind::YouTubeSeekBackward, score: score(toks, YT_BACK_WORDS, 0.84, 2.0) },
        IntentScore { kind: IntentKind::WeatherToday, score: score(toks, WEATHER_WORDS, 0.84, 2.0) + score(toks, &["today", "heute", "now", "jetzt"], 0.88, 0.7) },
        IntentScore { kind: IntentKind::ExplainSelection, score: score(toks, EXPLAIN_WORDS, 0.84, 1.8) + score(toks, &["text", "this", "that", "das"], 0.84, 0.5) },
    ];

    scores.sort_by(|a, b| b.score.total_cmp(&a.score));
    let best = scores.first().map(|s| (s.kind, s.score)).unwrap_or((IntentKind::None, 0.0));

    if best.1 >= 1.8 { best.0 } else { IntentKind::None }
}

pub fn parse_voice_command(input: &str) -> CompanionAction {
    let normalized = normalize(input);
    let toks = tokens(&normalized);

    if normalized.is_empty() {
        return CompanionAction::None;
    }

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
        

        IntentKind::GoogleSearch => CompanionAction::GoogleSearch {
            query: extract_search_query(&normalized, &toks, &["google", "googel", "gogle", "search", "suche", "such", "find"]),
        },

        IntentKind::YouTubeSearch => CompanionAction::YouTubeSearch {
            query: extract_search_query(&normalized, &toks, &["youtube", "youtub", "jutube", "jutub", "yt", "search", "suche", "such", "find"]),
        },

        IntentKind::YouTubePlayTitle => {
            let title = extract_quoted_text(input).unwrap_or_else(|| {
                normalized
                    .replace("spiele", "")
                    .replace("play", "")
                    .replace("abspielen", "")
                    .replace("video", "")
                    .replace("youtube", "")
                    .trim()
                    .to_string()
            });

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
            let text = extract_quoted_text(input).unwrap_or_else(|| {
                normalized
                    .replace("klicke", "")
                    .replace("click", "")
                    .replace("oeffne", "")
                    .replace("open", "")
                    .replace("link", "")
                    .replace("button", "")
                    .trim()
                    .to_string()
            });

            if text.is_empty() {
                CompanionAction::None
            } else {
                CompanionAction::BrowserClickBestMatch { text }
            }
        }

        IntentKind::BrowserContext => CompanionAction::BrowserContext,
        IntentKind::BrowserClickLinkByText => {
            let text = extract_quoted_text(input).unwrap_or_else(|| {
                normalized
                    .replace("klicke", "")
                    .replace("click", "")
                    .replace("open link", "")
                    .replace("oeffne link", "")
                    .replace("link", "")
                    .trim()
                    .to_string()
            });

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
            let raw = extract_quoted_text(input).unwrap_or_else(|| {
                normalized
                    .replace("oeffne ", "")
                    .replace("open ", "")
                    .replace("go to ", "")
                    .replace("navigiere zu ", "")
                    .trim()
                    .to_string()
            });

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

        IntentKind::WeatherToday => CompanionAction::WeatherToday {
            location: extract_weather_location(&normalized),
        },

        IntentKind::ExplainSelection => CompanionAction::ExplainSelection,
        IntentKind::None => CompanionAction::None,
    }
}