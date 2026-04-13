use strsim::jaro_winkler;

use super::constants::{
    BROWSER_WORDS, INCOGNITO_WORDS, KNOWN_TARGETS, STREAMING_SERVICE_ALIASES, TAB_NEW_WORDS,
    WINDOW_NEW_WORDS,
};
use super::fuzzy::{best_similarity, fuzzy_has_any};
use super::normalize::{normalize, tokens};

pub fn extract_percent(normalized: &str) -> Option<u8> {
    for token in normalized.split_whitespace() {
        let cleaned = token.replace('%', "").replace("prozent", "").replace("percent", "");
        if let Ok(value) = cleaned.parse::<u8>() {
            return Some(value.min(100));
        }
    }
    None
}

pub fn extract_after_prefixes(normalized: &str, prefixes: &[&str]) -> Option<String> {
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

pub fn extract_quoted_text(input: &str) -> Option<String> {
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

pub fn extract_number(tokens: &[&str]) -> Option<usize> {
    for token in tokens {
        if let Ok(n) = token.parse::<usize>() {
            if n > 0 {
                return Some(n);
            }
        }
    }
    None
}

pub fn extract_search_query(normalized: &str, toks: &[&str], remove_aliases: &[&str]) -> String {
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

pub fn extract_generic_search_query(input: &str) -> Option<String> {
    let normalized = normalize(input);

    for prefix in ["suche nach ", "suche ", "search for ", "search ", "finde ", "find "] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            let q = rest.trim();
            if !q.is_empty() {
                return Some(q.to_string());
            }
        }
    }

    None
}

pub fn extract_timer_seconds(normalized: &str) -> Option<u64> {
    let text = normalized.trim();

    for token in text.split_whitespace() {
        if let Some((left, right)) = token.split_once(':') {
            if let (Ok(minutes), Ok(seconds)) = (left.parse::<u64>(), right.parse::<u64>()) {
                if seconds < 60 {
                    return Some(minutes * 60 + seconds);
                }
            }
        }
    }

    let tokens: Vec<&str> = text.split_whitespace().collect();

    let mut total_seconds = 0u64;
    let mut found = false;

    for i in 0..tokens.len() {
        if let Ok(value) = tokens[i].parse::<u64>() {
            if let Some(unit) = tokens.get(i + 1) {
                match *unit {
                    "m" | "min" | "mins" | "minute" | "minuten" => {
                        total_seconds += value * 60;
                        found = true;
                    }
                    "s" | "sec" | "secs" | "second" | "seconds" | "sek" | "sekunde" | "sekunden" => {
                        total_seconds += value;
                        found = true;
                    }
                    _ => {}
                }
            }
        }
    }

    if found {
        Some(total_seconds.max(1))
    } else {
        None
    }
}

pub fn extract_timer_minutes(input: &str) -> Option<u64> {
    let tokens: Vec<&str> = input.split_whitespace().collect();

    for (i, token) in tokens.iter().enumerate() {
        if let Ok(value) = token.parse::<u64>() {
            let next = tokens.get(i + 1).copied().unwrap_or("");

            if next.starts_with("min") || next == "minute" || next == "minuten" {
                return Some(value);
            }

            if input.contains("timer") {
                return Some(value);
            }
        }
    }

    if input.contains("one minute") || input.contains("eine minute") {
        return Some(1);
    }
    if input.contains("two minutes") || input.contains("zwei minuten") {
        return Some(2);
    }
    if input.contains("five minutes") || input.contains("fünf minuten") {
        return Some(5);
    }
    if input.contains("ten minutes") || input.contains("zehn minuten") {
        return Some(10);
    }

    None
}

pub fn detect_known_target(tokens: &[&str]) -> Option<String> {
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

pub fn detect_streaming_service(normalized: &str, toks: &[&str]) -> Option<String> {
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

pub fn extract_open_target(normalized: &str, toks: &[&str]) -> (String, bool) {
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

pub fn extract_weather_location(normalized: &str) -> Option<String> {
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

pub fn wants_new_tab(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("new tab")
        || normalized.contains("neuer tab")
        || fuzzy_has_any(toks, TAB_NEW_WORDS, 0.82)
}

pub fn wants_new_window(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("new window")
        || normalized.contains("neues fenster")
        || fuzzy_has_any(toks, WINDOW_NEW_WORDS, 0.82)
}

pub fn wants_incognito(normalized: &str, toks: &[&str]) -> bool {
    normalized.contains("inkognito")
        || normalized.contains("incognito")
        || fuzzy_has_any(toks, INCOGNITO_WORDS, 0.82)
}

pub fn extract_after_command(normalized: &str, commands: &[&str]) -> Option<String> {
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

pub fn extract_stream_title(normalized: &str, service: &str) -> Option<String> {
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

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}