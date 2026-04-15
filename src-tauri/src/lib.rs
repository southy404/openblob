use device_query::{DeviceQuery, DeviceState};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use tauri::{Emitter, Manager};
use window_vibrancy::{apply_blur, NSVisualEffectMaterial};

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
    pub mod tts;
    pub mod voice;
    pub mod windows_discovery;
}

use crate::core::app::run_command_pipeline;
use modules::companion::bonding::load_or_create_bonding_state;
use modules::companion::personality::{load_or_create_personality_state, load_personality_state};
use modules::context::{is_internal_companion_app, resolve_active_context};
use modules::memory::semantic_memory::load_or_create_semantic_memory;
use modules::profile::companion_config::{
    load_or_create_companion_config,
    save_companion_config,
};
use modules::profile::onboarding_state::load_or_create_onboarding_state;
use modules::profile::user_profile::{
    load_or_create_user_profile,
    save_user_profile,
};

fn initialize_companion_persistence() -> Result<(), String> {
    let _config = load_or_create_companion_config()?;
    let _onboarding = load_or_create_onboarding_state()?;
    let _personality = load_or_create_personality_state()?;
    let _bonding = load_or_create_bonding_state()?;
    let _user_profile = load_or_create_user_profile()?;
    let _semantic_memory = load_or_create_semantic_memory()?;
    Ok(())
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
    crate::core::legacy::browser_runtime::youtube_play_best_match(title).await
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
fn update_identity(
    blob_name: String,
    owner_name: String,
    language: String,
) -> Result<(), String> {
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

                    let is_toggle =
                        shortcut_str == "control+space" || shortcut_str == "ctrl+space";

                    let is_voice = shortcut_str == "alt+keym" || shortcut_str == "alt+m";

                    if is_toggle {
                        let _ = app.emit("companion-toggle", ());
                        return;
                    }

                    if is_voice {
                        let _ = app.emit("companion-voice-toggle", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;

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

            let shortcut = app.global_shortcut();

            if !shortcut.is_registered("Ctrl+Space") {
                shortcut.register("Ctrl+Space")?;
            }

            if !shortcut.is_registered("Alt+M") {
                shortcut.register("Alt+M")?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping_ollama,
            ask_ollama,
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