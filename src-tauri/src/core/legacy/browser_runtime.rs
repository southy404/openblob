use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::modules::browser_automations;
use crate::modules::i18n::replies::{reply, reply_with};
use crate::modules::session_memory;

fn is_port_open() -> bool {
    TcpStream::connect("127.0.0.1:9222").is_ok()
}

async fn is_debug_browser_running() -> bool {
    if !is_port_open() {
        return false;
    }

    let client = reqwest::Client::new();

    match client.get("http://127.0.0.1:9222/json").send().await {
        Ok(res) => res.status().is_success(),
        Err(_) => false,
    }
}

fn ensure_user_data_dir(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("Konnte Browser-Profilordner nicht anlegen: {e}"))
}

fn spawn_browser_process(exe_path: &str, user_data: &str) -> Result<(), String> {
    Command::new(exe_path)
        .args([
            "--remote-debugging-port=9222",
            "--remote-debugging-address=127.0.0.1",
            &format!("--user-data-dir={}", user_data),
            "--no-first-run",
            "--no-default-browser-check",
            "https://www.google.com",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Browser konnte nicht gestartet werden: {e}"))?;

    Ok(())
}

fn open_url_normal_browser(url: &str, new_window: bool, incognito: bool) -> Result<(), String> {
    let chrome_candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    let edge_candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    for path in chrome_candidates {
        if Path::new(path).exists() {
            let mut cmd = Command::new(path);

            if incognito {
                cmd.arg("--incognito");
            }
            if new_window {
                cmd.arg("--new-window");
            }

            cmd.arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Chrome konnte nicht normal geöffnet werden: {e}"))?;

            return Ok(());
        }
    }

    for path in edge_candidates {
        if Path::new(path).exists() {
            let mut cmd = Command::new(path);

            if incognito {
                cmd.arg("-inprivate");
            }
            if new_window {
                cmd.arg("--new-window");
            }

            cmd.arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("Edge konnte nicht normal geöffnet werden: {e}"))?;

            return Ok(());
        }
    }

    Command::new("explorer")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("URL konnte nicht geöffnet werden: {e}"))?;

    Ok(())
}

fn spawn_debug_browser() -> Result<(), String> {
    let chrome_candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];

    let edge_candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    let user_data = r"D:\companion-browser";

    ensure_user_data_dir(user_data)?;

    for path in chrome_candidates {
        if Path::new(path).exists() {
            if let Ok(()) = spawn_browser_process(path, user_data) {
                return Ok(());
            }
        }
    }

    for path in edge_candidates {
        if Path::new(path).exists() {
            if let Ok(()) = spawn_browser_process(path, user_data) {
                return Ok(());
            }
        }
    }

    Err("Kein Chrome oder Edge an den erwarteten Pfaden gefunden oder startbar.".into())
}

pub async fn ensure_debug_browser() -> Result<(), String> {
    if is_debug_browser_running().await {
        return Ok(());
    }

    if is_port_open() && !is_debug_browser_running().await {
        return Err(
            "Port 9222 ist erreichbar, aber der Browser liefert keine gültige Debug-Antwort. Bitte vorhandene Chrome-/Edge-Prozesse schließen und erneut versuchen."
                .into(),
        );
    }

    spawn_debug_browser()?;

    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if is_debug_browser_running().await {
            return Ok(());
        }
    }

    Err(
        "Browser debugging nicht erreichbar. Chrome/Edge wurde entweder nicht gestartet, sofort beendet oder Port 9222 wurde nicht gebunden."
            .into(),
    )
}

async fn open_or_navigate_debug_url(url: &str) -> Result<(), String> {
    if ensure_debug_browser().await.is_ok() {
        if browser_automations::navigate_best_tab(url).await.is_err() {
            browser_automations::new_tab(url).await?;
        }
    } else {
        open_url_normal_browser(url, false, false)?;
    }

    Ok(())
}

pub async fn browser_new_tab_with_url(url: String) -> Result<String, String> {
    if ensure_debug_browser().await.is_ok() {
        browser_automations::new_tab(&url).await?;
        return Ok(reply("browser_new_tab_opened"));
    }

    open_url_normal_browser(&url, true, false)?;
    Ok(reply("browser_new_tab_opened"))
}

pub async fn youtube_search_and_play(query: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::youtube_search(&query, false).await?;
    tokio::time::sleep(Duration::from_millis(1500)).await;
    browser_automations::youtube_play_best_match(&query).await
}

pub async fn youtube_play_best_match(title: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::youtube_play_best_match(&title).await
}

pub async fn browser_close_tab_by_index(index: usize) -> Result<String, String> {
    ensure_debug_browser().await?;
    let tabs = browser_automations::list_tabs().await?;

    let page_tabs: Vec<_> = tabs
        .into_iter()
        .filter(|t| t.tab_type.as_deref() == Some("page"))
        .collect();

    let tab = page_tabs
        .get(index)
        .ok_or_else(|| format!("Tab {} nicht gefunden.", index + 1))?;

    browser_automations::close_tab(&tab.id).await?;
    Ok(reply_with(
        "browser_tab_closed_by_index",
        &[("index", (index + 1).to_string())],
    ))
}

pub async fn browser_open_url(
    url: String,
    new_tab: bool,
    new_window: bool,
    incognito: bool,
) -> Result<String, String> {
    match ensure_debug_browser().await {
        Ok(()) => {
            if incognito {
                return Err(reply("browser_incognito_not_supported"));
            }

            if new_window {
                browser_automations::new_tab(&url).await?;
                return Ok(reply("browser_url_opened_new_window"));
            }

            if new_tab {
                browser_automations::new_tab(&url).await?;
                return Ok(reply("browser_url_opened_new_tab"));
            }

            if browser_automations::navigate_best_tab(&url).await.is_err() {
                browser_automations::new_tab(&url).await?;
            }

            Ok(reply("browser_url_opened"))
        }
        Err(_) => {
            open_url_normal_browser(&url, new_window || new_tab, incognito)?;
            if new_window {
                Ok(reply("browser_url_opened_new_window"))
            } else if new_tab {
                Ok(reply("browser_url_opened_new_tab"))
            } else {
                Ok(reply("browser_url_opened"))
            }
        }
    }
}

pub async fn browser_get_context() -> Result<browser_automations::BrowserContext, String> {
    ensure_debug_browser().await?;
    let ctx = browser_automations::get_browser_context().await?;
    session_memory::set_browser_context(&ctx.url, &ctx.title, &ctx.page_kind);
    Ok(ctx)
}

pub async fn browser_back() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::navigate_back().await
}

pub async fn browser_forward() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::navigate_forward().await
}

pub async fn browser_scroll_down() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::scroll_by(700).await
}

pub async fn browser_scroll_up() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::scroll_by(-700).await
}

pub async fn browser_type_text(text: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    session_memory::set_last_search_query(&text);
    browser_automations::type_in_best_input(&text).await
}

pub async fn browser_submit() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::submit_best_form().await
}

pub async fn browser_click_best_match(text: String) -> Result<String, String> {
    ensure_debug_browser().await?;
    session_memory::set_last_clicked_label(&text);
    browser_automations::click_best_match(&text).await
}

pub async fn browser_click_link_by_text(text: String, new_tab: bool) -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::click_link_by_text(&text, new_tab).await
}

pub async fn browser_click_first_result() -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::click_nth_result(0).await
}

pub async fn browser_click_nth_result(index: usize) -> Result<String, String> {
    ensure_debug_browser().await?;
    browser_automations::click_nth_result(index).await
}

pub async fn google_search(query: &str) -> Result<String, String> {
    let url = format!(
        "https://www.google.com/search?q={}",
        urlencoding::encode(query)
    );

    open_or_navigate_debug_url(&url).await?;

    Ok(reply_with(
        "browser_google_search",
        &[("query", query.to_string())],
    ))
}

pub async fn youtube_search(query: &str) -> Result<String, String> {
    let url = format!(
        "https://www.youtube.com/results?search_query={}",
        urlencoding::encode(query)
    );

    open_or_navigate_debug_url(&url).await?;

    Ok(reply_with(
        "browser_youtube_search",
        &[("query", query.to_string())],
    ))
}

pub async fn youtube_play_title(title: &str) -> Result<String, String> {
    youtube_search_and_play(title.to_string()).await
}

pub async fn new_tab() -> Result<String, String> {
    if ensure_debug_browser().await.is_ok() {
        browser_automations::new_tab("https://www.google.com").await?;
        return Ok(reply("browser_new_tab_opened"));
    }

    open_url_normal_browser("https://www.google.com", true, false)?;
    Ok(reply("browser_new_tab_opened"))
}

pub async fn close_active_tab() -> Result<String, String> {
    ensure_debug_browser().await?;
    let tab = browser_automations::get_active_tab().await?;
    browser_automations::close_tab(&tab.id).await?;
    Ok(reply("browser_active_tab_closed"))
}

pub async fn new_window() -> Result<String, String> {
    if ensure_debug_browser().await.is_ok() {
        browser_automations::new_tab("https://www.google.com").await?;
        return Ok(reply("browser_new_tab_opened"));
    }

    open_url_normal_browser("https://www.google.com", true, false)?;
    Ok(reply("browser_new_tab_opened"))
}
