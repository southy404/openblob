use super::types::{TranscriptSession, TranscriptSummary};

pub fn summarize_session(session: &TranscriptSession) -> TranscriptSummary {
    let joined = session
        .segments
        .iter()
        .map(|s| s.text.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let summary = if joined.is_empty() {
        "No transcript content available yet.".to_string()
    } else if joined.len() > 500 {
        format!("{}...", &joined[..500])
    } else {
        joined
    };

    TranscriptSummary {
        summary,
        action_items: Vec::new(),
    }
}