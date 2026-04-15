use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::time::Duration;

use crate::core::legacy::app_open_runtime;
use crate::modules::context::{is_internal_companion_app, resolve_active_context};
use crate::modules::i18n::replies::{reply, reply_with};
use crate::modules::snip_session::{set_snip, SnipSession};

#[derive(Debug, Serialize)]
pub struct ActiveSnipContext {
    pub app_name: String,
    pub window_title: String,
    pub context_domain: String,
}

#[derive(Debug, Serialize)]
pub struct OllamaResult {
    pub content: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

fn default_vision_model() -> String {
    "gemma3".to_string()
}

fn is_useful_external_app(app: &str) -> bool {
    let trimmed = app.trim();
    !trimmed.is_empty() && trimmed != "unknown" && !is_internal_companion_app(trimmed)
}

pub fn resolve_snip_context() -> ActiveSnipContext {
    let context = resolve_active_context();

    let mut app_name = context.app_name.clone();
    let mut window_title = context.window_title.clone();
    let context_domain = context.domain.clone();

    if !is_useful_external_app(&app_name) {
        let remembered = app_open_runtime::get_last_external_app();
        if is_useful_external_app(&remembered) {
            app_name = remembered;
        }
    }

    if window_title.trim().is_empty() && is_useful_external_app(&app_name) {
        window_title = app_name.clone();
    }

    ActiveSnipContext {
        app_name: if app_name.trim().is_empty() {
            reply("snip_unknown")
        } else {
            app_name
        },
        window_title,
        context_domain,
    }
}

pub fn create_snip(comment: Option<String>) -> Result<String, String> {
    let context = resolve_active_context();

    let image_path = crate::modules::screen_capture::capture_region_to_file(0, 0, 400, 300)?;

    let session = SnipSession {
        image_path: image_path.clone(),
        comment: comment.unwrap_or_default(),
        context_app: context.app_name,
        context_domain: context.domain,
        window_title: context.window_title,
    };

    set_snip(session);

    Ok(reply_with("snip_created", &[("path", image_path)]))
}

fn clean_search_query(query: &str) -> String {
    let mut q = query.to_string();

    let noise = [
        "snip overlay",
        "snip panel",
        "companion",
        "companion-v1",
        "overlay",
        ".exe",
    ];

    for n in noise {
        q = q.replace(n, "");
    }

    q.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_snip_search_prompt(comment: &str, app_name: &str, window_title: &str) -> String {
    format!(
        "You are analyzing a screenshot.

STEP 1: Extract ALL visible text from the image.
- Prioritize LARGE titles, headers, mission names, locations.
- Then extract all smaller readable text.
- Preserve original language.
- Do NOT summarize.

STEP 2: Determine context.
- Is this a game, UI, error, or other?
If a game is detected or app context is known:
- ALWAYS include the real game/app name in the search query
- Prefer real game name over executable name

STEP 3: Build a HIGH QUALITY search query.
STRICT RULES:
- MUST be based on extracted image text
- MUST include key phrases from the image
- DO NOT use user comment as main text
- DO NOT invent quest names
- DO NOT use generic queries

USER COMMENT:
{comment}

APP CONTEXT:
{app_name}

WINDOW TITLE:
{window_title}

Return EXACTLY in this format:

INTENT: <quest_help | puzzle_help | location_help | error_help | general_search>
GAME_OR_APP: <best guess or unknown>
EXTRACTED_TEXT:
<all visible text from image>

KEY_TEXT:
<most important title or mission text>

SEARCH_QUERY:
<precise query using extracted text>

ALT_QUERY_1:
<broader query>

ALT_QUERY_2:
<video/guide query>

ANSWER:
<short helpful explanation>",
        comment = comment.trim(),
        app_name = app_name.trim(),
        window_title = window_title.trim(),
    )
}

fn build_snip_vision_prompt(mode: &str, comment: &str) -> String {
    let extra = if comment.trim().is_empty() {
        String::new()
    } else {
        format!("\n\nUSER COMMENT:\n{}", comment.trim())
    };

    match mode {
        "ocr" => format!(
            "Read all visible text from this screenshot exactly as well as possible. \
Return plain text only. Preserve line breaks where helpful. \
If some text is unclear, mark it with [unclear].{}",
            extra
        ),
        "translate" => format!(
            "Look at this screenshot, read the visible text, and translate it into natural German. \
Do not describe the image unless necessary. \
If there is very little text, say that clearly.{}",
            extra
        ),
        "search" => format!(
            "Look at this screenshot and identify the main relevant text, topic, quest, error, or UI issue. \
Return your answer in exactly this format:\n\
SEARCH QUERY: <a concise web search query>\n\
SUMMARY: <1-3 lines explaining what the screenshot likely shows>\n\
KEY TEXT: <important extracted text>\n\
If nothing useful is visible, say so clearly.{}",
            extra
        ),
        _ => format!(
            "Look at this screenshot and explain clearly what it shows. \
If there is visible text, include the important text in your explanation. \
If this appears to be a game, app, or UI, explain what is happening and what the user likely needs.{}",
            extra
        ),
    }
}

fn extract_labeled_value(text: &str, label: &str) -> String {
    let prefix = format!("{label}:");
    let lines: Vec<&str> = text.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with(&prefix) {
            let after = line.trim_start()[prefix.len()..].trim();
            if !after.is_empty() {
                return after.to_string();
            }

            let mut collected = Vec::new();
            for next in lines.iter().skip(idx + 1) {
                let trimmed = next.trim();
                if trimmed.is_empty() {
                    if !collected.is_empty() {
                        break;
                    }
                    continue;
                }

                let looks_like_next_label = [
                    "INTENT:",
                    "GAME_OR_APP:",
                    "EXTRACTED_TEXT:",
                    "KEY_TEXT:",
                    "SEARCH_QUERY:",
                    "ALT_QUERY_1:",
                    "ALT_QUERY_2:",
                    "ANSWER:",
                ]
                .iter()
                .any(|known| trimmed.starts_with(known));

                if looks_like_next_label {
                    break;
                }

                collected.push(trimmed);
            }

            return collected.join(" ");
        }
    }

    String::new()
}

fn format_search_result(raw: &str) -> String {
    let unknown = reply("snip_unknown");
    let no_answer = reply("snip_no_concise_answer");

    let intent = extract_labeled_value(raw, "INTENT");
    let game_or_app = extract_labeled_value(raw, "GAME_OR_APP");
    let extracted_text = extract_labeled_value(raw, "EXTRACTED_TEXT");
    let key_text = extract_labeled_value(raw, "KEY_TEXT");
    let search_query_raw = extract_labeled_value(raw, "SEARCH_QUERY");
    let search_query = clean_search_query(&search_query_raw);
    let alt_query_1 = extract_labeled_value(raw, "ALT_QUERY_1");
    let alt_query_2 = extract_labeled_value(raw, "ALT_QUERY_2");
    let answer = extract_labeled_value(raw, "ANSWER");

    reply_with(
        "snip_search_intent",
        &[
            ("intent", if intent.is_empty() { unknown.clone() } else { intent }),
            (
                "game_or_app",
                if game_or_app.is_empty() {
                    unknown.clone()
                } else {
                    game_or_app
                },
            ),
            (
                "extracted_text",
                if extracted_text.is_empty() {
                    unknown.clone()
                } else {
                    extracted_text
                },
            ),
            ("key_text", if key_text.is_empty() { unknown.clone() } else { key_text }),
            (
                "search_query",
                if search_query.is_empty() {
                    unknown.clone()
                } else {
                    search_query
                },
            ),
            (
                "alt_query_1",
                if alt_query_1.is_empty() {
                    unknown.clone()
                } else {
                    alt_query_1
                },
            ),
            (
                "alt_query_2",
                if alt_query_2.is_empty() {
                    unknown.clone()
                } else {
                    alt_query_2
                },
            ),
            (
                "answer",
                if answer.is_empty() { no_answer } else { answer },
            ),
        ],
    )
}

async fn ask_ollama_vision_with_model(
    client: &Client,
    model: &str,
    image_b64: &str,
    prompt: &str,
) -> Result<OllamaChatResponse, String> {
    let body = json!({
        "model": model,
        "stream": false,
        "keep_alive": "10m",
        "messages": [
            {
                "role": "system",
                "content": "You are a desktop screenshot assistant. Be precise, useful, and concise."
            },
            {
                "role": "user",
                "content": prompt,
                "images": [image_b64]
            }
        ]
    });

    let response = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&body)
        .send()
        .await
        .map_err(|e: reqwest::Error| {
            reply_with("vision_call_failed", &[("error", e.to_string())])
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama Vision Fehler {}: {}", status, text));
    }

    response
        .json::<OllamaChatResponse>()
        .await
        .map_err(|e: reqwest::Error| {
            reply_with("vision_response_read_failed", &[("error", e.to_string())])
        })
}

pub async fn ask_ollama_vision(prompt: &str, image_path: &str) -> Result<OllamaResult, String> {
    let bytes = fs::read(image_path)
        .map_err(|e| reply_with("vision_file_read_failed", &[("error", e.to_string())]))?;

    let image_b64 = BASE64.encode(bytes);

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| reply_with("vision_client_init_failed", &[("error", e.to_string())]))?;

    let preferred_model = default_vision_model();

    let mut candidate_models: Vec<String> = vec![preferred_model];

    for fallback in [
        "gemma3:4b",
        "gemma3",
        "llama3.2-vision",
        "qwen2.5vl:7b",
        "qwen2.5vl",
    ] {
        if !candidate_models.iter().any(|m| m == fallback) {
            candidate_models.push(fallback.to_string());
        }
    }

    let mut attempted_models: Vec<String> = Vec::new();
    let mut not_found_errors: Vec<String> = Vec::new();

    for model in candidate_models {
        attempted_models.push(model.clone());

        match ask_ollama_vision_with_model(&client, &model, &image_b64, prompt).await {
            Ok(parsed) => {
                return Ok(OllamaResult {
                    content: parsed.message.content,
                    model: parsed.model,
                });
            }
            Err(err) => {
                let lower = err.to_lowercase();

                let is_missing_model = lower.contains("not found")
                    || lower.contains("unknown model")
                    || lower.contains("model")
                    || lower.contains("pull");

                if is_missing_model {
                    not_found_errors.push(format!("{} -> {}", model, err));
                    continue;
                }

                return Err(reply_with(
                    "vision_model_failed",
                    &[
                        ("model", model),
                        ("error", err),
                    ],
                ));
            }
        }
    }

    Err(reply_with(
        "vision_no_model_found",
        &[
            ("models", attempted_models.join("\n- ")),
            (
                "errors",
                if not_found_errors.is_empty() {
                    "No additional details available.".to_string()
                } else {
                    not_found_errors.join("\n")
                },
            ),
        ],
    ))
}

pub async fn analyze_snip(
    mode: String,
    comment: String,
    image_path: String,
    app_name: Option<String>,
    window_title: Option<String>,
) -> Result<String, String> {
    if image_path.trim().is_empty() {
        return Err(reply("snip_no_file"));
    }

    if !std::path::Path::new(&image_path).exists() {
        return Err(reply_with(
            "snip_file_not_found",
            &[("path", image_path.clone())],
        ));
    }

    let mut resolved_app_name = app_name.unwrap_or_else(|| reply("snip_unknown"));
    let resolved_window_title = window_title.unwrap_or_default();

    if is_internal_companion_app(&resolved_app_name) {
        let remembered = app_open_runtime::get_last_external_app();
        if is_useful_external_app(&remembered) {
            resolved_app_name = remembered;
        } else {
            resolved_app_name = reply("snip_unknown");
        }
    }

    let prompt = if mode == "search" {
        build_snip_search_prompt(&comment, &resolved_app_name, &resolved_window_title)
    } else {
        build_snip_vision_prompt(&mode, &comment)
    };

    let result = ask_ollama_vision(&prompt, &image_path).await?;

    if mode == "search" {
        return Ok(format_search_result(&result.content));
    }

    Ok(result.content)
}