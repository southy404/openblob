use std::fs;
use std::path::PathBuf;

use super::types::TranscriptSession;

fn transcript_root_dir() -> Result<PathBuf, String> {
    Ok(PathBuf::from("D:\\openblob-data").join("transcripts"))
}

fn session_dir(session_id: &str) -> Result<PathBuf, String> {
    Ok(transcript_root_dir()?.join(session_id))
}

pub fn save_session(session: &TranscriptSession) -> Result<PathBuf, String> {
    let dir = session_dir(&session.id)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let json_path = dir.join("session.json");
    let md_path = dir.join("transcript.md");

    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(&json_path, json).map_err(|e| e.to_string())?;

    let markdown = render_markdown(session);
    fs::write(&md_path, markdown).map_err(|e| e.to_string())?;

    Ok(dir)
}

pub fn render_markdown(session: &TranscriptSession) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Transcript {}\n\n", session.id));
    out.push_str(&format!("- Source: {:?}\n", session.source));
    out.push_str(&format!("- Started: {}\n", session.started_at));

    if let Some(ended_at) = &session.ended_at {
        out.push_str(&format!("- Ended: {}\n", ended_at));
    }

    if let Some(app_name) = &session.context.app_name {
        out.push_str(&format!("- App: {}\n", app_name));
    }

    if let Some(window_title) = &session.context.window_title {
        out.push_str(&format!("- Window: {}\n", window_title));
    }

    out.push_str("\n## Segments\n\n");

    for seg in &session.segments {
        out.push_str(&format!(
            "- [{} - {}] {}\n",
            seg.start_ms, seg.end_ms, seg.text
        ));
    }

    out
}