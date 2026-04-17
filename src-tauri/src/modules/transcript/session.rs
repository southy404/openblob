use std::sync::Mutex;

use chrono::Utc;
use once_cell::sync::Lazy;

use super::types::{
    StartTranscriptRequest, TranscriptContext, TranscriptSegment, TranscriptSession,
    TranscriptState, TranscriptStatus,
};

static ACTIVE_SESSION: Lazy<Mutex<Option<TranscriptSession>>> = Lazy::new(|| Mutex::new(None));
static LAST_FINISHED_SESSION: Lazy<Mutex<Option<TranscriptSession>>> =
    Lazy::new(|| Mutex::new(None));

pub fn start_session(request: StartTranscriptRequest) -> Result<TranscriptSession, String> {
    let mut guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;

    if guard.is_some() {
        return Err("Transcript session is already active".into());
    }

    let session = TranscriptSession {
        id: format!("transcript_{}", Utc::now().format("%Y_%m_%d_%H_%M_%S")),
        source: request.source,
        state: TranscriptState::Recording,
        started_at: Utc::now().to_rfc3339(),
        ended_at: None,
        context: TranscriptContext {
            app_name: request.app_name,
            window_title: request.window_title,
        },
        segments: Vec::new(),
    };

    *guard = Some(session.clone());
    Ok(session)
}

pub fn stop_session() -> Result<TranscriptSession, String> {
    let mut guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;

    let session = guard
        .as_mut()
        .ok_or_else(|| "No active transcript session".to_string())?;

    session.state = TranscriptState::Stopping;
    Ok(session.clone())
}

pub fn finish_session() -> Result<TranscriptSession, String> {
    let mut active_guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;

    let mut finished = active_guard
        .take()
        .ok_or_else(|| "No active transcript session".to_string())?;

    finished.state = TranscriptState::Idle;
    finished.ended_at = Some(Utc::now().to_rfc3339());

    let mut last_guard = LAST_FINISHED_SESSION
        .lock()
        .map_err(|_| "Failed to lock last transcript session".to_string())?;
    *last_guard = Some(finished.clone());

    Ok(finished)
}

pub fn append_segment(segment: TranscriptSegment) -> Result<TranscriptSession, String> {
    let mut guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;

    let session = guard
        .as_mut()
        .ok_or_else(|| "No active transcript session".to_string())?;

    let exists = session.segments.iter().any(|s| {
        s.start_ms == segment.start_ms && s.end_ms == segment.end_ms && s.text == segment.text
    });

    if !exists {
        session.segments.push(segment);
    }

    Ok(session.clone())
}

pub fn get_active_session() -> Result<Option<TranscriptSession>, String> {
    let guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;
    Ok(guard.clone())
}

pub fn get_last_finished_session() -> Result<Option<TranscriptSession>, String> {
    let guard = LAST_FINISHED_SESSION
        .lock()
        .map_err(|_| "Failed to lock last transcript session".to_string())?;
    Ok(guard.clone())
}

pub fn get_best_available_session() -> Result<Option<TranscriptSession>, String> {
    if let Some(active) = get_active_session()? {
        return Ok(Some(active));
    }

    get_last_finished_session()
}

pub fn get_status() -> Result<TranscriptStatus, String> {
    let guard = ACTIVE_SESSION
        .lock()
        .map_err(|_| "Failed to lock active transcript session".to_string())?;

    if let Some(session) = guard.as_ref() {
        return Ok(TranscriptStatus {
            state: session.state.clone(),
            active_session_id: Some(session.id.clone()),
            segment_count: session.segments.len(),
        });
    }

    let last_guard = LAST_FINISHED_SESSION
        .lock()
        .map_err(|_| "Failed to lock last transcript session".to_string())?;

    if let Some(session) = last_guard.as_ref() {
        return Ok(TranscriptStatus {
            state: TranscriptState::Idle,
            active_session_id: None,
            segment_count: session.segments.len(),
        });
    }

    Ok(TranscriptStatus {
        state: TranscriptState::Idle,
        active_session_id: None,
        segment_count: 0,
    })
}

pub fn build_clean_transcript(session: &TranscriptSession) -> String {
    let mut cleaned: Vec<String> = Vec::new();

    for segment in &session.segments {
        let text = normalize_segment_text(&segment.text);
        if text.is_empty() {
            continue;
        }

        if let Some(last) = cleaned.last_mut() {
            if same_text(last, &text) {
                continue;
            }

            if let Some(merged) = merge_if_overlap(last, &text) {
                *last = merged;
                continue;
            }

            if should_join_inline(last, &text) {
                if !last.ends_with(' ') {
                    last.push(' ');
                }
                last.push_str(&text);
                continue;
            }
        }

        cleaned.push(text);
    }

    cleaned.join("\n")
}

pub fn get_clean_transcript_from_best_session() -> Result<String, String> {
    let session = get_best_available_session()?
        .ok_or_else(|| "No transcript session available".to_string())?;

    Ok(build_clean_transcript(&session))
}

fn normalize_segment_text(input: &str) -> String {
    let mut text = input.trim().replace('\n', " ");

    while text.contains("  ") {
        text = text.replace("  ", " ");
    }

    text = text
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" :", ":")
        .replace(" ;", ";");

    text.trim().to_string()
}

fn same_text(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn should_join_inline(prev: &str, next: &str) -> bool {
    if prev.is_empty() || next.is_empty() {
        return false;
    }

    let prev_ends_hard = prev.ends_with('.') || prev.ends_with('!') || prev.ends_with('?');
    let next_starts_lowercase = next.chars().next().is_some_and(|c| c.is_lowercase());

    !prev_ends_hard || next_starts_lowercase
}

fn merge_if_overlap(prev: &str, next: &str) -> Option<String> {
    let prev_words: Vec<&str> = prev.split_whitespace().collect();
    let next_words: Vec<&str> = next.split_whitespace().collect();

    let max_overlap = prev_words.len().min(next_words.len()).min(8);

    for overlap in (2..=max_overlap).rev() {
        let prev_tail = &prev_words[prev_words.len() - overlap..];
        let next_head = &next_words[..overlap];

        let matches = prev_tail
            .iter()
            .zip(next_head.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let mut merged = prev_words.join(" ");
            let remainder = next_words[overlap..].join(" ");

            if !remainder.is_empty() {
                if !merged.ends_with(' ') {
                    merged.push(' ');
                }
                merged.push_str(&remainder);
            }

            return Some(merged);
        }
    }

    None
}