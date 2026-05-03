use std::process::{Command, Stdio};

use crate::modules::context::is_internal_companion_app;
use crate::modules::i18n::replies::reply_with;

#[cfg(windows)]
fn command_exists_windows(command: &str) -> bool {
    Command::new("where")
        .arg(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn spawn_hidden_cmd(args: &[&str]) -> Result<(), String> {
    Command::new("cmd")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Konnte nicht öffnen: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn spawn_open(args: &[&str]) -> Result<(), String> {
    Command::new("open")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not open: {e}"))?;
    Ok(())
}

pub fn open_url_prefer_browser(url: &str, new_window: bool, incognito: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = (new_window, incognito);
        spawn_open(&[url])?;
        return Ok(());
    }

    #[cfg(windows)]
    {
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
    #[cfg(target_os = "macos")]
    {
        let opened = match target {
            "finder" | "file explorer" | "explorer" => {
                spawn_open(&["-a", "Finder"])?;
                true
            }
            "settings" | "system settings" => {
                // Works across macOS versions (Monterey+: System Settings).
                // Falls back to the bundle id if the name is different.
                if spawn_open(&["-a", "System Settings"]).is_ok() {
                    true
                } else if spawn_open(&["-b", "com.apple.systempreferences"]).is_ok() {
                    true
                } else {
                    false
                }
            }
            "safari" => {
                spawn_open(&["-a", "Safari"])?;
                true
            }
            "chrome" | "google chrome" => {
                spawn_open(&["-a", "Google Chrome"])?;
                true
            }
            "terminal" => {
                spawn_open(&["-a", "Terminal"])?;
                true
            }
            "calculator" | "calc" => {
                spawn_open(&["-a", "Calculator"])?;
                true
            }
            "notes" => {
                spawn_open(&["-a", "Notes"])?;
                true
            }
            _ => false,
        };

        return Ok(opened);
    }

    #[cfg(windows)]
    {
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
}

pub fn open_app_target(
    target: &str,
    prefer_browser: bool,
) -> Result<String, String> {
    let normalized = target.trim().to_lowercase();

    if prefer_browser {
        if let Some(url) = known_web_fallback(&normalized) {
            open_url_prefer_browser(url, false, false)?;
            return Ok(reply_with(
                "open_app_opening_browser",
                &[("target", target.to_string())],
            ));
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
        return Ok(reply_with(
            "open_app_opening",
            &[("target", target.to_string())],
        ));
    }

    #[cfg(windows)]
    {
        if let Some(game) = crate::modules::steam_games::find_steam_game(&normalized) {
            let uri = crate::modules::steam_games::steam_launch_uri(&game.appid);
            spawn_hidden_cmd(&["/C", "start", "", &uri])?;
            return Ok(reply_with(
                "open_app_launching_steam",
                &[("target", game.name)],
            ));
        }

        if let Some(app) = crate::modules::windows_discovery::find_app_launch_target(&normalized) {
            spawn_hidden_cmd(&["/C", "start", "", &app.launch_target])?;
            return Ok(format!("Opening {}.", app.canonical_name));
        }

        if command_exists_windows(&normalized) {
            spawn_hidden_cmd(&["/C", "start", "", &normalized])?;
            return Ok(format!("Opening {}.", target));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Basic "open app by name" for macOS.
        // `open -a` works for installed apps; otherwise we'll fall back to web.
        if spawn_open(&["-a", target.trim()]).is_ok() {
            return Ok(reply_with(
                "open_app_opening",
                &[("target", target.to_string())],
            ));
        }
    }

    if let Some(url) = known_web_fallback(&normalized) {
        open_url_prefer_browser(url, false, false)?;
        return Ok(reply_with(
            "open_app_opening_web_version",
            &[("target", target.to_string())],
        ));
    }

    Err(format!("I couldn't open '{}'.", target))
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

pub fn focus_app_window(app: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let app = app.trim();
        if app.is_empty() {
            return Ok(());
        }

        let script = format!(
            "tell application {} to activate",
            serde_json::to_string(app).unwrap_or_else(|_| "\"\"".into())
        );

        Command::new("osascript")
            .args(["-e", &script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Could not focus app: {e}"))?;

        std::thread::sleep(std::time::Duration::from_millis(120));
        return Ok(());
    }

    #[cfg(windows)]
    {
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

        std::thread::sleep(std::time::Duration::from_millis(120));
    }

    Ok(())
    }
}

pub fn remember_external_app(app: &str) {
    if is_internal_companion_app(app) {
        return;
    }

    crate::modules::session_memory::set_last_external_app(app);
}

pub fn get_last_external_app() -> String {
    let state = crate::modules::session_memory::get_state();
    if state.last_external_app.trim().is_empty() {
        "unknown".into()
    } else {
        state.last_external_app
    }
}

pub fn ensure_external_focus(preferred_app: &str) -> Result<String, String> {
    if !is_internal_companion_app(preferred_app) {
        remember_external_app(preferred_app);
        focus_app_window(preferred_app)?;
        return Ok(preferred_app.to_string());
    }

    let remembered = get_last_external_app();
    if remembered != "unknown" {
        focus_app_window(&remembered)?;
    }

    Ok(remembered)
}
