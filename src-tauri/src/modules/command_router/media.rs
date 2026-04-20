use super::constants::{STREAMING_MORE_WORDS};
use super::extract::{detect_streaming_service, extract_stream_title};
use super::fuzzy::contains_any_phrase;
use super::normalize::tokens;
use super::types::CompanionAction;

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

pub fn parse_media_command(normalized: &str) -> Option<CompanionAction> {
    let toks = tokens(normalized);
    let service = detect_streaming_service(normalized, &toks);

    if matches!(service.as_deref(), Some("youtube")) {
        if is_direct_service_open_command(normalized) {
            return None;
        }
        let has_content_verb =
            contains_any_phrase(normalized, &["open ", "play ", "spiele ", "oeffne "]);
        if has_content_verb {
            let title = extract_stream_title(normalized, "youtube");
            if let Some(t) = title {
                return Some(CompanionAction::YouTubeSearch { query: t });
            }
        }
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
        || normalized == "mach"
    {
        return Some(CompanionAction::StreamOpenLastSuggestion);
    }

    if contains_any_phrase(normalized, STREAMING_MORE_WORDS) {
        return Some(CompanionAction::StreamMoreLikeLast);
    }

    if normalized == "no" || normalized == "nein" || normalized == "cancel" || normalized == "stop"
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

    let sounds_like_recommendation = normalized.contains("recommend")
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

    let play_or_open_media = contains_any_phrase(
        normalized,
        &["play ", "spiele ", "open ", "oeffne ", "launch ", "starte ", "start "],
    ) && service.is_some();

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