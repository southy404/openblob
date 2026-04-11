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

    StreamOpenTitle {
        service: String,
        title: String,
        autoplay: bool,
    },
    StreamRecommend {
        service: Option<String>,
        mood: Option<String>,
        genre: Option<String>,
        kind: Option<String>,
        trending: bool,
    },
    StreamOpenLastSuggestion,
    StreamMoreLikeLast,
    StreamCapability {
        service: Option<String>,
    },

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
    BrowserClickButtonByText {
        text: String,
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
    YouTubePlay,
    YouTubePause,
    YouTubeSkipAd,

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
    CurrentTime,
    CurrentDate,
    WeatherToday { location: Option<String> },
    ExplainSelection,
    TakeScreenshot,
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
    CurrentTime,
    CurrentDate,
    WeatherToday,
    ExplainSelection,
    TakeScreenshot,
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
    "wetter",
    "weather",
    "temperatur",
    "temperature",
    "regen",
    "rain",
    "sun",
    "sonne",
    "forecast",
];
const TIME_WORDS: &[&str] = &[
    "uhr",
    "uhrzeit",
    "spaet",
    "spät",
    "zeit",
    "time",
];

const DATE_WORDS: &[&str] = &[
    "datum",
    "date",
    "heute",
    "tag",
    "today",
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
const SCREENSHOT_WORDS: &[&str] = &[
    "screenshot",
    "screen shot",
    "snip",
    "snapshot",
    "capture",
    "screen capture",
    "take screenshot",
    "take a screenshot",
    "make screenshot",
    "mach screenshot",
    "mach einen screenshot",
    "mach ein screenshot",
    "mach einen screen",
    "bildschirmfoto",
    "bildschirm foto",
    "aufnahme",
    "screenie",
    "snipping",
];
const KNOWN_TARGETS: &[(&str, &[&str])] = &[
    ("discord", &["discord", "discrod", "discort", "disord"]),
    ("spotify", &["spotify", "spotfy", "spoti"]),
    ("youtube", &["youtube", "youtub", "jutube", "yt"]),
    ("google", &["google", "googel", "gogle"]),
    ("chrome", &["chrome", "chrom"]),
    ("edge", &["edge", "msedge", "microsoftedge"]),
    ("twitch", &["twitch", "twuicth", "twich", "twtich"]),
    ("github", &["github", "git hub"]),
    ("reddit", &["reddit", "redit"]),
    ("paint", &["paint", "mspaint"]),
    ("notepad", &["notepad", "editor", "texteditor"]),
    ("explorer", &["explorer", "fileexplorer", "dateiexplorer"]),
    ("calc", &["calc", "calculator", "rechner", "taschenrechner"]),
    ("taskmgr", &["taskmanager", "taskmgr"]),
    ("settings", &["settings", "einstellungen"]),
    ("gmail", &["gmail", "googlemail", "mail"]),
    ("steam", &["steam", "steem", "steeeam", "stim"]),
    ("fl studio", &["fl", "flstudio", "fl studio"]),
];

const STREAMING_SERVICE_ALIASES: &[(&str, &[&str])] = &[
    ("netflix", &["netflix", "netflx", "netfliks"]),
    ("youtube", &["youtube", "yt", "youtub", "jutube"]),
    ("prime", &["prime", "prime video", "amazon prime"]),
    ("disney", &["disney", "disney plus", "disneyplus"]),
    ("twitch", &["twitch", "twuicth", "twich"]),
    ("spotify", &["spotify", "spotfy"]),
];

const STREAMING_FOLLOWUP_CONFIRM: &[&str] = &[
    "yes",
    "yeah",
    "yep",
    "ja",
    "mach",
    "do it",
    "open it",
    "launch it",
    "play it",
    "open that",
    "launch that",
    "yes open it",
    "yes launch it",
    "yes play it",
];

const STREAMING_MORE_WORDS: &[&str] = &[
    "something else",
    "another one",
    "more like this",
    "more like that",
    "was anderes",
    "noch was",
    "gib mir was anderes",
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

fn contains_any_phrase(normalized: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| normalized.contains(p))
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

fn detect_streaming_service(normalized: &str, toks: &[&str]) -> Option<String> {
    for (canonical, aliases) in STREAMING_SERVICE_ALIASES {
        if aliases.iter().any(|a| normalized.contains(a)) {
            return Some((*canonical).to_string());
        }

        for token in toks {
            if best_similarity(token, aliases) >= 0.88 {
                return Some((*canonical).to_string());
            }
        }
    }

    None
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

fn extract_generic_search_query(input: &str) -> Option<String> {
    let normalized = normalize(input);

    for prefix in [
        "suche nach ",
        "suche ",
        "search for ",
        "search ",
        "finde ",
        "find ",
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let q = rest.trim();
            if !q.is_empty() {
                return Some(q.to_string());
            }
        }
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

    match normalized.trim() {
        "wie viel uhr ist es"
        | "wie spaet ist es"
        | "wie spät ist es"
        | "uhrzeit"
        | "what time is it"
        | "current time" => {
            return CompanionAction::CurrentTime;
        }

        "welcher tag ist heute"
        | "welches datum haben wir"
        | "welches datum ist heute"
        | "heutiges datum"
        | "what date is it"
        | "current date" => {
            return CompanionAction::CurrentDate;
        }

        _ => {}
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

    if on_youtube {
        match normalized.trim() {
            "play" | "spiele" | "video starten" | "start video" => {
                return CompanionAction::YouTubePlay;
            }
            "pause" | "pausiere" | "video pausieren" => {
                return CompanionAction::YouTubePause;
            }
            "skip"
            | "skip ad"
            | "skip ads"
            | "ueberspringen"
            | "überspringen"
            | "werbung ueberspringen"
            | "werbung überspringen" => {
                return CompanionAction::YouTubeSkipAd;
            }
            "klick erstes video" | "click first video" => {
                return CompanionAction::BrowserClickFirstResult;
            }
            _ => {}
        }

        if let Some(query) = extract_generic_search_query(input) {
            return CompanionAction::YouTubeSearch { query };
        }

        if let Some(text) = extract_quoted_text(input) {
            if !text.trim().is_empty() {
                return CompanionAction::BrowserClickBestMatch {
                    text: text.trim().to_string(),
                };
            }
        }
    }

    if in_browser {
        if let Some(query) = extract_generic_search_query(input) {
            return CompanionAction::GoogleSearch { query };
        }
    }

    let parsed = parse_voice_command(input);

    if !matches!(parsed, CompanionAction::None) {
        return parsed;
    }

    CompanionAction::None
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

fn is_exact_open_known_site(normalized: &str) -> Option<&'static str> {
    match normalized.trim() {
        "open youtube" | "oeffne youtube" | "start youtube" | "launch youtube" | "run youtube" => {
            Some("https://www.youtube.com")
        }
        "open netflix" | "oeffne netflix" | "start netflix" | "launch netflix" | "run netflix" => {
            Some("https://www.netflix.com")
        }
        "open spotify" | "oeffne spotify" | "start spotify" | "launch spotify" | "run spotify" => {
            Some("https://open.spotify.com")
        }
        "open twitch" | "oeffne twitch" | "start twitch" | "launch twitch" | "run twitch" => {
            Some("https://www.twitch.tv")
        }
        "open github" | "oeffne github" | "start github" | "launch github" | "run github" => {
            Some("https://github.com")
        }
        "open reddit" | "oeffne reddit" | "start reddit" | "launch reddit" | "run reddit" => {
            Some("https://www.reddit.com")
        }
        "open google" | "oeffne google" | "start google" | "launch google" | "run google" => {
            Some("https://www.google.com")
        }
        _ => None,
    }
}

fn is_direct_open_service_without_title(normalized: &str) -> bool {
    matches!(
        normalized.trim(),
        "open youtube"
            | "oeffne youtube"
            | "start youtube"
            | "launch youtube"
            | "run youtube"
            | "open netflix"
            | "oeffne netflix"
            | "start netflix"
            | "launch netflix"
            | "run netflix"
            | "open spotify"
            | "oeffne spotify"
            | "start spotify"
            | "launch spotify"
            | "run spotify"
            | "open twitch"
            | "oeffne twitch"
            | "start twitch"
            | "launch twitch"
            | "run twitch"
    )
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

fn extract_stream_title(normalized: &str, service: &str) -> Option<String> {
    let mut text = normalized.to_string();

    for prefix in [
        "play ",
        "spiele ",
        "abspielen ",
        "open ",
        "oeffne ",
        "launch ",
        "run ",
        "starte ",
        "start ",
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.trim().to_string();
            break;
        }
    }

    let patterns = [
        format!(" on {}", service),
        format!(" auf {}", service),
        format!(" in {}", service),
        format!(" im {}", service),
    ];

    for p in patterns {
        if let Some(idx) = text.find(&p) {
            text = text[..idx].trim().to_string();
        }
    }

    let cleaned = text
        .replace(" on netflix", "")
        .replace(" auf netflix", "")
        .replace(" in netflix", "")
        .replace(" on youtube", "")
        .replace(" auf youtube", "")
        .replace(" on prime", "")
        .replace(" auf prime", "")
        .replace(" on disney", "")
        .replace(" auf disney", "")
        .trim()
        .to_string();

    if cleaned.is_empty() { None } else { Some(cleaned) }
}

fn is_direct_service_open_command(normalized: &str) -> bool {
    matches!(
        normalized.trim(),
        "open youtube"
            | "oeffne youtube"
            | "start youtube"
            | "starte youtube"
            | "launch youtube"
            | "run youtube"
            | "open netflix"
            | "oeffne netflix"
            | "start netflix"
            | "starte netflix"
            | "launch netflix"
            | "run netflix"
            | "open spotify"
            | "oeffne spotify"
            | "start spotify"
            | "starte spotify"
            | "launch spotify"
            | "run spotify"
            | "open twitch"
            | "oeffne twitch"
            | "start twitch"
            | "starte twitch"
            | "launch twitch"
            | "run twitch"
    )
}

fn parse_media_command(normalized: &str) -> Option<CompanionAction> {
    let toks = tokens(normalized);
    let service = detect_streaming_service(normalized, &toks);
    if matches!(service.as_deref(), Some("youtube")) {
        return None;
    }
        if is_direct_service_open_command(normalized) {
        return None;
    }
    if is_direct_open_service_without_title(normalized) {
        return None;
    }
    if normalized == "yes"
    || normalized == "ja"
    || normalized == "yeah"
    || normalized == "yep"
    || normalized == "do it"
    || normalized == "open it"
    || normalized == "launch it"
    || normalized == "play it"
    || normalized == "open that"
    || normalized == "launch that"
    || normalized == "yes open it"
    || normalized == "yes launch it"
    || normalized == "yes play it"
{
    return Some(CompanionAction::StreamOpenLastSuggestion);
}

    if contains_any_phrase(normalized, STREAMING_MORE_WORDS) {
        return Some(CompanionAction::StreamMoreLikeLast);
    }

    if normalized == "yes"
        || normalized == "ja"
        || normalized == "yeah"
        || normalized == "yep"
        || normalized == "mach"
    {
        return Some(CompanionAction::StreamOpenLastSuggestion);
    }

    if normalized == "no"
        || normalized == "nein"
        || normalized == "cancel"
        || normalized == "stop"
    {
        return Some(CompanionAction::Clear);
    }

    if normalized.contains("what can you play on")
        || normalized.contains("what can you recommend on")
        || normalized.contains("what can you do on")
        || normalized.contains("was kannst du auf")
        || normalized.contains("was kannst du auf netflix")
        || normalized.contains("what can you play on netflix")
    {
        return Some(CompanionAction::StreamCapability { service });
    }

    let trending = normalized.contains("trending")
        || normalized.contains("trend")
        || normalized.contains("popular on")
        || normalized.contains("top on")
        || normalized.contains("was ist im trend")
        || normalized.contains("was ist gerade im trend")
        || normalized.contains("was ist beliebt");

    if trending {
        return Some(CompanionAction::StreamRecommend {
            service,
            mood: None,
            genre: None,
            kind: None,
            trending: true,
        });
    }

    let mood = if contains_any_phrase(normalized, &["funny", "lustig", "witzig", "comedy"]) {
        Some("funny".to_string())
    } else if contains_any_phrase(normalized, &["dark", "duester", "dunkel"]) {
        Some("dark".to_string())
    } else if contains_any_phrase(normalized, &["smart", "clever", "klug", "mind bending"]) {
        Some("smart".to_string())
    } else if contains_any_phrase(normalized, &["sad", "emotional", "traurig"]) {
        Some("emotional".to_string())
    } else if contains_any_phrase(normalized, &["action"]) {
        Some("action".to_string())
    } else if contains_any_phrase(normalized, &["thriller"]) {
        Some("thriller".to_string())
    } else if contains_any_phrase(normalized, &["sci fi", "scifi", "sci-fi", "science fiction"]) {
        Some("scifi".to_string())
    } else {
        None
    };

    let genre = if contains_any_phrase(normalized, &["comedy"]) {
        Some("comedy".to_string())
    } else if contains_any_phrase(normalized, &["animation", "animated"]) {
        Some("animation".to_string())
    } else if contains_any_phrase(normalized, &["crime", "krimi"]) {
        Some("crime".to_string())
    } else if contains_any_phrase(normalized, &["drama"]) {
        Some("drama".to_string())
    } else if contains_any_phrase(normalized, &["fantasy"]) {
        Some("fantasy".to_string())
    } else if contains_any_phrase(normalized, &["documentary", "doku"]) {
        Some("documentary".to_string())
    } else if contains_any_phrase(normalized, &["mystery"]) {
        Some("mystery".to_string())
    } else if contains_any_phrase(normalized, &["thriller"]) {
        Some("thriller".to_string())
    } else if contains_any_phrase(normalized, &["sci fi", "scifi", "sci-fi"]) {
        Some("sci-fi".to_string())
    } else {
        None
    };

    let kind = if contains_any_phrase(normalized, &["movie", "film"]) {
        Some("movie".to_string())
    } else if contains_any_phrase(normalized, &["series", "serie", "show"]) {
        Some("series".to_string())
    } else {
        None
    };

    let sounds_like_recommendation =
        normalized.contains("recommend")
            || normalized.contains("empfiehl")
            || normalized.contains("schlag")
            || normalized.contains("i want")
            || normalized.contains("ich will")
            || normalized.contains("i am in the mood")
            || normalized.contains("ich habe lust")
            || normalized.contains("give me")
            || normalized.contains("gib mir");

    if sounds_like_recommendation || mood.is_some() || genre.is_some() || kind.is_some() {
        return Some(CompanionAction::StreamRecommend {
            service,
            mood,
            genre,
            kind,
            trending: false,
        });
    }

    let play_or_open_media =
        contains_any_phrase(normalized, &["play ", "spiele ", "open ", "oeffne ", "launch ", "starte ", "start "])
            && service.is_some();

    if play_or_open_media {
        let service_name = service.clone().unwrap_or_else(|| "netflix".to_string());
        if let Some(title) = extract_stream_title(normalized, &service_name) {
            return Some(CompanionAction::StreamOpenTitle {
                service: service_name,
                title,
                autoplay: true,
            });
        }
    }

    None
}

fn best_intent(normalized: &str, toks: &[&str]) -> IntentKind {
    if extract_percent(normalized).is_some() && fuzzy_has_any(toks, VOLUME_WORDS, 0.82) {
        return IntentKind::SetVolume;
    }

    let mut scores = vec![
        IntentScore {
            kind: IntentKind::CurrentTime,
            score: score(toks, TIME_WORDS, 0.84, 2.4),
        },
        IntentScore {
            kind: IntentKind::CurrentDate,
            score: score(toks, DATE_WORDS, 0.84, 2.0)
                + if normalized.contains("datum") || normalized.contains("date") {
                    0.8
                } else {
                    0.0
                },
        },
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
            kind: IntentKind::TakeScreenshot,
            score: score(toks, SCREENSHOT_WORDS, 0.90, 2.6),
        },
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
        IntentScore { kind: IntentKind::BrowserOpenUrl, score: if normalized.contains("http") || normalized.contains("www.") || normalized.contains(".com") { 2.1 } else { 0.0 } },
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
    let best = scores
        .first()
        .map(|s| (s.kind, s.score))
        .unwrap_or((IntentKind::None, 0.0));

    if best.1 >= 1.8 {
        best.0
    } else {
        IntentKind::None
    }
}

fn parse_explicit_search_command(normalized: &str) -> Option<CompanionAction> {
    for prefix in [
        "google nach ",
        "google ",
        "suche auf google nach ",
        "search google for ",
        "search on google for ",
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let query = rest.trim();
            if !query.is_empty() {
                return Some(CompanionAction::GoogleSearch {
                    query: query.to_string(),
                });
            }
        }
    }

    for prefix in [
        "youtube nach ",
        "youtube ",
        "suche auf youtube nach ",
        "search youtube for ",
        "search on youtube for ",
    ] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let query = rest.trim();
            if !query.is_empty() {
                return Some(CompanionAction::YouTubeSearch {
                    query: query.to_string(),
                });
            }
        }
    }

    None
}

fn parse_explicit_browser_window_command(normalized: &str) -> Option<CompanionAction> {
    match normalized.trim() {
        "öffne neuen tab" | "oeffne neuen tab" | "open new tab" | "new tab" | "neuer tab" => {
            Some(CompanionAction::NewTab)
        }
        "schließe tab" | "schliesse tab" | "close tab" | "tab schließen" | "tab schliessen" => {
            Some(CompanionAction::CloseTab)
        }
        "öffne neues fenster" | "oeffne neues fenster" | "open new window" | "new window" | "neues fenster" => {
            Some(CompanionAction::NewWindow)
        }
        "schließe fenster" | "schliesse fenster" | "close window" | "fenster schließen" | "fenster schliessen" => {
            Some(CompanionAction::Close)
        }
        _ => None,
    }
}

pub fn parse_voice_command(input: &str) -> CompanionAction {
    let normalized = normalize(input);

    if normalized.is_empty() {
        return CompanionAction::None;
    }

    if let Some(action) = parse_explicit_search_command(&normalized) {
        return action;
    }

    if let Some(action) = parse_explicit_browser_window_command(&normalized) {
        return action;
    }

    if let Some(action) = parse_media_command(&normalized) {
        return action;
    }

    match normalized.trim() {
        "wie viel uhr ist es"
        | "wie spaet ist es"
        | "wie spät ist es"
        | "uhrzeit"
        | "what time is it"
        | "current time" => {
            return CompanionAction::CurrentTime;
        }

        "welcher tag ist heute"
        | "welches datum haben wir"
        | "welches datum ist heute"
        | "heutiges datum"
        | "what date is it"
        | "current date" => {
            return CompanionAction::CurrentDate;
        }
        _ => {}
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

        IntentKind::GoogleSearch => CompanionAction::GoogleSearch {
            query: extract_search_query(
                &normalized,
                &toks,
                &["google", "googel", "gogle", "search", "suche", "such", "find"],
            ),
        },

        IntentKind::YouTubeSearch => CompanionAction::YouTubeSearch {
            query: extract_search_query(
                &normalized,
                &toks,
                &["youtube", "youtub", "jutube", "jutub", "yt", "search", "suche", "such", "find"],
            ),
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

        IntentKind::CurrentTime => CompanionAction::CurrentTime,
        IntentKind::CurrentDate => CompanionAction::CurrentDate,

        IntentKind::WeatherToday => CompanionAction::WeatherToday {
            location: extract_weather_location(&normalized),
        },

        IntentKind::ExplainSelection => CompanionAction::ExplainSelection,
        IntentKind::TakeScreenshot => CompanionAction::TakeScreenshot,
        IntentKind::None => CompanionAction::None,
    }
}