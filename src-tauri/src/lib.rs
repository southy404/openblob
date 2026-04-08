use device_query::{DeviceQuery, DeviceState};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::Emitter;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::ProcessStatus::K32GetModuleFileNameExW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

mod modules {
    pub mod app_profiles;
    pub mod browser_automations;
    pub mod command_router;
    pub mod session_memory;
    pub mod steam_games;
    pub mod system;
    pub mod voice;
    pub mod windows_discovery;
}

use modules::app_profiles::resolve_app_action;
use modules::command_router::{parse_voice_command, CompanionAction};

static LAST_EXTERNAL_APP: OnceLock<Mutex<String>> = OnceLock::new();

fn last_external_app_store() -> &'static Mutex<String> {
    LAST_EXTERNAL_APP.get_or_init(|| Mutex::new(String::from("unknown")))
}

#[derive(Debug, Serialize)]
struct OllamaResult {
    content: String,
    model: String,
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

fn system_prompt() -> &'static str {
    "You are a smart desktop companion living on the user's computer.
Be concise, helpful, and context-aware.
When asked to explain text, explain clearly and practically.
When asked to translate, preserve tone and meaning.
If context is incomplete, say so briefly and still do your best."
}

fn build_user_prompt(mode: &str, text: &str, question: Option<&str>) -> String {
    match mode {
        "translate_de" => format!(
            "Translate the following text into German. Preserve meaning and tone.\n\nTEXT:\n{}",
            text
        ),
        "translate_en" => format!(
            "Translate the following text into English. Preserve meaning and tone.\n\nTEXT:\n{}",
            text
        ),
        "explain" => format!(
            "Explain the following text clearly and simply. If it contains technical terms, explain them too.\n\nTEXT:\n{}",
            text
        ),
        "ask" => format!(
            "Answer the user's question using the context below when relevant.\n\nCONTEXT:\n{}\n\nQUESTION:\n{}",
            text,
            question.unwrap_or("What does this mean?")
        ),
        _ => format!("Help the user with the following text.\n\nTEXT:\n{}", text),
    }
}

fn is_internal_companion_app(app: &str) -> bool {
    let lower = app.to_lowercase();
    lower.contains("companion-v1")
        || lower.contains("webview")
        || lower.contains("msedgewebview2")
        || lower.contains("bubble")
        || lower.contains("speech")
        || lower == "unknown"
}

fn remember_external_app(app: &str) {
    if is_internal_companion_app(app) {
        return;
    }

    if let Ok(mut guard) = last_external_app_store().lock() {
        *guard = app.to_string();
        modules::session_memory::set_last_external_app(app);
    }
}

fn get_last_external_app() -> String {
    if let Ok(guard) = last_external_app_store().lock() {
        return guard.clone();
    }
    "unknown".into()
}

fn focus_hint_for_app(app: &str) -> Option<&'static str> {
    let lower = app.to_lowercase();

    if lower.contains("chrome") {
        return Some("Chrome");
    }
    if lower.contains("msedge") || lower.contains("edge") {
        return Some("Edge");
    }
    if lower.contains("firefox") {
        return Some("Firefox");
    }
    if lower.contains("mspaint") || lower.contains("paint") {
        return Some("Paint");
    }
    if lower.contains("notepad") {
        return Some("Notepad");
    }
    if lower.contains("calc") {
        return Some("Calculator");
    }
    if lower.contains("explorer") {
        return Some("File Explorer");
    }
    None
}

fn focus_app_window(app: &str) -> Result<(), String> {
    if let Some(hint) = focus_hint_for_app(app) {
        let script = format!(
            "$ws = New-Object -ComObject WScript.Shell; $null = $ws.AppActivate('{}')",
            hint.replace('\'', "''")
        );

        Command::new("powershell")
            .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Fokus konnte nicht gesetzt werden: {e}"))?;

        std::thread::sleep(Duration::from_millis(120));
    }

    Ok(())
}

fn focus_last_external_app() -> Result<String, String> {
    let app = get_last_external_app();
    if app != "unknown" {
        focus_app_window(&app)?;
    }
    Ok(app)
}

fn ensure_external_focus(preferred_app: &str) -> Result<String, String> {
    if !is_internal_companion_app(preferred_app) {
        remember_external_app(preferred_app);
        focus_app_window(preferred_app)?;
        return Ok(preferred_app.to_string());
    }

    focus_last_external_app()
}

async fn is_debug_browser_running() -> bool {
    let client = reqwest::Client::new();

    match client.get("http://127.0.0.1:9222/json").send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    }
}

fn spawn_debug_browser() -> Result<(), String> {
    let chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
    let edge_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
    let user_data = r"D:\companion-browser";

    if std::path::Path::new(chrome_path).exists() {
        Command::new(chrome_path)
            .args([
                "--remote-debugging-port=9222",
                &format!("--user-data-dir={}", user_data),
                "--new-window",
            ])
            .spawn()
            .map_err(|e| format!("Chrome konnte nicht gestartet werden: {e}"))?;
        return Ok(());
    }

    if std::path::Path::new(edge_path).exists() {
        Command::new(edge_path)
            .args([
                "--remote-debugging-port=9222",
                &format!("--user-data-dir={}", user_data),
                "--new-window",
            ])
            .spawn()
            .map_err(|e| format!("Edge konnte nicht gestartet werden: {e}"))?;
        return Ok(());
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

    let chosen_model = model.unwrap_or_else(|| "llama3.1:8b".to_string());

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

    if new_tab {
        modules::browser_automations::new_tab(&url).await?;
        return Ok("URL in neuem Tab geöffnet.".into());
    }

    if new_window || incognito {
        open_url_prefer_browser(&url, new_window, incognito)?;
        return Ok("URL im Browser geöffnet.".into());
    }

    modules::browser_automations::navigate_active_tab(&url).await?;
    Ok("URL im aktiven Tab geöffnet.".into())
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
    unsafe {
        let hwnd: HWND = GetForegroundWindow();

        if hwnd.0 == 0 {
            return "unknown".into();
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        if pid == 0 {
            return "unknown".into();
        }

        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);

        if process.is_err() {
            return "unknown".into();
        }

        let process = process.unwrap();

        let mut buffer = [0u16; 260];
        let len = K32GetModuleFileNameExW(process, None, &mut buffer);

        if len == 0 {
            return "unknown".into();
        }

        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        let app = path.split('\\').last().unwrap_or("unknown").to_string();

        remember_external_app(&app);
        app
    }
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
    if command_exists_windows("chrome") {
        let mut args = vec!["/C", "start", "", "chrome"];
        if incognito {
            args.push("--incognito");
        }
        if new_window {
            args.push("--new-window");
        }
        args.push(url);
        return spawn_hidden_cmd(&args);
    }

    if command_exists_windows("msedge") {
        let mut args = vec!["/C", "start", "", "msedge"];
        if incognito {
            args.push("-inprivate");
        }
        if new_window {
            args.push("--new-window");
        }
        args.push(url);
        return spawn_hidden_cmd(&args);
    }

    spawn_hidden_cmd(&["/C", "start", "", url])
}

fn known_web_fallback(target: &str) -> Option<&'static str> {
    match target {
        "discord" => Some("https://discord.com/app"),
        "spotify" => Some("https://open.spotify.com"),
        "youtube" => Some("https://www.youtube.com"),
        "google" => Some("https://www.google.com"),
        "gmail" => Some("https://mail.google.com"),
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

    if open_known_local_target(&normalized)? {
        return Ok(format!("Öffne {}.", target));
    }

    if let Some(game) = modules::steam_games::find_steam_game(&normalized) {
        let uri = modules::steam_games::steam_launch_uri(&game.appid);
        spawn_hidden_cmd(&["/C", "start", "", &uri])?;
        return Ok(format!("Starte {} über Steam.", game.name));
    }

    if let Some(app) = modules::windows_discovery::find_app_launch_target(&normalized) {
        spawn_hidden_cmd(&["/C", "start", "", &app.launch_target])?;
        return Ok(format!("Öffne {}.", app.canonical_name));
    }

    if command_exists_windows(&normalized) {
        spawn_hidden_cmd(&["/C", "start", "", &normalized])?;
        return Ok(format!("Öffne {}.", target));
    }

    if prefer_browser {
        if let Some(url) = known_web_fallback(&normalized) {
            open_url_prefer_browser(url, false, false)?;
            return Ok(format!("Öffne {} im Browser.", target));
        }
    }

    if let Some(url) = known_web_fallback(&normalized) {
        open_url_prefer_browser(url, false, false)?;
        return Ok(format!("{} lokal nicht gefunden. Öffne Web-Version.", target));
    }

    Err(format!("Ich konnte '{}' nicht öffnen.", target))
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

async fn weather_reply(location: Option<String>) -> Result<String, String> {
    let place = location.unwrap_or_else(|| "Hamburg".to_string());
    Ok(format!(
        "Live-Wetter ist in V5.5 noch nicht angebunden. Für '{}' kann ich als Nächstes echte Wetterdaten einbauen.",
        place
    ))
}

#[tauri::command]
async fn handle_voice_command(input: String) -> Result<String, String> {
    modules::session_memory::set_last_command(&input);

    let parsed_action = parse_voice_command(&input);
    let active_app = get_active_app();

    let action = if let Some(mapped) = resolve_app_action(parsed_action.clone(), &active_app) {
        mapped
    } else {
        parsed_action
    };

    match action {
        CompanionAction::VolumeUp => {
            modules::system::change_system_volume(0.08)?;
            Ok("Okay, lauter.".into())
        }
        CompanionAction::VolumeDown => {
            modules::system::change_system_volume(-0.08)?;
            Ok("Okay, leiser.".into())
        }
        CompanionAction::SetVolume { percent } => Err(format!(
            "Exakte Prozent-Lautstärke auf {}% ist noch nicht implementiert.",
            percent
        )),
        CompanionAction::Mute => {
            modules::system::set_system_mute(true)?;
            Ok("Okay, Ton aus.".into())
        }
        CompanionAction::Unmute => {
            modules::system::set_system_mute(false)?;
            Ok("Okay, Ton an.".into())
        }
        CompanionAction::ToggleMute => {
            modules::system::toggle_system_mute()?;
            Ok("Mute umgeschaltet.".into())
        }

        CompanionAction::MediaPlayPause => {
            let target = ensure_external_focus(&active_app)?;
            press_key("k")?;
            Ok(format!("Play oder Pause in {}.", target))
        }
        CompanionAction::MediaNext => {
            modules::system::media_next_track()?;
            Ok("Okay, nächster Titel.".into())
        }
        CompanionAction::MediaPrev => {
            modules::system::media_prev_track()?;
            Ok("Okay, vorheriger Titel.".into())
        }

        CompanionAction::Save => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('s')?;
            Ok(format!("Speichern in {} ausgelöst.", target))
        }
        CompanionAction::SaveAs => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl_shift('s')?;
            Ok(format!("Speichern unter in {} ausgelöst.", target))
        }
        CompanionAction::OpenFile => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('o')?;
            Ok(format!("Öffnen in {} ausgelöst.", target))
        }
        CompanionAction::NewFile => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('n')?;
            Ok(format!("Neu in {} ausgelöst.", target))
        }
        CompanionAction::Close | CompanionAction::CloseApp => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_alt_f4()?;
            Ok(format!("{} geschlossen.", target))
        }

        CompanionAction::NewTab => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('t')?;
            Ok(format!("Neuer Tab in {}.", target))
        }
        CompanionAction::CloseTab => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('w')?;
            Ok(format!("Tab in {} geschlossen.", target))
        }
        CompanionAction::CloseTabByIndex { index } => browser_close_tab_by_index(index).await,
        CompanionAction::NewWindow => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('n')?;
            Ok(format!("Neues Fenster in {}.", target))
        }
        CompanionAction::Incognito => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl_shift('n')?;
            Ok(format!("Inkognito-Fenster in {} geöffnet.", target))
        }
        CompanionAction::Reload => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('r')?;
            Ok(format!("Seite in {} neu geladen.", target))
        }
        CompanionAction::Undo => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('z')?;
            Ok(format!("Rückgängig in {}.", target))
        }
        CompanionAction::Redo => {
            let target = ensure_external_focus(&active_app)?;
            shortcut_ctrl('y')?;
            Ok(format!("Wiederholen in {}.", target))
        }

        CompanionAction::BrowserBack => browser_back().await,
        CompanionAction::BrowserForward => browser_forward().await,
        CompanionAction::BrowserScrollDown => browser_scroll_down().await,
        CompanionAction::BrowserScrollUp => browser_scroll_up().await,
        CompanionAction::BrowserTypeText { text } => browser_type_text(text).await,
        CompanionAction::BrowserSubmit => browser_submit().await,
        CompanionAction::BrowserClickBestMatch { text } => browser_click_best_match(text).await,

        CompanionAction::BrowserContext => {
            let ctx = browser_get_context().await?;
            Ok(format!(
                "Seite: {} | URL: {} | Typ: {} | Links: {} | Buttons: {} | Inputs: {}",
                ctx.title,
                ctx.url,
                ctx.page_kind,
                ctx.visible_links.len(),
                ctx.visible_buttons.len(),
                ctx.visible_inputs.len()
            ))
        }

        CompanionAction::GoogleSearch { query } => {
            let url = format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(&query)
            );
            open_url_prefer_browser(&url, false, false)?;
            Ok(format!("Suche auf Google nach {}.", query))
        }

        CompanionAction::YouTubeSearch { query } => match youtube_search_and_play(query.clone()).await {
            Ok(msg) => Ok(msg),
            Err(_) => {
                let url = format!(
                    "https://www.youtube.com/results?search_query={}",
                    urlencoding::encode(&query)
                );
                open_url_prefer_browser(&url, false, false)?;
                Ok(format!("Suche auf YouTube nach {}.", query))
            }
        },

        CompanionAction::YouTubePlayTitle { title } => match youtube_play_title(title.clone()).await {
            Ok(msg) => Ok(msg),
            Err(_) => {
                let url = format!(
                    "https://www.youtube.com/results?search_query={}",
                    urlencoding::encode(&title)
                );
                open_url_prefer_browser(&url, false, false)?;
                Ok(format!("Suche auf YouTube nach {}.", title))
            }
        },

        CompanionAction::BrowserOpenUrl {
            url,
            new_tab,
            new_window,
            incognito,
        } => browser_open_url(url, new_tab, new_window, incognito).await,

        CompanionAction::BrowserClickLinkByText { text, new_tab } => {
            browser_click_link_by_text(text, new_tab).await
        }
        CompanionAction::BrowserClickFirstResult => browser_click_first_result().await,
        CompanionAction::BrowserClickNthResult { index } => browser_click_nth_result(index).await,

        CompanionAction::OpenApp {
            target,
            prefer_browser,
        } => open_app_target(&target, prefer_browser),

        CompanionAction::InsertText(text) => {
            let target = ensure_external_focus(&active_app)?;
            insert_text(&text)?;
            Ok(format!("Text in {} eingegeben.", target))
        }
        CompanionAction::KeyPress(key) => {
            let target = ensure_external_focus(&active_app)?;
            press_key(key)?;
            Ok(format!("Taste {} in {} gesendet.", key, target))
        }
        CompanionAction::KeyCombo(keys) => {
            let target = ensure_external_focus(&active_app)?;
            press_key_combo(&keys)?;
            Ok(format!("Tastenkombination in {} gesendet.", target))
        }
        CompanionAction::Confirm => {
            let target = ensure_external_focus(&active_app)?;
            press_key("enter")?;
            Ok(format!("Bestätigt in {}.", target))
        }
        CompanionAction::Clear => {
            let target = ensure_external_focus(&active_app)?;
            press_key("escape")?;
            Ok(format!("Zurückgesetzt in {}.", target))
        }

        CompanionAction::YouTubeNextVideo => {
            let target = ensure_external_focus(&active_app)?;
            press_key_combo(&["shift", "n"])?;
            Ok(format!("Nächstes YouTube-Video in {}.", target))
        }
        CompanionAction::YouTubeSeekForward => {
            let target = ensure_external_focus(&active_app)?;
            press_key("l")?;
            Ok(format!("YouTube in {} vorgespult.", target))
        }
        CompanionAction::YouTubeSeekBackward => {
            let target = ensure_external_focus(&active_app)?;
            press_key("j")?;
            Ok(format!("YouTube in {} zurückgespult.", target))
        }

        CompanionAction::WeatherToday { location } => weather_reply(location).await,
        CompanionAction::ExplainSelection => Ok("NO_ACTION".into()),
        CompanionAction::None => Ok("NO_ACTION".into()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = app.emit("companion-hotkey", "selection");
                    }
                })
                .build(),
        )
        .setup(|app| {
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().register("Ctrl+Alt+Q")?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping_ollama,
            ask_ollama,
            trigger_copy_shortcut,
            get_cursor_position,
            get_active_app,
            handle_voice_command,
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