use crate::modules::i18n::command_locale::command_locale;

use super::constants::*;
use super::extract::{detect_known_target, extract_after_prefixes, extract_number, extract_percent};
use super::fuzzy::{fuzzy_has_any, score, score_strings};
use super::types::{IntentKind, IntentScore};

pub fn best_intent(normalized: &str, toks: &[&str]) -> IntentKind {
    if extract_percent(normalized).is_some() && fuzzy_has_any(toks, VOLUME_WORDS, 0.82) {
        return IntentKind::SetVolume;
    }

    let locale = command_locale();

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
        IntentScore {
            kind: IntentKind::VolumeUp,
            score: score(toks, VOLUME_UP_WORDS, 0.84, 1.8)
                + score(toks, VOLUME_WORDS, 0.84, 0.8),
        },
        IntentScore {
            kind: IntentKind::VolumeDown,
            score: score(toks, VOLUME_DOWN_WORDS, 0.84, 1.8)
                + score(toks, VOLUME_WORDS, 0.84, 0.8),
        },
        IntentScore {
            kind: IntentKind::Mute,
            score: score(toks, MUTE_WORDS, 0.84, 1.7)
                + score(toks, VOLUME_WORDS, 0.82, 0.5),
        },
        IntentScore {
            kind: IntentKind::Unmute,
            score: score(toks, UNMUTE_WORDS, 0.84, 1.7)
                + score(toks, VOLUME_WORDS, 0.82, 0.4),
        },
        IntentScore {
            kind: IntentKind::MediaPlayPause,
            score: score(toks, PAUSE_WORDS, 0.84, 1.8)
                + score(toks, &["music", "musik", "song", "track", "video"], 0.84, 0.6),
        },
        IntentScore {
            kind: IntentKind::MediaNext,
            score: score(toks, NEXT_WORDS, 0.84, 1.8)
                + score(toks, &["music", "musik", "song", "track"], 0.84, 0.5),
        },
        IntentScore {
            kind: IntentKind::MediaPrev,
            score: score(toks, PREV_WORDS, 0.84, 1.8)
                + score(toks, &["music", "musik", "song", "track"], 0.84, 0.5),
        },
        IntentScore {
            kind: IntentKind::GoogleSearch,
            score: score(toks, GOOGLE_WORDS, 0.88, 2.0)
                + score(toks, &["search", "suche", "such", "find"], 0.86, 1.0),
        },
        IntentScore {
            kind: IntentKind::YouTubeSearch,
            score: score(toks, YOUTUBE_WORDS, 0.86, 2.0)
                + score(toks, &["search", "suche", "such", "find"], 0.86, 0.9),
        },
        IntentScore {
            kind: IntentKind::YouTubePlayTitle,
            score: score(toks, PLAY_WORDS, 0.84, 1.7)
                + score(toks, YOUTUBE_WORDS, 0.84, 0.6)
                + score(toks, RESULT_WORDS, 0.84, 0.3),
        },
        IntentScore {
            kind: IntentKind::TakeScreenshot,
            score: score_strings(toks, &locale.screenshot_words, 0.90, 2.6),
        },
        IntentScore {
            kind: IntentKind::OpenApp,
            score: score(toks, OPEN_WORDS, 0.88, 1.8)
                + if detect_known_target(toks).is_some() { 1.5 } else { 0.0 }
                + if extract_after_prefixes(
                    normalized,
                    &["oeffne ", "oeffne mal ", "starte ", "start ", "open ", "launch ", "run "],
                )
                .is_some()
                {
                    1.2
                } else {
                    0.0
                }
                + score(toks, BROWSER_WORDS, 0.88, 0.5),
        },
        IntentScore {
            kind: IntentKind::Close,
            score: score(toks, CLOSE_WORDS, 0.86, 2.0),
        },
        IntentScore {
            kind: IntentKind::SaveAs,
            score: score(toks, SAVE_AS_WORDS, 0.82, 2.4)
                + if normalized.contains("save as") || normalized.contains("speichern unter") {
                    1.0
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::Save,
            score: score(toks, SAVE_WORDS, 0.88, 2.0),
        },
        IntentScore {
            kind: IntentKind::OpenFile,
            score: score(toks, OPEN_FILE_WORDS, 0.82, 1.8)
                + if normalized == "oeffnen" || normalized == "open" {
                    0.8
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::NewFile,
            score: score(toks, NEW_FILE_WORDS, 0.82, 1.8),
        },
        IntentScore {
            kind: IntentKind::Undo,
            score: score(toks, UNDO_WORDS, 0.86, 2.0),
        },
        IntentScore {
            kind: IntentKind::Redo,
            score: score(toks, REDO_WORDS, 0.86, 2.0),
        },
        IntentScore {
            kind: IntentKind::CloseTab,
            score: score(toks, TAB_CLOSE_WORDS, 0.82, 2.3)
                + if normalized.contains("close tab") || normalized.contains("tab schliessen") {
                    0.8
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::CloseTabByIndex,
            score: if normalized.contains("tab")
                && extract_number(toks).is_some()
                && fuzzy_has_any(toks, CLOSE_WORDS, 0.84)
            {
                2.6
            } else {
                0.0
            },
        },
        IntentScore {
            kind: IntentKind::NewTab,
            score: score(toks, TAB_NEW_WORDS, 0.82, 2.3)
                + if normalized.contains("new tab") || normalized.contains("neuer tab") {
                    0.8
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::NewWindow,
            score: score(toks, WINDOW_NEW_WORDS, 0.82, 2.3)
                + if normalized.contains("new window") || normalized.contains("neues fenster") {
                    0.8
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::Incognito,
            score: score(toks, INCOGNITO_WORDS, 0.82, 2.3),
        },
        IntentScore {
            kind: IntentKind::Reload,
            score: score(toks, RELOAD_WORDS, 0.82, 2.0),
        },
        IntentScore {
            kind: IntentKind::BrowserOpenUrl,
            score: if normalized.contains("http")
                || normalized.contains("www.")
                || normalized.contains(".com")
            {
                2.1
            } else {
                0.0
            },
        },
        IntentScore {
            kind: IntentKind::BrowserClickLinkByText,
            score: score(toks, CLICK_WORDS, 0.82, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserClickFirstResult,
            score: if normalized.contains("erstes ergebnis")
                || normalized.contains("first result")
            {
                2.5
            } else {
                0.0
            },
        },
        IntentScore {
            kind: IntentKind::BrowserClickNthResult,
            score: if normalized.contains("ergebnis") && extract_number(toks).is_some() {
                2.4
            } else {
                0.0
            },
        },
        IntentScore {
            kind: IntentKind::BrowserBack,
            score: score(toks, BACK_WORDS, 0.82, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserForward,
            score: score(toks, FORWARD_WORDS, 0.82, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserScrollDown,
            score: score(toks, SCROLL_DOWN_WORDS, 0.80, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserScrollUp,
            score: score(toks, SCROLL_UP_WORDS, 0.80, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserTypeText,
            score: score(toks, TYPE_WORDS, 0.82, 2.1),
        },
        IntentScore {
            kind: IntentKind::BrowserSubmit,
            score: score(toks, SUBMIT_WORDS, 0.82, 2.2),
        },
        IntentScore {
            kind: IntentKind::BrowserClickBestMatch,
            score: score(toks, CLICK_WORDS, 0.82, 1.8)
                + score(toks, RESULT_WORDS, 0.80, 0.3),
        },
        IntentScore {
            kind: IntentKind::BrowserContext,
            score: score(toks, CONTEXT_WORDS, 0.80, 2.3),
        },
        IntentScore {
            kind: IntentKind::YouTubeNextVideo,
            score: score(toks, YT_NEXT_WORDS, 0.82, 2.2)
                + if normalized.contains("next video") || normalized.contains("naechstes video") {
                    0.7
                } else {
                    0.0
                },
        },
        IntentScore {
            kind: IntentKind::YouTubeSeekForward,
            score: score(toks, YT_FORWARD_WORDS, 0.84, 2.0),
        },
        IntentScore {
            kind: IntentKind::YouTubeSeekBackward,
            score: score(toks, YT_BACK_WORDS, 0.84, 2.0),
        },
        IntentScore {
            kind: IntentKind::WeatherToday,
            score: score(toks, WEATHER_WORDS, 0.84, 2.0)
                + score(toks, &["today", "heute", "now", "jetzt"], 0.88, 0.7),
        },
        IntentScore {
            kind: IntentKind::ExplainSelection,
            score: score(toks, EXPLAIN_WORDS, 0.84, 1.8)
                + score(toks, &["text", "this", "that", "das"], 0.84, 0.5),
        },
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