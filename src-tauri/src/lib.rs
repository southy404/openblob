use device_query::{DeviceQuery, DeviceState};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use window_vibrancy::{apply_blur, NSVisualEffectMaterial};

static TRANSCRIPT_RUNTIME: Lazy<
    Mutex<Option<modules::transcript::runtime::TranscriptRuntimeHandle>>,
> = Lazy::new(|| Mutex::new(None));

mod core;
mod modules {
    pub mod app_profiles;
    pub mod browser_automations;
    pub mod command_router;
    pub mod companion;
    pub mod context;
    pub mod i18n;
    pub mod memory;
    pub mod profile;
    pub mod screen_capture;
    pub mod session_memory;
    pub mod snip_session;
    pub mod snippets;
    pub mod steam_games;
    pub mod storage;
    pub mod streaming;
    pub mod system;
    pub mod transcript;
    pub mod tts;
    pub mod voice;
    pub mod wake_word;
    pub mod windows_discovery;
}

use crate::core::app::run_command_pipeline;
use crate::modules::memory::context::{
    build_memory_context_for_query_and_context_async, ActiveMemoryContext,
};
use crate::modules::memory::embeddings::backfill_missing_event_embeddings;
use crate::modules::memory::episodic_memory::{append_episode, EpisodicMemoryEntry};
use crate::modules::memory::events::{
    privacy_tier_for_kind, MemoryEvent, MemoryEventKind, PrivacyTier,
};
use crate::modules::memory::import::{import_legacy_episodic_memory, import_legacy_semantic_facts};
use crate::modules::memory::management::{
    export_memory_json, forget_memory as forget_memory_store, wipe_memory as wipe_memory_store,
    MemoryMutationReport,
};
use crate::modules::memory::reflection::{
    reflect_memory as reflect_memory_store, ReflectiveSummary,
};
use crate::modules::memory::retention::apply_memory_retention as apply_memory_retention_store;
use crate::modules::memory::writer::{
    clear_memory_private_mode as clear_memory_private_mode_store, enable_memory_private_mode,
    enqueue_memory_event, memory_private_mode_until, start_memory_writer,
};
use modules::companion::bonding::load_or_create_bonding_state;
use modules::companion::personality::{load_or_create_personality_state, load_personality_state};
use modules::context::{is_internal_companion_app, resolve_active_context};
use modules::memory::semantic_memory::load_or_create_semantic_memory;
use modules::profile::companion_config::{load_or_create_companion_config, save_companion_config};
use modules::profile::onboarding_state::load_or_create_onboarding_state;
use modules::profile::user_profile::{load_or_create_user_profile, save_user_profile};

fn initialize_companion_persistence() -> Result<(), String> {
    let config = load_or_create_companion_config()?;
    let _onboarding = load_or_create_onboarding_state()?;
    let _personality = load_or_create_personality_state()?;
    let _bonding = load_or_create_bonding_state()?;
    let _user_profile = load_or_create_user_profile()?;
    let _semantic_memory = load_or_create_semantic_memory()?;
    let report = import_legacy_episodic_memory()?;
    if report.imported > 0 || report.skipped > 0 {
        println!(
            "[openblob] Imported {} legacy memory events into SQLite ({} skipped)",
            report.imported, report.skipped
        );
    }
    let semantic_report = import_legacy_semantic_facts()?;
    if semantic_report.imported > 0 || semantic_report.skipped > 0 {
        println!(
            "[openblob] Imported {} legacy semantic facts into SQLite ({} skipped)",
            semantic_report.imported, semantic_report.skipped
        );
    }
    let retention_report = apply_memory_retention_store(&config.memory)?;
    if retention_report.events > 0
        || retention_report.facts > 0
        || retention_report.summaries > 0
        || retention_report.embeddings > 0
    {
        println!(
            "[openblob] Applied memory retention: events={}, facts={}, summaries={}, embeddings={}",
            retention_report.events,
            retention_report.facts,
            retention_report.summaries,
            retention_report.embeddings
        );
    }
    Ok(())
}

#[tauri::command]
fn start_transcript(
    app: tauri::AppHandle,
    source: String,
    app_name: Option<String>,
    window_title: Option<String>,
) -> Result<modules::transcript::types::TranscriptSession, String> {
    use modules::transcript::types::{StartTranscriptRequest, TranscriptSourceKind};

    let source = match source.trim().to_lowercase().as_str() {
        "system" | "system_audio" => TranscriptSourceKind::SystemAudio,
        "microphone" | "mic" => TranscriptSourceKind::Microphone,
        "mixed" => TranscriptSourceKind::Mixed,
        _ => return Err("Unsupported transcript source".into()),
    };

    {
        let guard = TRANSCRIPT_RUNTIME
            .lock()
            .map_err(|_| "Failed to lock transcript runtime".to_string())?;

        if guard.is_some() {
            return Err("Transcript is already running".into());
        }
    }

    let session = modules::transcript::session::start_session(StartTranscriptRequest {
        source: source.clone(),
        app_name,
        window_title,
    })?;

    let runtime = modules::transcript::runtime::start_runtime(app, session.clone())?;

    {
        let mut guard = TRANSCRIPT_RUNTIME
            .lock()
            .map_err(|_| "Failed to lock transcript runtime".to_string())?;
        *guard = Some(runtime);
    }

    Ok(session)
}

#[tauri::command]
fn stop_transcript() -> Result<modules::transcript::types::TranscriptSession, String> {
    {
        let mut guard = TRANSCRIPT_RUNTIME
            .lock()
            .map_err(|_| "Failed to lock transcript runtime".to_string())?;

        if let Some(runtime) = guard.take() {
            runtime.stop()?;
        } else {
            return Err("No active transcript runtime".into());
        }
    }

    let _ = modules::transcript::session::stop_session()?;
    let session = modules::transcript::session::finish_session()?;
    let _ = modules::transcript::transcript_store::save_session(&session);

    Ok(session)
}

#[tauri::command]
fn get_transcript_status() -> Result<modules::transcript::types::TranscriptStatus, String> {
    modules::transcript::session::get_status()
}

#[tauri::command]
fn get_current_transcript() -> Result<Option<modules::transcript::types::TranscriptSession>, String>
{
    modules::transcript::session::get_active_session()
}

#[tauri::command]
fn save_current_transcript() -> Result<String, String> {
    let session = modules::transcript::session::get_active_session()?
        .ok_or_else(|| "No active transcript session".to_string())?;

    let dir = modules::transcript::transcript_store::save_session(&session)?;
    Ok(dir.display().to_string())
}

#[tauri::command]
fn summarize_current_transcript() -> Result<modules::transcript::types::TranscriptSummary, String> {
    let session = modules::transcript::session::get_active_session()?
        .ok_or_else(|| "No active transcript session".to_string())?;

    Ok(modules::transcript::summary::summarize_session(&session))
}

#[tauri::command]
async fn process_transcript(
) -> Result<modules::transcript::types::ProcessedTranscriptResult, String> {
    modules::transcript::processor::process_best_available_transcript().await
}

#[tauri::command]
fn apply_glass_effect(window: tauri::Window) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        window_vibrancy::apply_vibrancy(
            &window,
            window_vibrancy::NSVisualEffectMaterial::UnderWindowBackground,
            None,
            None,
        )
        .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        apply_blur(&window, Some((0, 0, 0, 0))).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn capture_snip_region(x: i32, y: i32, width: u32, height: u32) -> Result<String, String> {
    modules::screen_capture::capture_region_to_file(x, y, width, height)
}

#[tauri::command]
async fn snip_file_exists(path: String) -> Result<bool, String> {
    Ok(modules::screen_capture::file_exists(&path))
}

#[tauri::command]
fn get_active_snip_context() -> crate::core::legacy::snip_runtime::ActiveSnipContext {
    crate::core::legacy::snip_runtime::resolve_snip_context()
}

#[tauri::command]
async fn create_snip(comment: Option<String>) -> Result<String, String> {
    crate::core::legacy::snip_runtime::create_snip(comment)
}

#[tauri::command]
async fn analyze_snip(
    mode: String,
    comment: String,
    image_path: String,
    app_name: Option<String>,
    window_title: Option<String>,
) -> Result<String, String> {
    crate::core::legacy::snip_runtime::analyze_snip(
        mode,
        comment,
        image_path,
        app_name,
        window_title,
    )
    .await
}

#[tauri::command]
async fn ping_ollama() -> Result<bool, String> {
    crate::core::legacy::ollama_text_runtime::ping_ollama().await
}

#[tauri::command]
async fn ask_ollama(
    mode: String,
    text: String,
    question: Option<String>,
    model: Option<String>,
) -> Result<crate::core::legacy::ollama_text_runtime::OllamaTextResult, String> {
    crate::core::legacy::ollama_text_runtime::ask_ollama(mode, text, question, model).await
}

#[tauri::command]
fn export_memory() -> Result<String, String> {
    export_memory_json()
}

#[tauri::command]
fn forget_memory(query: String) -> Result<MemoryMutationReport, String> {
    forget_memory_store(&query)
}

#[tauri::command]
fn wipe_memory() -> Result<MemoryMutationReport, String> {
    wipe_memory_store()
}

#[tauri::command]
fn apply_memory_retention() -> Result<MemoryMutationReport, String> {
    let config = load_or_create_companion_config()?;
    apply_memory_retention_store(&config.memory)
}

#[tauri::command]
async fn reflect_memory(scope: String) -> Result<ReflectiveSummary, String> {
    reflect_memory_store(&scope).await
}

#[tauri::command]
async fn backfill_memory_embeddings(limit: Option<usize>) -> Result<usize, String> {
    backfill_missing_event_embeddings(limit.unwrap_or(100).clamp(1, 1_000)).await
}

#[tauri::command]
fn set_memory_private_mode(minutes: Option<u32>) -> Result<String, String> {
    Ok(enable_memory_private_mode(minutes.unwrap_or(30)))
}

#[tauri::command]
fn clear_memory_private_mode() -> Result<(), String> {
    clear_memory_private_mode_store();
    Ok(())
}

#[tauri::command]
fn get_memory_private_mode() -> Option<String> {
    memory_private_mode_until()
}

#[tauri::command]
async fn trigger_copy_shortcut() -> Result<(), String> {
    crate::core::legacy::input_runtime::trigger_copy_shortcut()
}

#[tauri::command]
async fn browser_list_tabs() -> Result<Vec<modules::browser_automations::BrowserTab>, String> {
    crate::core::legacy::browser_runtime::ensure_debug_browser().await?;
    modules::browser_automations::list_tabs().await
}

#[tauri::command]
async fn browser_close_tab_by_index(index: usize) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_close_tab_by_index(index).await
}

#[tauri::command]
async fn browser_new_tab(url: String) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_new_tab_with_url(url).await
}

#[tauri::command]
async fn browser_click_link_by_text(text: String, new_tab: bool) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_click_link_by_text(text, new_tab).await
}

#[tauri::command]
async fn browser_click_first_result() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_click_first_result().await
}

#[tauri::command]
async fn browser_click_nth_result(index: usize) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_click_nth_result(index).await
}

#[tauri::command]
async fn browser_open_url(
    url: String,
    new_tab: bool,
    new_window: bool,
    incognito: bool,
) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_open_url(url, new_tab, new_window, incognito)
        .await
}

#[tauri::command]
async fn browser_get_context() -> Result<modules::browser_automations::BrowserContext, String> {
    crate::core::legacy::browser_runtime::browser_get_context().await
}

#[tauri::command]
async fn browser_back() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_back().await
}

#[tauri::command]
async fn browser_forward() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_forward().await
}

#[tauri::command]
async fn browser_scroll_down() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_scroll_down().await
}

#[tauri::command]
async fn browser_scroll_up() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_scroll_up().await
}

#[tauri::command]
async fn browser_type_text(text: String) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_type_text(text).await
}

#[tauri::command]
async fn browser_submit() -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_submit().await
}

#[tauri::command]
async fn browser_click_best_match(text: String) -> Result<String, String> {
    crate::core::legacy::browser_runtime::browser_click_best_match(text).await
}

#[tauri::command]
async fn youtube_search_and_play(query: String) -> Result<String, String> {
    crate::core::legacy::browser_runtime::youtube_search_and_play(query).await
}

#[tauri::command]
async fn youtube_play_title(title: String) -> Result<String, String> {
    crate::core::legacy::browser_runtime::youtube_search_and_play(title).await
}

#[tauri::command]
fn get_cursor_position() -> (i32, i32) {
    let device_state = DeviceState::new();
    let mouse = device_state.get_mouse();
    mouse.coords
}

#[tauri::command]
fn get_active_app() -> String {
    let ctx = resolve_active_context();
    let app = ctx.app_name.clone();

    if !is_internal_companion_app(&app) {
        crate::core::legacy::app_open_runtime::remember_external_app(&app);
        return app;
    }

    let last = crate::core::legacy::app_open_runtime::get_last_external_app();
    if last != "unknown" {
        return last;
    }

    app
}

#[tauri::command]
fn get_identity() -> Result<(String, String, String), String> {
    let config = load_or_create_companion_config()?;
    let profile = load_or_create_user_profile()?;

    Ok((
        config.blob_name,
        profile.display_name.unwrap_or_default(),
        config.preferred_language,
    ))
}

#[tauri::command]
fn update_identity(blob_name: String, owner_name: String, language: String) -> Result<(), String> {
    let mut config = load_or_create_companion_config()?;
    let mut profile = load_or_create_user_profile()?;

    config.blob_name = blob_name.trim().to_string();
    config.preferred_language = language.trim().to_lowercase();

    profile.display_name = if owner_name.trim().is_empty() {
        None
    } else {
        Some(owner_name.trim().to_string())
    };

    save_companion_config(&config)?;
    save_user_profile(&profile)?;

    Ok(())
}

#[tauri::command]
async fn handle_voice_command(app: tauri::AppHandle, input: String) -> Result<String, String> {
    modules::session_memory::set_last_command(&input);

    let personality = load_personality_state().unwrap_or_default();
    let _personality_mood_hint = personality.mood_hint();

    let mut ctx = resolve_active_context();
    let active_app = get_active_app();

    let normalized = input
        .trim()
        .to_lowercase()
        .replace('?', "")
        .replace('!', "")
        .replace('.', "")
        .replace(',', "");

    let asks_blob_name = normalized.contains("what is your name")
        || normalized.contains("whats your name")
        || normalized.contains("what's your name")
        || normalized.contains("who are you")
        || normalized.contains("tell me your name")
        || normalized.contains("wie heißt du")
        || normalized.contains("wer bist du");

    if asks_blob_name {
        let config = load_or_create_companion_config()?;
        let blob_name = if config.blob_name.trim().is_empty() {
            "OpenBlob".to_string()
        } else {
            config.blob_name.trim().to_string()
        };

        return Ok(format!("My name is {}.", blob_name));
    }

    let asks_owner_name = normalized.contains("what is my name")
        || normalized.contains("whats my name")
        || normalized.contains("what's my name")
        || normalized.contains("who am i")
        || normalized.contains("do you know my name")
        || normalized.contains("wie heiße ich")
        || normalized.contains("wer bin ich");

    if asks_owner_name {
        let profile = load_or_create_user_profile()?;
        let owner_name = profile
            .display_name
            .clone()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "Owner".to_string());

        return Ok(format!("Your name is {}.", owner_name));
    }

    if normalized.contains("forget everything from today")
        || normalized.contains("forget today")
        || normalized.contains("vergiss alles von heute")
    {
        let report = forget_memory_store("today")?;
        return Ok(format!(
            "Forgot today's memory entries: {} events and {} summaries.",
            report.events, report.summaries
        ));
    }

    if let Some(topic) = normalized
        .strip_prefix("forget about ")
        .or_else(|| normalized.strip_prefix("forget memory about "))
    {
        let report = forget_memory_store(topic)?;
        return Ok(format!(
            "Forgot matching memory entries for '{}': {} events, {} facts, {} summaries.",
            topic, report.events, report.facts, report.summaries
        ));
    }

    if is_internal_companion_app(&ctx.app_name) && active_app != "unknown" {
        ctx.app_name = active_app.clone();

        if ctx.domain == "companion" || ctx.domain == "desktop" {
            ctx.domain = "browser".to_string();
        }

        if ctx.window_title.trim().is_empty() || is_internal_companion_app(&ctx.window_title) {
            ctx.window_title = active_app.clone();
        }
    }

    let pipeline = run_command_pipeline(&app, &input, &ctx).await?;

    Ok(match pipeline.result {
        Some(result) => result.message,
        None => "NO_ACTION".to_string(),
    })
}

#[tauri::command]
async fn speak_text(text: String, lang: Option<String>) -> Result<(), String> {
    modules::tts::manager::speak(&text, lang.as_deref()).await
}

#[tauri::command]
async fn stop_tts() -> Result<(), String> {
    modules::tts::manager::stop().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;

                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    let shortcut_str = shortcut.to_string().replace(' ', "").to_lowercase();
                    println!("global shortcut pressed: {}", shortcut_str);

                    let is_toggle = shortcut_str == "control+space" || shortcut_str == "ctrl+space";

                    let is_voice = shortcut_str == "alt+keym" || shortcut_str == "alt+m";
                    let is_private_memory = shortcut_str == "control+shift+keym"
                        || shortcut_str == "ctrl+shift+keym"
                        || shortcut_str == "control+shift+m"
                        || shortcut_str == "ctrl+shift+m";

                    if is_toggle {
                        let _ = app.emit("companion-toggle", ());
                        return;
                    }

                    if is_private_memory {
                        let until = enable_memory_private_mode(30);
                        let _ = app.emit("memory-private-mode-enabled", until);
                        return;
                    }

                    if is_voice {
                        let _ = app.emit("companion-voice-toggle", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            use axum::{
                extract::{Query, State},
                routing::{get, post},
                Json, Router,
            };
            use serde::{Deserialize, Serialize};
            use serde_json::{json, Value};
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

            #[derive(Clone)]
            struct ExternalCommandState {
                app: tauri::AppHandle,
            }

            #[derive(Deserialize)]
            struct CommandRequest {
                input: String,
                channel: Option<String>,
            }

            #[derive(Serialize)]
            struct CommandResponse {
                result: String,
                action_taken: bool,
            }

            #[derive(Deserialize)]
            struct MemoryContextQuery {
                q: Option<String>,
                app: Option<String>,
                domain: Option<String>,
                limit: Option<usize>,
            }

            #[derive(Serialize)]
            struct MemoryContextResponse {
                memory: String,
                event_count: usize,
                error: Option<String>,
            }

            #[derive(Deserialize)]
            struct MemoryEventRequest {
                kind: Option<String>,
                source: Option<String>,
                channel: Option<String>,
                app_name: Option<String>,
                domain: Option<String>,
                user_input: Option<String>,
                summary: Option<String>,
                outcome: Option<String>,
                importance: Option<f32>,
                privacy_tier: Option<String>,
                metadata: Option<Value>,
            }

            #[derive(Serialize)]
            struct MemoryEventResponse {
                queued: bool,
                result: String,
            }

            async fn handle_external_command(
                State(state): State<ExternalCommandState>,
                Json(payload): Json<CommandRequest>,
            ) -> Json<CommandResponse> {
                let input = payload.input.trim().to_string();

                if input.is_empty() {
                    return Json(CommandResponse {
                        result: "NO_ACTION".to_string(),
                        action_taken: false,
                    });
                }

                let mut ctx = resolve_active_context();
                let active_app = get_active_app();

                if is_internal_companion_app(&ctx.app_name) && active_app != "unknown" {
                    ctx.app_name = active_app.clone();

                    if ctx.domain == "companion" || ctx.domain == "desktop" {
                        ctx.domain = "browser".to_string();
                    }

                    if ctx.window_title.trim().is_empty()
                        || is_internal_companion_app(&ctx.window_title)
                    {
                        ctx.window_title = active_app;
                    }
                }

                match run_command_pipeline(&state.app, &input, &ctx).await {
                    Ok(pipeline) => {
                        let mut result_msg = pipeline
                            .result
                            .map(|r| r.message)
                            .unwrap_or_else(|| "NO_ACTION".to_string());
                        let action_taken = result_msg != "NO_ACTION";

                        if !action_taken {
                            match crate::core::legacy::ollama_text_runtime::ask_ollama(
                                "chat".to_string(),
                                input.clone(),
                                Some(input.clone()),
                                None,
                            )
                            .await
                            {
                                Ok(answer) => {
                                    result_msg = answer.content;
                                }
                                Err(err) => {
                                    result_msg = format!("ERROR: {}", err);
                                }
                            }
                        }

                        // Episode speichern
                        if result_msg != "NO_ACTION" {
                            let channel = payload.channel.unwrap_or_else(|| "unknown".to_string());
                            let entry = EpisodicMemoryEntry::new(
                                "external_command",
                                &channel,
                                "external",
                                &input,
                                &result_msg,
                                "success",
                                0.6,
                            );
                            let _ = append_episode(&entry);

                            if let Ok(config) = load_or_create_companion_config() {
                                let event = MemoryEvent::successful_connector_command(
                                    channel,
                                    &input,
                                    &result_msg,
                                    "success",
                                    &config.privacy,
                                );
                                let _ = enqueue_memory_event(event);
                            }
                        }

                        Json(CommandResponse {
                            action_taken,
                            result: result_msg,
                        })
                    }
                    Err(err) => Json(CommandResponse {
                        result: format!("ERROR: {}", err),
                        action_taken: false,
                    }),
                }
            }

            async fn handle_memory_context(
                Query(query): Query<MemoryContextQuery>,
            ) -> Json<MemoryContextResponse> {
                let active_context = if query.app.is_some() || query.domain.is_some() {
                    Some(ActiveMemoryContext {
                        app_name: query.app,
                        context_domain: query.domain,
                    })
                } else {
                    None
                };

                match build_memory_context_for_query_and_context_async(
                    query.q.as_deref(),
                    active_context,
                    query.limit,
                )
                .await
                {
                    Ok(context) => Json(MemoryContextResponse {
                        memory: context.memory,
                        event_count: context.event_count,
                        error: None,
                    }),
                    Err(err) => Json(MemoryContextResponse {
                        memory: String::new(),
                        event_count: 0,
                        error: Some(err),
                    }),
                }
            }

            async fn handle_memory_event(
                Json(payload): Json<MemoryEventRequest>,
            ) -> Json<MemoryEventResponse> {
                let kind = MemoryEventKind::from_str(
                    payload.kind.as_deref().unwrap_or("connector_message"),
                );
                let source = payload
                    .source
                    .or(payload.channel.clone())
                    .unwrap_or_else(|| "external".to_string());
                let configured_tier = load_or_create_companion_config()
                    .map(|config| privacy_tier_for_kind(kind, &config.privacy))
                    .unwrap_or(PrivacyTier::Redacted);
                let requested_tier = payload.privacy_tier.as_deref().map(PrivacyTier::from_str);
                let privacy_tier = match requested_tier {
                    Some(PrivacyTier::Transient) => PrivacyTier::Transient,
                    Some(PrivacyTier::MetadataOnly)
                        if configured_tier != PrivacyTier::Transient =>
                    {
                        PrivacyTier::MetadataOnly
                    }
                    Some(PrivacyTier::Full) if configured_tier == PrivacyTier::Full => {
                        PrivacyTier::Full
                    }
                    _ => configured_tier,
                };

                let mut event = MemoryEvent::new(kind, source, privacy_tier)
                    .with_importance(payload.importance.unwrap_or(0.5))
                    .with_metadata(payload.metadata.unwrap_or_else(|| json!({})));

                if let Some(app_name) = payload.app_name.or(payload.channel) {
                    event = event.with_app_name(app_name);
                }
                if let Some(domain) = payload.domain {
                    event = event.with_context_domain(domain);
                }
                if let Some(user_input) = payload.user_input {
                    event = event.with_user_input(user_input);
                }
                if let Some(summary) = payload.summary {
                    event = event.with_summary(summary);
                }
                if let Some(outcome) = payload.outcome {
                    event = event.with_outcome(outcome);
                }

                let result = enqueue_memory_event(event);

                Json(MemoryEventResponse {
                    queued: matches!(
                        result,
                        crate::modules::memory::writer::EnqueueMemoryEventResult::Queued
                    ),
                    result: format!("{result:?}"),
                })
            }

            let app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                let router = Router::new()
                    .route("/command", post(handle_external_command))
                    .route("/memory/context", get(handle_memory_context))
                    .route("/memory/event", post(handle_memory_event))
                    .with_state(ExternalCommandState { app: app_handle });

                let listener = tokio::net::TcpListener::bind("127.0.0.1:7842")
                    .await
                    .expect("Konnte lokalen Command-Server nicht starten");

                println!("[openblob] Command-Server läuft auf http://127.0.0.1:7842");

                if let Err(err) = axum::serve(listener, router).await {
                    eprintln!("[openblob] Command-Server Fehler: {}", err);
                }
            });

            if let Some(window) = app.get_webview_window("bubble") {
                #[cfg(target_os = "macos")]
                {
                    let _ = window_vibrancy::apply_vibrancy(
                        &window,
                        NSVisualEffectMaterial::UnderWindowBackground,
                        None,
                        None,
                    );
                }

                #[cfg(target_os = "windows")]
                {
                    let _ = window_vibrancy::apply_acrylic(&window, Some((18, 18, 18, 125)));
                }
            }

            if let Some(window) = app.get_webview_window("bubble-dev") {
                #[cfg(target_os = "macos")]
                {
                    let _ = window_vibrancy::apply_vibrancy(
                        &window,
                        NSVisualEffectMaterial::UnderWindowBackground,
                        None,
                        None,
                    );
                }

                #[cfg(target_os = "windows")]
                {
                    let _ = window_vibrancy::apply_blur(&window, Some((18, 18, 18, 125)));
                }
            }

            if let Err(err) = modules::i18n::command_locale::init_command_locale("en") {
                eprintln!("Failed to initialize command locale: {err}");
            }

            if let Err(err) = initialize_companion_persistence() {
                eprintln!("Failed to initialize companion persistence: {err}");
            }

            if let Err(err) = start_memory_writer() {
                eprintln!("Failed to start memory writer: {err}");
            }

            tauri::async_runtime::spawn(async {
                if let Err(err) = reflect_memory_store("session").await {
                    eprintln!("Failed to refresh session memory reflection: {err}");
                }
                if let Err(err) = reflect_memory_store("daily").await {
                    eprintln!("Failed to refresh daily memory reflection: {err}");
                }
                if let Err(err) = reflect_memory_store("weekly").await {
                    eprintln!("Failed to refresh weekly memory reflection: {err}");
                }
                if let Err(err) = backfill_missing_event_embeddings(20).await {
                    eprintln!("Failed to backfill memory embeddings: {err}");
                }
            });

            let shortcut = app.global_shortcut();

            if !shortcut.is_registered("Ctrl+Space") {
                shortcut.register("Ctrl+Space")?;
            }

            if !shortcut.is_registered("Alt+M") {
                shortcut.register("Alt+M")?;
            }

            if !shortcut.is_registered("Ctrl+Shift+M") {
                shortcut.register("Ctrl+Shift+M")?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping_ollama,
            ask_ollama,
            export_memory,
            forget_memory,
            wipe_memory,
            apply_memory_retention,
            reflect_memory,
            backfill_memory_embeddings,
            set_memory_private_mode,
            clear_memory_private_mode,
            get_memory_private_mode,
            trigger_copy_shortcut,
            get_cursor_position,
            get_active_app,
            get_active_snip_context,
            handle_voice_command,
            speak_text,
            apply_glass_effect,
            stop_tts,
            get_identity,
            update_identity,
            browser_list_tabs,
            browser_close_tab_by_index,
            browser_new_tab,
            browser_click_link_by_text,
            browser_click_first_result,
            browser_click_nth_result,
            browser_open_url,
            browser_get_context,
            browser_back,
            browser_forward,
            browser_scroll_down,
            browser_scroll_up,
            browser_type_text,
            browser_submit,
            browser_click_best_match,
            youtube_search_and_play,
            youtube_play_title,
            create_snip,
            analyze_snip,
            capture_snip_region,
            snip_file_exists,
            start_transcript,
            stop_transcript,
            get_transcript_status,
            get_current_transcript,
            save_current_transcript,
            summarize_current_transcript,
            process_transcript,
            modules::wake_word::get_wake_word_settings,
            modules::wake_word::update_wake_word_settings,
            modules::wake_word::start_wake_word_listener,
            modules::wake_word::stop_wake_word_listener,
            modules::wake_word::get_wake_word_status,
            modules::wake_word::get_wake_word_model_status,
            modules::wake_word::get_wake_word_install_status,
            modules::wake_word::verify_wake_word_runtime,
            modules::wake_word::verify_wake_word_model_bundle,
            modules::wake_word::list_wake_word_models,
            modules::wake_word::set_wake_word_model_path,
            modules::wake_word::set_wake_word_runtime_path,
            modules::wake_word::open_wake_word_model_folder,
            modules::wake_word::open_wake_word_runtime_folder,
            modules::wake_word::download_wake_word_runtime,
            modules::wake_word::download_wake_word_model_bundle,
            modules::system::get_system_volume,
            modules::system::set_system_volume,
            modules::system::change_system_volume,
            modules::system::get_system_mute,
            modules::system::set_system_mute,
            modules::system::toggle_system_mute,
            modules::system::media_play_pause,
            modules::system::media_next_track,
            modules::system::media_prev_track,
            modules::system::volume_key_up,
            modules::system::volume_key_down,
            modules::system::volume_key_mute,
            modules::voice::record_and_transcribe_voice
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
