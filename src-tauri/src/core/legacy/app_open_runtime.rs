use std::process::{Command, Stdio};

use crate::modules::context::is_internal_companion_app;
use crate::modules::i18n::replies::reply_with;
use winreg::enums::HKEY_CLASSES_ROOT;
use winreg::RegKey;

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

fn protocol_registered(protocol: &str) -> bool {
    RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(protocol)
        .is_ok()
}

fn open_shell_target(target: &str) -> Result<(), String> {
    spawn_hidden_cmd(&["/C", "start", "", target])
}

fn open_protocol_uri(protocol: &str, uri: &str) -> Result<bool, String> {
    if !protocol_registered(protocol) {
        return Ok(false);
    }

    open_shell_target(uri)?;
    Ok(true)
}

pub fn open_url_prefer_browser(url: &str, new_window: bool, incognito: bool) -> Result<(), String> {
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
        "steam" => Some("https://store.steampowered.com"),
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
                open_shell_target(path)?;
                true
            } else if command_exists_windows("steam") {
                open_shell_target("steam")?;
                true
            } else if open_protocol_uri("steam", "steam://open/main")? {
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
                open_shell_target(path)?;
                true
            } else if command_exists_windows("FL64") {
                open_shell_target("FL64")?;
                true
            } else {
                false
            }
        }

        "discord" => {
            if command_exists_windows("discord") {
                open_shell_target("discord")?;
                true
            } else if open_protocol_uri("discord", "discord://-/")? {
                true
            } else {
                false
            }
        }

        "spotify" => {
            if command_exists_windows("spotify") {
                open_shell_target("spotify")?;
                true
            } else if open_protocol_uri("spotify", "spotify:")? {
                true
            } else {
                false
            }
        }

        "chrome" => {
            if command_exists_windows("chrome") {
                open_shell_target("chrome")?;
                true
            } else {
                false
            }
        }

        "edge" => {
            if command_exists_windows("msedge") {
                open_shell_target("msedge")?;
                true
            } else {
                false
            }
        }

        "explorer" => {
            open_shell_target("explorer")?;
            true
        }

        "notepad" => {
            open_shell_target("notepad")?;
            true
        }

        "paint" => {
            open_shell_target("mspaint")?;
            true
        }

        "calc" => {
            open_shell_target("calc")?;
            true
        }

        "taskmgr" => {
            open_shell_target("taskmgr")?;
            true
        }

        "settings" => {
            open_shell_target("ms-settings:")?;
            true
        }

        _ => false,
    };

    Ok(opened)
}

pub fn open_app_target(
    target: &str,
    prefer_browser: bool,
) -> Result<String, String> {
    let normalized = clean_app_target(target);

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

    if let Some(game) = crate::modules::steam_games::find_steam_game(&normalized) {
        let uri = crate::modules::steam_games::steam_launch_uri(&game.appid);
        open_shell_target(&uri)?;
        return Ok(reply_with(
            "open_app_launching_steam",
            &[("target", game.name)],
        ));
    }

    if let Some(app) = crate::modules::windows_discovery::find_app_launch_target(&normalized) {
        open_shell_target(&app.launch_target)?;
        return Ok(format!("Opening {}.", app.canonical_name));
    }

    if command_exists_windows(&normalized) {
        open_shell_target(&normalized)?;
        return Ok(format!("Opening {}.", target));
    }

    if let Some(url) = known_web_fallback(&normalized) {
        open_url_prefer_browser(url, false, false)?;
        return Ok(reply_with(
            "open_app_opening_web_version",
            &[("target", target.to_string())],
        ));
    }

    let url = generic_web_search_url(&normalized);
    open_url_prefer_browser(&url, false, false)?;
    Ok(format!(
        "I couldn't find a local app for '{}', so I searched the web.",
        target
    ))
}

pub fn play_spotify_title(query: &str) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Spotify search query was empty.".into());
    }

    let uri = format!("spotify:search:{}", urlencoding::encode(query));
    if open_protocol_uri("spotify", &uri)? {
        return Ok(format!("Searching Spotify for '{}'.", query));
    }

    let url = format!(
        "https://open.spotify.com/search/{}",
        urlencoding::encode(query)
    );
    open_url_prefer_browser(&url, false, false)?;
    Ok(format!("Opening Spotify search for '{}'.", query))
}

pub fn play_steam_title(query: &str) -> Result<String, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Steam title was empty.".into());
    }

    if let Some(game) = crate::modules::steam_games::find_steam_game(query) {
        let uri = crate::modules::steam_games::steam_launch_uri(&game.appid);
        open_shell_target(&uri)?;
        return Ok(reply_with(
            "open_app_launching_steam",
            &[("target", game.name)],
        ));
    }

    let store_url = steam_store_search_url(query);
    if protocol_registered("steam") {
        let steam_url = format!("steam://openurl/{}", store_url);
        open_shell_target(&steam_url)?;
        return Ok(format!("Searching Steam for '{}'.", query));
    }

    open_url_prefer_browser(&store_url, false, false)?;
    Ok(format!("Opening Steam store search for '{}'.", query))
}

pub fn play_on_service(service: &str, query: &str) -> Result<String, String> {
    match service.trim().to_lowercase().as_str() {
        "spotify" => play_spotify_title(query),
        "steam" => play_steam_title(query),
        other => Err(format!("Playing on '{}' is not supported yet.", other)),
    }
}

fn clean_app_target(target: &str) -> String {
    target
        .trim()
        .to_lowercase()
        .replace(" application", "")
        .replace(" app", "")
        .replace(" programm", "")
        .replace(" program", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn steam_store_search_url(query: &str) -> String {
    format!(
        "https://store.steampowered.com/search/?term={}",
        urlencoding::encode(query)
    )
}

fn generic_web_search_url(target: &str) -> String {
    format!(
        "https://www.google.com/search?q={}",
        urlencoding::encode(&format!("{target} app"))
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_app_target_removes_filler_words() {
        assert_eq!(clean_app_target(" Spotify app "), "spotify");
        assert_eq!(clean_app_target("Spotify application"), "spotify");
        assert_eq!(clean_app_target("Steam program"), "steam");
    }

    #[test]
    fn builds_stable_web_fallback_urls() {
        assert_eq!(
            steam_store_search_url("Michael Jackson Thriller"),
            "https://store.steampowered.com/search/?term=Michael%20Jackson%20Thriller"
        );
        assert_eq!(
            generic_web_search_url("unknown tool"),
            "https://www.google.com/search?q=unknown%20tool%20app"
        );
    }
}
