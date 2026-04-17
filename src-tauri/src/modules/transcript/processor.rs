use serde_json::Value;

use super::{
    session,
    types::{ProcessedTranscriptResult, SpeakerBlock, TranscriptSession},
};

fn build_prompt(session: &TranscriptSession, transcript: &str) -> String {
    let app_name = session
        .context
        .app_name
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    let window_title = session
        .context
        .window_title
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        r#"You are an AI assistant that processes raw transcripts.

Context:
- Source app: {}
- Window title: {}

Your job:
Return useful transcript material in a faithful, non-hallucinating way.

Requirements:

1. faithful_transcript
- Keep it as close as possible to the original wording
- Fix only obvious ASR breakage, punctuation, grammar, and chunk fragmentation
- Do NOT freely rewrite
- Do NOT invent facts
- Do NOT over-compress
- Preserve order and meaning
- Keep technical terms and names as faithfully as possible

2. speaker_blocks
- Group the transcript into conservative speaker turns
- Use generic labels like "Speaker 1", "Speaker 2", etc.
- IMPORTANT: the "text" field must contain ONLY the spoken content
- IMPORTANT: do NOT repeat labels like "Speaker 1:" inside the text
- IMPORTANT: prefer fewer, larger blocks over many tiny repeated blocks
- If speaker boundaries are unclear, keep grouping conservative and stable

3. summary
- concise and useful
- based only on the transcript

4. action_items
- only concrete follow-ups or tasks
- if none, return []

Return ONLY valid JSON with this exact shape:

{{
  "faithful_transcript": "...",
  "speaker_blocks": [
    {{
      "speaker": "Speaker 1",
      "text": "..."
    }}
  ],
  "summary": "...",
  "action_items": ["..."]
}}

Transcript:
{}
"#,
        app_name, window_title, transcript
    )
}

fn extract_json(content: &str) -> Option<String> {
    let trimmed = content.trim();

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed.to_string());
    }

    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return Some(trimmed[start..=end].to_string());
            }
        }
    }

    None
}

fn strip_embedded_speaker_prefix(text: &str, speaker: &str) -> String {
    let trimmed = text.trim();

    let variants = [
        format!("{}:", speaker),
        format!("{} -", speaker),
        format!("{} –", speaker),
    ];

    for variant in variants {
        if trimmed.starts_with(&variant) {
            return trimmed[variant.len()..].trim().to_string();
        }
    }

    if trimmed.to_ascii_lowercase().starts_with("speaker ") {
        if let Some(pos) = trimmed.find(':') {
            return trimmed[pos + 1..].trim().to_string();
        }
    }

    trimmed.to_string()
}

fn parse_speaker_blocks(parsed: &Value) -> Vec<SpeakerBlock> {
    let mut blocks = parsed
        .get("speaker_blocks")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let speaker = item.get("speaker")?.as_str()?.trim().to_string();
                    let raw_text = item.get("text")?.as_str()?.trim().to_string();

                    if speaker.is_empty() || raw_text.is_empty() {
                        return None;
                    }

                    let text = strip_embedded_speaker_prefix(&raw_text, &speaker);

                    if text.is_empty() {
                        return None;
                    }

                    Some(SpeakerBlock { speaker, text })
                })
                .collect::<Vec<SpeakerBlock>>()
        })
        .unwrap_or_default();

    blocks = merge_adjacent_same_speaker(blocks);
    blocks
}

fn merge_adjacent_same_speaker(blocks: Vec<SpeakerBlock>) -> Vec<SpeakerBlock> {
    let mut merged: Vec<SpeakerBlock> = Vec::new();

    for block in blocks {
        if let Some(last) = merged.last_mut() {
            if last.speaker == block.speaker {
                if !last.text.ends_with(' ') {
                    last.text.push(' ');
                }
                last.text.push_str(block.text.trim());
                continue;
            }
        }

        merged.push(block);
    }

    merged
}

pub async fn process_best_available_transcript() -> Result<ProcessedTranscriptResult, String> {
    let session = session::get_best_available_session()?
        .ok_or_else(|| "No transcript session available".to_string())?;

    let transcript = session::build_clean_transcript(&session);

    if transcript.trim().is_empty() {
        return Err("Transcript is empty".into());
    }

    let prompt = build_prompt(&session, &transcript);

    let client = reqwest::Client::new();

    let response = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&serde_json::json!({
            "model": "llama3.1:8b",
            "stream": false,
            "messages": [
                {
                    "role": "system",
                    "content": "You return only valid JSON. Do not include markdown fences."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    let value: Value = response
        .json()
        .await
        .map_err(|e| format!("Invalid Ollama response: {}", e))?;

    let raw_content = value
        .get("message")
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing message.content".to_string())?;

    let json_str = extract_json(raw_content)
        .ok_or_else(|| format!("No JSON found in AI output:\n{}", raw_content))?;

    let parsed: Value = serde_json::from_str(&json_str)
        .unwrap_or_else(|_| serde_json::json!({}));

    let faithful_transcript = parsed
        .get("faithful_transcript")
        .and_then(|v| v.as_str())
        .unwrap_or("No faithful transcript generated.")
        .trim()
        .to_string();

    let summary = parsed
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("No summary generated.")
        .trim()
        .to_string();

    let action_items = parsed
        .get("action_items")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let speaker_blocks = parse_speaker_blocks(&parsed);

    Ok(ProcessedTranscriptResult {
        faithful_transcript,
        speaker_blocks,
        summary,
        action_items,
    })
}