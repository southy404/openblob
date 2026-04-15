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
use modules::snip_session::set_snip;

static LAST_EXTERNAL_APP: OnceLock<Mutex<String>> = OnceLock::new();
static ACTIVE_TIMER_ID: OnceLock<Mutex<u64>> = OnceLock::new();

fn active_timer_id_store() -> &'static Mutex<u64> {
    ACTIVE_TIMER_ID.get_or_init(|| Mutex::new(0))
}

fn next_timer_id() -> u64 {
    if let Ok(mut guard) = active_timer_id_store().lock() {
        *guard += 1;
        *guard
    } else {
        1
    }
}

fn current_timer_id() -> u64 {
    if let Ok(guard) = active_timer_id_store().lock() {
        *guard
    } else {
        0
    }
}

fn cancel_active_timer() {
    if let Ok(mut guard) = active_timer_id_store().lock() {
        *guard += 1;
    }
}

fn default_text_model() -> String {
    "llama3.1:8b".to_string()
}

fn default_vision_model() -> String {
    "gemma3".to_string()
}

fn last_external_app_store() -> &'static Mutex<String> {
    LAST_EXTERNAL_APP.get_or_init(|| Mutex::new(String::from("unknown")))
}

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

    match client.get("http://127.0.0.1:9222/json").send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    }
}

fn spawn_debug_browser() -> Result<(), String> {
    let chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
    let chrome_path_x86 = r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe";
    let edge_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
    let edge_path_alt = r"C:\Program Files\Microsoft\Edge\Application\msedge.exe";
    let user_data = r"D:\companion-browser";

    let chrome_candidates = [chrome_path, chrome_path_x86];
    for path in chrome_candidates {
        if std::path::Path::new(path).exists() {
            Command::new(path)
                .args([
                    "--remote-debugging-port=9222",
                    &format!("--user-data-dir={}", user_data),
                    "--no-first-run",
                    "--no-default-browser-check",
                    "https://www.google.com",
                ])
                .spawn()
                .map_err(|e| format!("Chrome konnte nicht gestartet werden: {e}"))?;
            return Ok(());
        }
    }

    let edge_candidates = [edge_path, edge_path_alt];
    for path in edge_candidates {
        if std::path::Path::new(path).exists() {
            Command::new(path)
                .args([
                    "--remote-debugging-port=9222",
                    &format!("--user-data-dir={}", user_data),
                    "--no-first-run",
                    "https://www.google.com",
                ])
                .spawn()
                .map_err(|e| format!("Edge konnte nicht gestartet werden: {e}"))?;
            return Ok(());
        }
    }

    Err("Kein Chrome oder Edge gefunden.".into())
}

async fn ensure_debug_browser() -> Result<(), String> {
    if is_debug_browser_running().await {
        return Ok(());
    }

    spawn_debug_browser()?;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    if is_debug_browser_running().await {
        Ok(())
    } else {
        Err("Debug-Browser konnte nicht gestartet werden.".into())
    }
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
async fn create_snip(comment: Option<String>) -> Result<String, String> {
    let context = resolve_active_context();

    let image_path = modules::screen_capture::capture_region_to_file(0, 0, 400, 300)?;

    let session = modules::snip_session::SnipSession {
        image_path: image_path.clone(),
        comment: comment.unwrap_or_default(),
        context_app: context.app_name,
        context_domain: context.domain,
        window_title: context.window_title,
    };

    set_snip(session);

    Ok(format!("Snip created: {}", image_path))
}

#[tauri::command]
async fn analyze_snip(
    mode: String,
    comment: String,
    image_path: String,
    app_name: Option<String>,
    window_title: Option<String>,
) -> Result<String, String> {
    println!(
        "[analyze_snip] mode={} image_path={} app_name={:?} window_title={:?}",
        mode, image_path, app_name, window_title
    );

    if image_path.trim().is_empty() {
        return Err("No snip found".into());
    }

    if !std::path::Path::new(&image_path).exists() {
        return Err(format!("Snip file not found: {}", image_path));
    }

    let mut resolved_app_name = app_name.unwrap_or_else(|| "unknown".to_string());
    let resolved_window_title = window_title.unwrap_or_default();

    if is_internal_companion_app(&resolved_app_name) {
        let remembered = get_last_external_app();
        if is_useful_external_app(&remembered) {
            resolved_app_name = remembered;
        } else {
            resolved_app_name = "unknown".to_string();
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

#[tauri::command]
async fn ping_ollama() -> Result<bool, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("http://127.0.0.1:11434/api/tags")
        .send()
        .await
        .map_err(|e| format!("Ollama nicht erreichbar: {}", e))?;

    Ok(res.status().is_success())
}

#[tauri::command]
async fn ask_ollama(
    mode: String,
    text: String,
    question: Option<String>,
    model: Option<String>,
) -> Result<OllamaResult, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Kein Text vorhanden.".into());
    }

    let chosen_model = model.unwrap_or_else(default_text_model);

    let body = json!({
        "model": chosen_model,
        "stream": false,
        "keep_alive": "10m",
        "messages": [
            { "role": "system", "content": system_prompt() },
            {
                "role": "user",
                "content": build_user_prompt(&mode, trimmed, question.as_deref())
            }
        ]
    });

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post("http://127.0.0.1:11434/api/chat")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Fehler beim Aufruf von Ollama: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Ollama Fehler {}: {}", status, text));
    }

    let parsed: OllamaChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Antwort konnte nicht gelesen werden: {}", e))?;

    Ok(OllamaResult {
        content: parsed.message.content,
        model: parsed.model,
    })
}

#[tauri::command]
async fn trigger_copy_shortcut() -> Result<(), String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;

    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('c'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn browser_list_tabs() -> Result<Vec<modules::browser_automations::BrowserTab>, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::list_tabs().await
}

#[tauri::command]
async fn browser_close_tab_by_index(index: usize) -> Result<String, String> {
    ensure_debug_browser().await?;
    let tabs: Vec<modules::browser_automations::BrowserTab> =
        modules::browser_automations::list_tabs().await?;

    let page_tabs: Vec<_> = tabs
        .into_iter()
        .filter(|t| t.tab_type.as_deref() == Some("page"))
        .collect();

    let tab = page_tabs
        .get(index)
        .ok_or_else(|| format!("Tab {} nicht gefunden.", index + 1))?;

    modules::browser_automations::close_tab(&tab.id).await?;
    Ok(format!("Tab {} geschlossen.", index + 1))
}

#[tauri::command]
async fn browser_new_tab(url: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::new_tab(&url).await?;
    Ok("Neuer Tab geöffnet.".into())
}

#[tauri::command]
async fn browser_click_link_by_text(text: String, new_tab: bool) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::click_link_by_text(&text, new_tab).await
}

#[tauri::command]
async fn browser_click_first_result() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::click_nth_result(0).await
}

#[tauri::command]
async fn browser_click_nth_result(index: usize) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::click_nth_result(index).await
}

#[tauri::command]
async fn browser_open_url(
    url: String,
    new_tab: bool,
    new_window: bool,
    incognito: bool,
) -> Result<String, String> {
    ensure_debug_browser().await?;

    if incognito {
        return Err(
            "Inkognito wird im gesteuerten Debug-Browser absichtlich nicht verwendet, damit derselbe Browser steuerbar bleibt."
                .into(),
        );
    }

    if new_window {
        modules::browser_automations::new_tab(&url).await?;
        return Ok("URL im gesteuerten Browser in neuem Tab geöffnet.".into());
    }

    if new_tab {
        modules::browser_automations::new_tab(&url).await?;
        return Ok("URL in neuem Tab geöffnet.".into());
    }

    modules::browser_automations::navigate_best_tab(&url).await?;
    Ok("URL im gesteuerten Browser geöffnet.".into())
}

#[tauri::command]
async fn browser_get_context() -> Result<modules::browser_automations::BrowserContext, String> {
    ensure_debug_browser().await?;
    let ctx = modules::browser_automations::get_browser_context().await?;
    modules::session_memory::set_browser_context(&ctx.url, &ctx.title, &ctx.page_kind);
    Ok(ctx)
}

#[tauri::command]
async fn browser_back() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::navigate_back().await
}

#[tauri::command]
async fn browser_forward() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::navigate_forward().await
}

#[tauri::command]
async fn browser_scroll_down() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::scroll_by(700).await
}

#[tauri::command]
async fn browser_scroll_up() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::scroll_by(-700).await
}

#[tauri::command]
async fn browser_type_text(text: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::session_memory::set_last_search_query(&text);
    modules::browser_automations::type_in_best_input(&text).await
}

#[tauri::command]
async fn browser_submit() -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::submit_best_form().await
}

#[tauri::command]
async fn browser_click_best_match(text: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::session_memory::set_last_clicked_label(&text);
    modules::browser_automations::click_best_match(&text).await
}

#[tauri::command]
async fn youtube_search_and_play(query: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::youtube_search(&query, false).await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    modules::browser_automations::youtube_play_best_match(&query).await
}

#[tauri::command]
async fn youtube_play_title(title: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    modules::browser_automations::youtube_play_best_match(&title).await
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
        remember_external_app(&app);
        return app;
    }

    let last = get_last_external_app();
    if last != "unknown" {
        return last;
    }

    app
}

fn command_exists_windows(command: &str) -> bool {
    Command::new("where")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn spawn_hidden_cmd(args: &[&str]) -> Result<(), String> {
    Command::new("cmd")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Konnte nicht öffnen: {e}"))?;
    Ok(())
}

fn open_url_prefer_browser(url: &str, new_window: bool, incognito: bool) -> Result<(), String> {
    let chrome_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    for chrome_path in chrome_paths {
        if std::path::Path::new(chrome_path).exists() {
            let mut cmd = Command::new(chrome_path);

            if incognito {
                cmd.arg("--incognito");
            }

            if new_window {
                cmd.arg("--new-window");
            }

            cmd.arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Could not open Chrome: {e}"))?;

            return Ok(());
        }
    }

    let edge_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    for edge_path in edge_paths {
        if std::path::Path::new(edge_path).exists() {
            let mut cmd = Command::new(edge_path);

            if incognito {
                cmd.arg("-inprivate");
            }

            if new_window {
                cmd.arg("--new-window");
            }

            cmd.arg(url)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Could not open Edge: {e}"))?;

            return Ok(());
        }
    }

    Command::new("explorer")
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not open URL: {e}"))?;

    Ok(())
}

fn known_web_fallback(target: &str) -> Option<&'static str> {
    match target {
        "discord" => Some("https://discord.com/app"),
        "spotify" => Some("https://open.spotify.com"),
        "youtube" => Some("https://www.youtube.com"),
        "google" => Some("https://www.google.com"),
        "gmail" => Some("https://mail.google.com"),
        "twitch" => Some("https://www.twitch.tv"),
        "x" => Some("https://x.com"),
        "twitter" => Some("https://x.com"),
        "reddit" => Some("https://www.reddit.com"),
        "github" => Some("https://github.com"),
        _ => None,
    }
}

fn open_known_local_target(target: &str) -> Result<bool, String> {
    let opened = match target {
        "steam" => {
            let candidates = [
                r"C:\Program Files (x86)\Steam\steam.exe",
                r"C:\Program Files\Steam\steam.exe",
                r"D:\Steam\steam.exe",
            ];

            if let Some(path) = candidates.iter().find(|p| std::path::Path::new(p).exists()) {
                spawn_hidden_cmd(&["/C", "start", "", path])?;
                true
            } else if command_exists_windows("steam") {
                spawn_hidden_cmd(&["/C", "start", "", "steam"])?;
                true
            } else {
                false
            }
        }

        "fl studio" | "fl" => {
            let candidates = [
                r"C:\Program Files\Image-Line\FL Studio 2024\FL64.exe",
                r"C:\Program Files\Image-Line\FL Studio 21\FL64.exe",
            ];

            if let Some(path) = candidates.iter().find(|p| std::path::Path::new(p).exists()) {
                spawn_hidden_cmd(&["/C", "start", "", path])?;
                true
            } else if command_exists_windows("FL64") {
                spawn_hidden_cmd(&["/C", "start", "", "FL64"])?;
                true
            } else {
                false
            }
        }

        "discord" => {
            if command_exists_windows("discord") {
                spawn_hidden_cmd(&["/C", "start", "", "discord"])?;
                true
            } else {
                false
            }
        }

        "spotify" => {
            if command_exists_windows("spotify") {
                spawn_hidden_cmd(&["/C", "start", "", "spotify"])?;
                true
            } else {
                false
            }
        }

        "chrome" => {
            if command_exists_windows("chrome") {
                spawn_hidden_cmd(&["/C", "start", "", "chrome"])?;
                true
            } else {
                false
            }
        }

        "edge" => {
            if command_exists_windows("msedge") {
                spawn_hidden_cmd(&["/C", "start", "", "msedge"])?;
                true
            } else {
                false
            }
        }

        "explorer" => {
            spawn_hidden_cmd(&["/C", "start", "", "explorer"])?;
            true
        }

        "notepad" => {
            spawn_hidden_cmd(&["/C", "start", "", "notepad"])?;
            true
        }

        "paint" => {
            spawn_hidden_cmd(&["/C", "start", "", "mspaint"])?;
            true
        }

        "calc" => {
            spawn_hidden_cmd(&["/C", "start", "", "calc"])?;
            true
        }

        "taskmgr" => {
            spawn_hidden_cmd(&["/C", "start", "", "taskmgr"])?;
            true
        }

        "settings" => {
            spawn_hidden_cmd(&["/C", "start", "", "ms-settings:"])?;
            true
        }

        _ => false,
    };

    Ok(opened)
}

fn open_app_target(target: &str, prefer_browser: bool) -> Result<String, String> {
    let normalized = target.trim().to_lowercase();

    if prefer_browser {
        if let Some(url) = known_web_fallback(&normalized) {
            open_url_prefer_browser(url, false, false)?;
            return Ok(format!("Opening {} in the browser.", target));
        }

        if normalized.contains('.')
            || normalized.starts_with("http://")
            || normalized.starts_with("https://")
        {
            let url = if normalized.starts_with("http://") || normalized.starts_with("https://") {
                normalized.clone()
            } else {
                format!("https://{}", normalized)
            };

            open_url_prefer_browser(&url, false, false)?;
            return Ok(format!("Opening {} in the browser.", target));
        }
    }

    if open_known_local_target(&normalized)? {
        return Ok(format!("Opening {}.", target));
    }

    if let Some(game) = modules::steam_games::find_steam_game(&normalized) {
        let uri = modules::steam_games::steam_launch_uri(&game.appid);
        spawn_hidden_cmd(&["/C", "start", "", &uri])?;
        return Ok(format!("Launching {} via Steam.", game.name));
    }

    if let Some(app) = modules::windows_discovery::find_app_launch_target(&normalized) {
        spawn_hidden_cmd(&["/C", "start", "", &app.launch_target])?;
        return Ok(format!("Opening {}.", app.canonical_name));
    }

    if command_exists_windows(&normalized) {
        spawn_hidden_cmd(&["/C", "start", "", &normalized])?;
        return Ok(format!("Opening {}.", target));
    }

    if let Some(url) = known_web_fallback(&normalized) {
        open_url_prefer_browser(url, false, false)?;
        return Ok(format!("{} was not found locally. Opening web version.", target));
    }

    Err(format!("I couldn't open '{}'.", target))
}

fn send_keys<F>(f: F) -> Result<String, String>
where
    F: FnOnce(&mut Enigo) -> Result<(), String>,
{
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;
    f(&mut enigo)?;
    Ok("OK".into())
}

fn shortcut_ctrl(key: char) -> Result<String, String> {
    send_keys(|enigo| {
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Unicode(key), Direction::Click)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn shortcut_ctrl_shift(key: char) -> Result<String, String> {
    send_keys(|enigo| {
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Shift, Direction::Press)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Unicode(key), Direction::Click)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Shift, Direction::Release)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn shortcut_alt_f4() -> Result<String, String> {
    send_keys(|enigo| {
        enigo
            .key(Key::Alt, Direction::Press)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::F4, Direction::Click)
            .map_err(|e| e.to_string())?;
        enigo
            .key(Key::Alt, Direction::Release)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn press_key(key: &str) -> Result<String, String> {
    send_keys(|enigo| {
        enigo
            .key(parse_key(key), Direction::Click)
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

fn press_key_combo(keys: &[&'static str]) -> Result<String, String> {
    send_keys(|enigo| {
        for key in keys {
            enigo
                .key(parse_key(key), Direction::Press)
                .map_err(|e| e.to_string())?;
        }

        for key in keys.iter().rev() {
            enigo
                .key(parse_key(key), Direction::Release)
                .map_err(|e| e.to_string())?;
        }

        Ok(())
    })
}

fn insert_text(text: &str) -> Result<String, String> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| format!("Enigo init fehlgeschlagen: {e}"))?;
    enigo.text(text).map_err(|e| e.to_string())?;
    Ok("OK".into())
}

fn parse_key(key: &str) -> Key {
    match key {
        "ctrl" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "enter" => Key::Return,
        "escape" => Key::Escape,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "j" => Key::Unicode('j'),
        "k" => Key::Unicode('k'),
        "l" => Key::Unicode('l'),
        "n" => Key::Unicode('n'),
        "o" => Key::Unicode('o'),
        "r" => Key::Unicode('r'),
        "s" => Key::Unicode('s'),
        "t" => Key::Unicode('t'),
        "w" => Key::Unicode('w'),
        "y" => Key::Unicode('y'),
        "z" => Key::Unicode('z'),
        _ => Key::Unicode(' '),
    }
}

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodingResponse {
    results: Option<Vec<OpenMeteoGeocodingResult>>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoGeocodingResult {
    name: String,
    country: Option<String>,
    admin1: Option<String>,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoForecastResponse {
    current: Option<OpenMeteoCurrent>,
    daily: Option<OpenMeteoDaily>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f32,
    apparent_temperature: Option<f32>,
    weather_code: Option<i32>,
    wind_speed_10m: Option<f32>,
    is_day: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoDaily {
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    precipitation_probability_max: Option<Vec<i32>>,
    weather_code: Option<Vec<i32>>,
}

fn weather_code_label(code: i32) -> &'static str {
    match code {
        0 => "klar",
        1 | 2 | 3 => "teilweise bewölkt",
        45 | 48 => "neblig",
        51 | 53 | 55 => "leichter Nieselregen",
        56 | 57 => "gefrierender Nieselregen",
        61 | 63 | 65 => "regnerisch",
        66 | 67 => "gefrierender Regen",
        71 | 73 | 75 | 77 => "schneit",
        80 | 81 | 82 => "Schauer",
        85 | 86 => "Schneeschauer",
        95 => "Gewitter",
        96 | 99 => "Gewitter mit Hagel",
        _ => "wechselhaft",
    }
}

fn build_clothing_advice(temp_now: f32, temp_max: f32, rain_prob: i32, wind_kmh: f32) -> String {
    let mut items: Vec<&str> = Vec::new();

    if temp_now <= 5.0 || temp_max <= 8.0 {
        items.push("eine warme Jacke");
    } else if temp_now <= 12.0 || temp_max <= 15.0 {
        items.push("eine leichte Jacke oder einen Hoodie");
    } else if temp_max >= 24.0 {
        items.push("leichte Kleidung");
    } else {
        items.push("normale Übergangskleidung");
    }

    if rain_prob >= 55 {
        items.push("einen Regenschirm");
    }

    if wind_kmh >= 30.0 {
        items.push("etwas Windfestes");
    }

    match items.len() {
        0 => "Kleidungsmäßig ist heute nichts Besonderes nötig.".to_string(),
        1 => format!("Ich würde dir heute {} empfehlen.", items[0]),
        _ => {
            let last = items.pop().unwrap_or("etwas Passendes");
            format!(
                "Ich würde dir heute {} und {} empfehlen.",
                items.join(", "),
                last
            )
        }
    }
}

fn german_weekday_name(weekday: chrono::Weekday) -> &'static str {
    match weekday {
        chrono::Weekday::Mon => "Montag",
        chrono::Weekday::Tue => "Dienstag",
        chrono::Weekday::Wed => "Mittwoch",
        chrono::Weekday::Thu => "Donnerstag",
        chrono::Weekday::Fri => "Freitag",
        chrono::Weekday::Sat => "Samstag",
        chrono::Weekday::Sun => "Sonntag",
    }
}

fn german_month_name(month: u32) -> &'static str {
    match month {
        1 => "Januar",
        2 => "Februar",
        3 => "März",
        4 => "April",
        5 => "Mai",
        6 => "Juni",
        7 => "Juli",
        8 => "August",
        9 => "September",
        10 => "Oktober",
        11 => "November",
        12 => "Dezember",
        _ => "Unbekannt",
    }
}

fn german_time_phrase(hour: u32, minute: u32) -> String {
    match minute {
        0 => format!("Es ist {} Uhr.", hour),
        15 => format!("Es ist Viertel nach {}.", hour),
        30 => format!("Es ist halb {}.", (hour + 1) % 24),
        45 => format!("Es ist Viertel vor {}.", (hour + 1) % 24),
        _ => format!("Es ist {:02}:{:02} Uhr.", hour, minute),
    }
}

async fn weather_reply(location: Option<String>) -> Result<String, String> {
    let place = location
        .unwrap_or_else(|| "Berlin".to_string())
        .trim()
        .to_string();

    if place.is_empty() {
        return Err("Kein Ort angegeben.".into());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Weather client konnte nicht erstellt werden: {e}"))?;

    let geo_url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=de&format=json",
        urlencoding::encode(&place)
    );

    let geo = client
        .get(&geo_url)
        .send()
        .await
        .map_err(|e| format!("Geocoding fehlgeschlagen: {e}"))?;

    if !geo.status().is_success() {
        let status = geo.status();
        let text = geo.text().await.unwrap_or_default();
        return Err(format!("Geocoding Fehler {}: {}", status, text));
    }

    let geo_data: OpenMeteoGeocodingResponse = geo
        .json()
        .await
        .map_err(|e| format!("Geocoding-Antwort konnte nicht gelesen werden: {e}"))?;

    let location_hit = geo_data
        .results
        .and_then(|mut results| results.drain(..).next())
        .ok_or_else(|| format!("Ich konnte keinen Ort für '{}' finden.", place))?;

    let forecast_url = format!(
        concat!(
            "https://api.open-meteo.com/v1/forecast",
            "?latitude={}",
            "&longitude={}",
            "&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,is_day",
            "&daily=temperature_2m_max,temperature_2m_min,precipitation_probability_max,weather_code",
            "&timezone=auto",
            "&forecast_days=1"
        ),
        location_hit.latitude, location_hit.longitude
    );

    let forecast = client
        .get(&forecast_url)
        .send()
        .await
        .map_err(|e| format!("Wetterabfrage fehlgeschlagen: {e}"))?;

    if !forecast.status().is_success() {
        let status = forecast.status();
        let text = forecast.text().await.unwrap_or_default();
        return Err(format!("Wetter API Fehler {}: {}", status, text));
    }

    let forecast_data: OpenMeteoForecastResponse = forecast
        .json()
        .await
        .map_err(|e| format!("Wetterdaten konnten nicht gelesen werden: {e}"))?;

    let current = forecast_data
        .current
        .ok_or_else(|| "Keine aktuellen Wetterdaten erhalten.".to_string())?;

    let daily = forecast_data
        .daily
        .ok_or_else(|| "Keine Tagesvorhersage erhalten.".to_string())?;

    let temp_now = current.temperature_2m;
    let feels_like = current.apparent_temperature.unwrap_or(temp_now);
    let wind = current.wind_speed_10m.unwrap_or(0.0);

    let temp_max = daily.temperature_2m_max.first().copied().unwrap_or(temp_now);
    let temp_min = daily.temperature_2m_min.first().copied().unwrap_or(temp_now);
    let rain_prob = daily
        .precipitation_probability_max
        .as_ref()
        .and_then(|v| v.first().copied())
        .unwrap_or(0);

    let weather_code = current
        .weather_code
        .or_else(|| daily.weather_code.as_ref().and_then(|v| v.first().copied()))
        .unwrap_or(-1);

    let summary = weather_code_label(weather_code);
    let advice = build_clothing_advice(temp_now, temp_max, rain_prob, wind);

    let pretty_place = match (&location_hit.admin1, &location_hit.country) {
        (Some(admin1), Some(country)) if !admin1.is_empty() => {
            format!("{}, {}, {}", location_hit.name, admin1, country)
        }
        (_, Some(country)) => format!("{}, {}", location_hit.name, country),
        _ => location_hit.name.clone(),
    };

    Ok(format!(
        "{}: Gerade sind es {:.0}°C, gefühlt {:.0}°C und es ist {}. Heute liegt die Temperatur ungefähr zwischen {:.0}°C und {:.0}°C, Regenwahrscheinlichkeit bis {}%, Wind etwa {:.0} km/h. {}",
        pretty_place,
        temp_now,
        feels_like,
        summary,
        temp_min,
        temp_max,
        rain_prob,
        wind,
        advice
    ))
}

#[tauri::command]
async fn handle_voice_command(app: tauri::AppHandle, input: String) -> Result<String, String> {
    modules::session_memory::set_last_command(&input);

    let personality = load_personality_state().unwrap_or_default();
    let _personality_mood_hint = personality.mood_hint();

    let mut ctx = resolve_active_context();
    let active_app = get_active_app();

    async fn maybe_speak_reply(reply: &str) {
        let lang = "en";

        if let Err(err) = modules::tts::manager::speak(reply, Some(lang)).await {
            eprintln!("TTS error: {err}");
        }
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

    let parsed_action =
        parse_voice_command_with_context(&input, &ctx.app_name, &ctx.window_title, &ctx.domain);

    let action = if let Some(mapped) = resolve_app_action(parsed_action.clone(), &active_app) {
        mapped
    } else {
        parsed_action
    };

    match action {
        CompanionAction::InsertSnippet { key } => {
            if let Some(value) = modules::snippets::get_snippet(&key) {
                let target = ensure_external_focus(&active_app)?;
                insert_text(&value)?;
                ok_and_remember!(
                    &input,
                    ctx,
                    format!("Snippet '{}' in {} eingefügt.", key, target)
                )
            } else {
                Ok(format!("Kein Snippet für '{}' gefunden.", key))
            }
        },

        CompanionAction::CoinFlip => {
            let result = if rand::thread_rng().gen_bool(0.5) {
                "Kopf"
            } else {
                "Zahl"
            };

            let reply = format!("Ich werfe eine Münze ... {}!", result);
            maybe_speak_reply(&reply).await;
            ok_and_remember!(&input, ctx, reply)
        },

        CompanionAction::RollDice => {
            let value = rand::thread_rng().gen_range(1..=6);

            ok_and_remember!(
                &input,
                ctx,
                format!("Du hast eine {} gewürfelt.", value)
            )
        },

        CompanionAction::CancelTimer => {
            cancel_active_timer();

            let _ = app.emit(
                "companion-timer-finished",
                serde_json::json!({
                    "seconds": 0,
                    "text": "Timer gestoppt.",
                }),
            );

            ok_and_remember!(&input, ctx, "Timer gestoppt.".into())
        },

        CompanionAction::SetTimer { seconds } => {
            let seconds = seconds.max(1);
            let app_handle = app.clone();
            let timer_id = next_timer_id();

            let _ = app.emit(
                "companion-timer-started",
                serde_json::json!({
                    "seconds": seconds,
                    "label": format!("{}:{:02} timer", seconds / 60, seconds % 60),
                    "startedAt": chrono::Utc::now().timestamp(),
                }),
            );

            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(seconds)).await;

                if current_timer_id() != timer_id {
                    return;
                }

                let text = format!(
                    "Dein Timer über {} Minute(n) und {} Sekunde(n) ist fertig.",
                    seconds / 60,
                    seconds % 60
                );

                let _ = app_handle.emit(
                    "companion-timer-finished",
                    serde_json::json!({
                        "seconds": seconds,
                        "text": text,
                    }),
                );

                let _ = modules::tts::manager::speak(&text, Some("de")).await;
            });

            ok_and_remember!(
                &input,
                ctx,
                format!("Timer für {}:{:02} gestartet.", seconds / 60, seconds % 60)
            )
        },

        CompanionAction::YouTubePlay => {
            let target = ensure_external_focus(&active_app)?;
            press_key("k")?;
            ok_and_remember!(&input, ctx, format!("YouTube-Play/Pause in {} ausgelöst.", target))
        }

        CompanionAction::YouTubePause => {
            let target = ensure_external_focus(&active_app)?;
            press_key("k")?;
            ok_and_remember!(&input, ctx, format!("YouTube-Play/Pause in {} ausgelöst.", target))
        }

        CompanionAction::YouTubeSkipAd => {
            ensure_debug_browser().await?;

            if browser_click_best_match("skip ads".to_string()).await.is_ok() {
                return ok_and_remember!(&input, ctx, String::from("Werbung übersprungen."));
            }

            if browser_click_best_match("skip ad".to_string()).await.is_ok() {
                return ok_and_remember!(&input, ctx, String::from("Werbung übersprungen."));
            }

            if browser_click_best_match("überspringen".to_string()).await.is_ok() {
                return ok_and_remember!(&input, ctx, String::from("Werbung übersprungen."));
            }

            if browser_click_best_match("ueberspringen".to_string()).await.is_ok() {
                return ok_and_remember!(&input, ctx, String::from("Werbung übersprungen."));
            }

            Ok("Kein Skip-Button gefunden.".into())
        }

        CompanionAction::StreamOpenTitle {
            service,
            title,
            autoplay: _,
        } => {
            println!("[stream-open] service='{}' title='{}'", service, title);

            if let Some(item) = modules::streaming::find_title(&service, &title) {
                println!("[stream-open] matched '{}' -> {}", item.title, item.url);

                open_url_prefer_browser(&item.url, false, false)?;
                modules::session_memory::set_last_suggestion(
                    &item.title,
                    &service,
                    &item.url,
                    &title,
                );

                ok_and_remember!(&input, ctx, format!("Opening {} on {}.", item.title, service))
            } else {
                Ok(format!("I couldn't find '{}' on {}.", title, service))
            }
        }

        CompanionAction::StreamRecommend {
            service,
            mood,
            genre,
            kind,
            trending,
        } => {
            let query_text = format!(
                "{} {} {} {} {}",
                service.clone().unwrap_or_else(|| "netflix".into()),
                mood.clone().unwrap_or_default(),
                genre.clone().unwrap_or_default(),
                kind.clone().unwrap_or_default(),
                if trending { "trending" } else { "" }
            );

            let rec = modules::streaming::recommend_title_with_reason(
                modules::streaming::RecommendationQuery {
                    service: service.clone(),
                    mood: mood.clone(),
                    genre: genre.clone(),
                    kind: kind.clone(),
                    trending,
                    exclude_titles: Vec::new(),
                },
            );

            if let Some(rec) = rec {
                modules::session_memory::set_last_suggestion(
                    &rec.title.title,
                    &rec.title.service,
                    &rec.title.url,
                    &query_text,
                );

                ok_and_remember!(
                    &input,
                    ctx,
                    modules::streaming::build_recommendation_reply(&rec)
                )
            } else {
                Ok("I couldn't find a strong recommendation yet.".into())
            }
        }

        CompanionAction::StreamCapability { service } => {
            let svc = service.unwrap_or_else(|| "netflix".into());

            ok_and_remember!(
                &input,
                ctx,
                format!(
                    "I can open specific titles on {}, show trending picks, and recommend movies or series by mood, genre, or type. For example: 'play Black Mirror on {}', 'what's trending on {}', or 'recommend a funny movie on {}'.",
                    svc, svc, svc, svc
                )
            )
        }

        CompanionAction::StreamOpenLastSuggestion => {
            let state = modules::session_memory::get_state();

            if !state.last_suggested_url.is_empty() {
                open_url_prefer_browser(&state.last_suggested_url, false, false)?;
                return ok_and_remember!(
                    &input,
                    ctx,
                    format!("Opening {}.", state.last_suggested_title)
                );
            }

            Ok("There is no recent streaming suggestion to open.".into())
        }

        CompanionAction::StreamMoreLikeLast => {
            let state = modules::session_memory::get_state();

            let rec = modules::streaming::best_followup_alternative(
                if state.last_suggested_service.is_empty() {
                    "netflix"
                } else {
                    &state.last_suggested_service
                },
                &[state.last_suggested_title.clone()],
                if state.last_recommendation_query.is_empty() {
                    None
                } else {
                    Some(state.last_recommendation_query.as_str())
                },
            );

            if let Some(item) = rec {
                modules::session_memory::set_last_suggestion(
                    &item.title.title,
                    &item.title.service,
                    &item.title.url,
                    &state.last_recommendation_query,
                );

                ok_and_remember!(
                    &input,
                    ctx,
                    format!(
                        "Then {} could be another good choice. {}. Want me to open it?",
                        item.title.title, item.reason
                    )
                )
            } else {
                Ok("I couldn't find another good option yet.".into())
            }
        }

        CompanionAction::VolumeUp => {
            modules::system::change_system_volume(0.08)?;
            ok_and_remember!(&input, ctx, String::from("Okay, lauter."))
        }

        CompanionAction::VolumeDown => {
            modules::system::change_system_volume(-0.08)?;
            ok_and_remember!(&input, ctx, String::from("Okay, leiser."))
        }

        CompanionAction::SetVolume { percent } => Err(format!(
            "Exakte Prozent-Lautstärke auf {}% ist noch nicht implementiert.",
            percent
        )),

        CompanionAction::Mute => {
            modules::system::set_system_mute(true)?;
            ok_and_remember!(&input, ctx, String::from("Okay, Ton aus."))
        }

        CompanionAction::Unmute => {
            modules::system::set_system_mute(false)?;
            ok_and_remember!(&input, ctx, String::from("Okay, Ton an."))
        }

        CompanionAction::ToggleMute => {
            modules::system::toggle_system_mute()?;
            ok_and_remember!(&input, ctx, String::from("Mute umgeschaltet."))
        }

        CompanionAction::MediaPlayPause => {
            let target = ensure_external_focus(&active_app)?;
            press_key("k")?;
            ok_and_remember!(&input, ctx, format!("Play oder Pause in {}.", target))
        }

        CompanionAction::MediaNext => {
            modules::system::media_next_track()?;
            ok_and_remember!(&input, ctx, String::from("Okay, nächster Titel."))
        }

        CompanionAction::MediaPrev => {
            modules::system::media_prev_track()?;
            ok_and_remember!(&input, ctx, String::from("Okay, vorheriger Titel."))
        }

        CompanionAction::Save => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('s')?;
            ok_and_remember!(&input, ctx, format!("Speichern in {} ausgelöst.", target))
        }

        CompanionAction::SaveAs => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl_shift('s')?;
            ok_and_remember!(&input, ctx, format!("Speichern unter in {} ausgelöst.", target))
        }

        CompanionAction::OpenFile => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('o')?;
            ok_and_remember!(&input, ctx, format!("Öffnen in {} ausgelöst.", target))
        }

        CompanionAction::NewFile => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('n')?;
            ok_and_remember!(&input, ctx, format!("Neu in {} ausgelöst.", target))
        }

        CompanionAction::Close | CompanionAction::CloseApp => {
            ensure_debug_browser().await?;
            let tab = modules::browser_automations::get_active_tab().await?;
            modules::browser_automations::close_tab(&tab.id).await?;
            ok_and_remember!(&input, ctx, "Aktiven Browser-Tab geschlossen.".into())
        }

        CompanionAction::NewTab => {
            ensure_debug_browser().await?;
            modules::browser_automations::new_tab("https://www.google.com").await?;
            ok_and_remember!(&input, ctx, "Neuer Browser-Tab geöffnet.".into())
        }

        CompanionAction::CloseTab => {
            ensure_debug_browser().await?;
            let tab = modules::browser_automations::get_active_tab().await?;
            modules::browser_automations::close_tab(&tab.id).await?;
            ok_and_remember!(&input, ctx, "Aktiven Browser-Tab geschlossen.".into())
        }

        CompanionAction::CloseTabByIndex { index } => {
            let reply = browser_close_tab_by_index(index).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::NewWindow => {
            ensure_debug_browser().await?;
            modules::browser_automations::new_tab("https://www.google.com").await?;
            ok_and_remember!(&input, ctx, "Neuer Browser-Tab geöffnet.".into())
        }

        CompanionAction::Incognito => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl_shift('n')?;
            ok_and_remember!(
                &input,
                ctx,
                format!("Inkognito-Fenster in {} geöffnet.", target)
            )
        }

        CompanionAction::Reload => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('r')?;
            ok_and_remember!(&input, ctx, format!("Seite in {} neu geladen.", target))
        }

        CompanionAction::Undo => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('z')?;
            ok_and_remember!(&input, ctx, format!("Rückgängig in {}.", target))
        }

        CompanionAction::Redo => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('y')?;
            ok_and_remember!(&input, ctx, format!("Wiederholen in {}.", target))
        }

        CompanionAction::BrowserBack => {
            let reply = browser_back().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserForward => {
            let reply = browser_forward().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserScrollDown => {
            let reply = browser_scroll_down().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserScrollUp => {
            let reply = browser_scroll_up().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserTypeText { text } => {
            let reply = browser_type_text(text).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserSubmit => {
            let reply = browser_submit().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserClickBestMatch { text } => {
            let reply = browser_click_best_match(text).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserClickButtonByText { text } => {
            let reply = browser_click_best_match(text).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserContext => {
            let browser_ctx = browser_get_context().await?;
            ok_and_remember!(
                &input,
                ctx,
                format!(
                    "Seite: {} | URL: {} | Typ: {} | Links: {} | Buttons: {} | Inputs: {}",
                    browser_ctx.title,
                    browser_ctx.url,
                    browser_ctx.page_kind,
                    browser_ctx.visible_links.len(),
                    browser_ctx.visible_buttons.len(),
                    browser_ctx.visible_inputs.len()
                )
            )
        }

        CompanionAction::GoogleSearch { query } => {
            ensure_debug_browser().await?;
            let url = format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(&query)
            );
            modules::browser_automations::navigate_best_tab(&url).await?;
            ok_and_remember!(&input, ctx, format!("Suche auf Google nach {}.", query))
        }

        CompanionAction::YouTubeSearch { query } => {
            ensure_debug_browser().await?;
            modules::browser_automations::youtube_search(&query, false).await?;
            ok_and_remember!(&input, ctx, format!("Suche auf YouTube nach {}.", query))
        }

        CompanionAction::YouTubePlayTitle { title } => {
            ensure_debug_browser().await?;
            let url = format!(
                "https://www.youtube.com/results?search_query={}",
                urlencoding::encode(&title)
            );
            modules::browser_automations::navigate_best_tab(&url).await?;
            ok_and_remember!(&input, ctx, format!("Suche auf YouTube nach {}.", title))
        }

        CompanionAction::BrowserOpenUrl {
            url,
            new_tab,
            new_window,
            incognito,
        } => {
            let reply = browser_open_url(url, new_tab, new_window, incognito).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserClickLinkByText { text, new_tab } => {
            let reply = browser_click_link_by_text(text, new_tab).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserClickFirstResult => {
            let reply = browser_click_first_result().await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::BrowserClickNthResult { index } => {
            let reply = browser_click_nth_result(index).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::OpenApp {
            target,
            prefer_browser,
        } => {
            let reply = open_app_target(&target, prefer_browser)?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::InsertText(text) => {
            let target = ensure_external_focus(&active_app)?;
            insert_text(&text)?;
            ok_and_remember!(&input, ctx, format!("Text in {} eingegeben.", target))
        }

        CompanionAction::KeyPress(key) => {
            let target = ensure_external_focus(&active_app)?;
            press_key(key)?;
            ok_and_remember!(&input, ctx, format!("Taste {} in {} gesendet.", key, target))
        }

        CompanionAction::KeyCombo(keys) => {
            let target = ensure_external_focus(&active_app)?;
            press_key_combo(&keys)?;
            ok_and_remember!(
                &input,
                ctx,
                format!("Tastenkombination in {} gesendet.", target)
            )
        }

        CompanionAction::Confirm => {
            let state = modules::session_memory::get_state();

            if !state.last_suggested_url.is_empty() {
                open_url_prefer_browser(&state.last_suggested_url, false, false)?;
                return ok_and_remember!(
                    &input,
                    ctx,
                    format!("Opening {}.", state.last_suggested_title)
                );
            }

            let target = ensure_external_focus(&active_app)?;
            press_key("enter")?;
            ok_and_remember!(&input, ctx, format!("Confirmed in {}.", target))
        }

        CompanionAction::Clear => {
            let target = ensure_external_focus(&active_app)?;
            press_key("escape")?;
            ok_and_remember!(&input, ctx, format!("Zurückgesetzt in {}.", target))
        }

        CompanionAction::YouTubeNextVideo => {
            let target = ensure_external_focus(&active_app)?;
            press_key_combo(&["shift", "n"])?;
            ok_and_remember!(
                &input,
                ctx,
                format!("Nächstes YouTube-Video in {}.", target)
            )
        }

        CompanionAction::YouTubeSeekForward => {
            let target = ensure_external_focus(&active_app)?;
            press_key("l")?;
            ok_and_remember!(&input, ctx, format!("YouTube in {} vorgespult.", target))
        }

        CompanionAction::YouTubeSeekBackward => {
            let target = ensure_external_focus(&active_app)?;
            press_key("j")?;
            ok_and_remember!(&input, ctx, format!("YouTube in {} zurückgespult.", target))
        }

        CompanionAction::CurrentTime => {
            let now = Local::now();
            ok_and_remember!(&input, ctx, german_time_phrase(now.hour(), now.minute()))
        }

        CompanionAction::CurrentDate => {
            let now = Local::now();
            ok_and_remember!(
                &input,
                ctx,
                format!(
                    "Heute ist {}, der {}. {} {}.",
                    german_weekday_name(now.weekday()),
                    now.day(),
                    german_month_name(now.month()),
                    now.year()
                )
            )
        }

        CompanionAction::WeatherToday { location } => {
            let reply = weather_reply(location).await?;
            ok_and_remember!(&input, ctx, reply)
        }

        CompanionAction::ExplainSelection => Ok("NO_ACTION".into()),

        CompanionAction::TakeScreenshot => {
            let _ = app.emit("companion-snip-hotkey", ());
            ok_and_remember!(&input, ctx, String::from("Snip-Modus geöffnet."))
        }

        CompanionAction::None => Ok("NO_ACTION".into()),
    }
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