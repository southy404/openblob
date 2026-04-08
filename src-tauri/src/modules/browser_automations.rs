use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const DEBUG_PORT: u16 = 9222;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    pub websocket_debugger_url: Option<String>,
    #[serde(rename = "type")]
    pub tab_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BrowserContext {
    pub title: String,
    pub url: String,
    pub domain: String,
    pub page_kind: String,
    pub visible_links: Vec<String>,
    pub visible_buttons: Vec<String>,
    pub visible_inputs: Vec<String>,
}

fn debug_json_url(path: &str) -> String {
    format!("http://127.0.0.1:{DEBUG_PORT}{path}")
}

pub async fn list_tabs() -> Result<Vec<BrowserTab>, String> {
    let url = debug_json_url("/json");
    let client = Client::new();

    let tabs = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Browser debugging nicht erreichbar: {e}"))?
        .json::<Vec<BrowserTab>>()
        .await
        .map_err(|e| format!("Tabs konnten nicht gelesen werden: {e}"))?;

    Ok(tabs)
}

pub async fn new_tab(target_url: &str) -> Result<(), String> {
    let url = debug_json_url(&format!("/json/new?{target_url}"));
    let client = Client::new();

    client
        .put(url)
        .send()
        .await
        .map_err(|e| format!("Neuer Tab konnte nicht geöffnet werden: {e}"))?;

    Ok(())
}

pub async fn activate_tab(tab_id: &str) -> Result<(), String> {
    let url = debug_json_url(&format!("/json/activate/{tab_id}"));
    let client = Client::new();

    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Tab konnte nicht aktiviert werden: {e}"))?;

    Ok(())
}

pub async fn close_tab(tab_id: &str) -> Result<(), String> {
    let url = debug_json_url(&format!("/json/close/{tab_id}"));
    let client = Client::new();

    client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Tab konnte nicht geschlossen werden: {e}"))?;

    Ok(())
}

pub async fn get_active_tab() -> Result<BrowserTab, String> {
    let tabs = list_tabs().await?;

    let tab = tabs
        .into_iter()
        .find(|t| {
            t.tab_type.as_deref() == Some("page")
                && t.websocket_debugger_url.is_some()
                && !t.url.starts_with("devtools://")
        })
        .ok_or("Kein aktiver Browser-Tab gefunden.")?;

    Ok(tab)
}

pub async fn eval_js(tab: &BrowserTab, expression: &str) -> Result<Value, String> {
    let ws_url = tab
        .websocket_debugger_url
        .clone()
        .ok_or("Tab hat keine Debug-WebSocket-URL.")?;

    let (mut socket, _) = connect_async(&ws_url)
        .await
        .map_err(|e| format!("WebSocket-Verbindung fehlgeschlagen: {e}"))?;

    let msg = json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": {
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true
        }
    });

    socket
        .send(Message::Text(msg.to_string()))
        .await
        .map_err(|e| format!("JS konnte nicht gesendet werden: {e}"))?;

    while let Some(message) = socket.next().await {
        let message = message.map_err(|e| format!("WebSocket-Lesefehler: {e}"))?;

        if let Message::Text(text) = message {
            let value: Value =
                serde_json::from_str(&text).map_err(|e| format!("CDP JSON Fehler: {e}"))?;

            if value.get("id").and_then(|v| v.as_i64()) == Some(1) {
                if let Some(result) = value
                    .get("result")
                    .and_then(|r| r.get("result"))
                    .and_then(|r| r.get("value"))
                {
                    return Ok(result.clone());
                }

                return Ok(value);
            }
        }
    }

    Err("Keine Antwort vom Browser erhalten.".into())
}

pub async fn navigate_active_tab(url: &str) -> Result<(), String> {
    let tab = get_active_tab().await?;
    let js = format!("window.location.href = {:?}", url);
    let _ = eval_js(&tab, &js).await?;
    Ok(())
}

pub async fn navigate_back() -> Result<String, String> {
    let tab = get_active_tab().await?;
    let _ = eval_js(&tab, "history.back(); true").await?;
    Ok("Eine Seite zurück.".into())
}

pub async fn navigate_forward() -> Result<String, String> {
    let tab = get_active_tab().await?;
    let _ = eval_js(&tab, "history.forward(); true").await?;
    Ok("Eine Seite vor.".into())
}

pub async fn scroll_by(amount: i32) -> Result<String, String> {
    let tab = get_active_tab().await?;
    let js = format!("window.scrollBy({{ top: {amount}, behavior: 'smooth' }}); true");
    let _ = eval_js(&tab, &js).await?;
    Ok("Seite gescrollt.".into())
}

pub async fn get_browser_context() -> Result<BrowserContext, String> {
    let tab = get_active_tab().await?;

    let script = r#"
(() => {
  const norm = (s) => (s || "").replace(/\s+/g, " ").trim();

  const links = [...document.querySelectorAll("a")]
    .map((el) => norm(el.innerText || el.textContent || el.getAttribute("title") || ""))
    .filter(Boolean)
    .slice(0, 30);

  const buttons = [...document.querySelectorAll("button, [role='button'], input[type='submit']")]
    .map((el) => norm(el.innerText || el.textContent || el.value || el.getAttribute("aria-label") || ""))
    .filter(Boolean)
    .slice(0, 30);

  const inputs = [...document.querySelectorAll("input, textarea")]
    .map((el) => norm(el.placeholder || el.getAttribute("aria-label") || el.name || el.id || ""))
    .filter(Boolean)
    .slice(0, 20);

  let pageKind = "generic";
  const url = window.location.href;

  if (url.includes("youtube.com/results")) pageKind = "youtube_results";
  else if (url.includes("youtube.com/watch")) pageKind = "youtube_watch";
  else if (url.includes("google.com/search")) pageKind = "google_results";
  else if (document.querySelector("form input[type='password']")) pageKind = "login";
  else if (document.querySelector("article")) pageKind = "article";

  return {
    title: document.title || "",
    url,
    domain: location.hostname || "",
    page_kind: pageKind,
    visible_links: links,
    visible_buttons: buttons,
    visible_inputs: inputs
  };
})()
"#;

    let result = eval_js(&tab, script).await?;
    serde_json::from_value(result).map_err(|e| format!("Browser-Kontext konnte nicht gelesen werden: {e}"))
}

pub async fn type_in_best_input(text: &str) -> Result<String, String> {
    let tab = get_active_tab().await?;
    let safe_text = serde_json::to_string(text).map_err(|e| e.to_string())?;

    let script = format!(
        r#"
(() => {{
  const candidates = [
    ...document.querySelectorAll("input:not([type='hidden']):not([type='submit']), textarea, [contenteditable='true']")
  ];

  const visible = candidates.filter(el => {{
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  }});

  const el = visible[0];
  if (!el) return {{ ok: false }};

  el.focus();

  if ("value" in el) {{
    el.value = {safe_text};
    el.dispatchEvent(new Event("input", {{ bubbles: true }}));
    el.dispatchEvent(new Event("change", {{ bubbles: true }}));
  }} else {{
    el.textContent = {safe_text};
    el.dispatchEvent(new Event("input", {{ bubbles: true }}));
  }}

  return {{ ok: true }};
}})()
"#
    );

    let result = eval_js(&tab, &script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok("Text eingegeben.".into())
    } else {
        Err("Kein passendes Eingabefeld gefunden.".into())
    }
}

pub async fn submit_best_form() -> Result<String, String> {
    let tab = get_active_tab().await?;

    let script = r#"
(() => {
  const active = document.activeElement;
  if (active && active.form) {
    active.form.requestSubmit ? active.form.requestSubmit() : active.form.submit();
    return { ok: true };
  }

  const form = document.querySelector("form");
  if (form) {
    form.requestSubmit ? form.requestSubmit() : form.submit();
    return { ok: true };
  }

  const btn = [...document.querySelectorAll("button, input[type='submit']")].find(Boolean);
  if (btn) {
    btn.click();
    return { ok: true };
  }

  return { ok: false };
})()
"#;

    let result = eval_js(&tab, script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok("Formular abgeschickt.".into())
    } else {
        Err("Kein Formular oder Submit-Button gefunden.".into())
    }
}

pub async fn click_link_by_text(text: &str, new_tab: bool) -> Result<String, String> {
    let tab = get_active_tab().await?;
    let safe_text = serde_json::to_string(text).map_err(|e| e.to_string())?;

    let script = format!(
        r#"
(() => {{
  const wanted = {safe_text}.toLowerCase();

  const normalize = (s) =>
    (s || "")
      .toLowerCase()
      .replace(/[^\p{{L}}\p{{N}}\s]/gu, " ")
      .replace(/\s+/g, " ")
      .trim();

  const target = normalize(wanted);
  const links = [...document.querySelectorAll('a, button, [role="button"]')];

  let best = null;
  let bestScore = -1;

  function score(label, target) {{
    if (!label || !target) return 0;
    if (label === target) return 1000;
    if (label.includes(target)) return 800 + target.length;
    const labelWords = new Set(label.split(" "));
    const targetWords = target.split(" ");
    let hits = 0;
    for (const w of targetWords) {{
      if (labelWords.has(w)) hits++;
    }}
    return hits * 100;
  }}

  for (const link of links) {{
    const text = normalize(
      link.textContent ||
      link.getAttribute("title") ||
      link.getAttribute("aria-label") ||
      link.href ||
      ""
    );
    const current = score(text, target);
    if (current > bestScore) {{
      bestScore = current;
      best = link;
    }}
  }}

  if (!best) return {{ ok: false }};

  if ({new_tab}) {{
    const href = best.href || best.getAttribute("href");
    if (href) {{
      window.open(href, "_blank");
      return {{
        ok: true,
        clickedText: best.textContent || best.getAttribute("title") || href
      }};
    }}
  }}

  best.click();

  return {{
    ok: true,
    clickedText: best.textContent || best.getAttribute("title") || best.href || ""
  }};
}})()
"#
    );

    let result = eval_js(&tab, &script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        let clicked = result
            .get("clickedText")
            .and_then(|v| v.as_str())
            .unwrap_or("Element");
        Ok(format!("Geöffnet: {}.", clicked))
    } else {
        Err("Kein passender Link oder Button gefunden.".into())
    }
}

pub async fn click_nth_result(index: usize) -> Result<String, String> {
    let tab = get_active_tab().await?;

    let script = format!(
        r#"
(() => {{
  const items = [
    ...document.querySelectorAll('a#video-title, a h3, a[href]')
  ].filter(Boolean);

  const el = items[{index}];
  if (!el) return {{ ok: false }};

  const clickable = el.closest('a') || el;
  clickable.click();

  return {{
    ok: true,
    text: clickable.textContent || clickable.getAttribute("title") || clickable.href || ""
  }};
}})()
"#
    );

    let result = eval_js(&tab, &script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        let text = result
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("Ergebnis");
        Ok(format!("Ergebnis geöffnet: {}.", text))
    } else {
        Err(format!("Ergebnis {} nicht gefunden.", index + 1))
    }
}

pub async fn click_best_match(text: &str) -> Result<String, String> {
    click_link_by_text(text, false).await
}

pub async fn youtube_play_best_match(query: &str) -> Result<String, String> {
    let tab = get_active_tab().await?;
    let safe_query = serde_json::to_string(query).map_err(|e| e.to_string())?;

    let script = format!(
        r#"
(() => {{
  const wanted = {safe_query}.toLowerCase();

  const normalize = (s) =>
    (s || "")
      .toLowerCase()
      .replace(/[^\p{{L}}\p{{N}}\s]/gu, " ")
      .replace(/\s+/g, " ")
      .trim();

  const target = normalize(wanted);

  const candidates = [...document.querySelectorAll('a#video-title, a.ytd-video-renderer')];

  let best = null;
  let bestScore = -1;

  function score(title, target) {{
    if (!title || !target) return 0;
    if (title === target) return 1000;
    if (title.includes(target)) return 800 + target.length;
    const titleWords = new Set(title.split(" "));
    const targetWords = target.split(" ");
    let hits = 0;
    for (const w of targetWords) {{
      if (titleWords.has(w)) hits++;
    }}
    return hits * 100;
  }}

  for (const el of candidates) {{
    const title = normalize(el.textContent || el.getAttribute("title") || "");
    const current = score(title, target);
    if (current > bestScore) {{
      bestScore = current;
      best = el;
    }}
  }}

  if (best) {{
    best.click();
    return {{
      ok: true,
      clickedTitle: best.textContent || best.getAttribute("title") || ""
    }};
  }}

  return {{ ok: false }};
}})()
"#
    );

    let result = eval_js(&tab, &script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        let title = result
            .get("clickedTitle")
            .and_then(|v| v.as_str())
            .unwrap_or("Video");
        return Ok(format!("Spiele {}.", title));
    }

    Err("Kein passendes YouTube-Video gefunden.".into())
}

pub async fn youtube_search(query: &str, new_tab_mode: bool) -> Result<(), String> {
    let url = format!(
        "https://www.youtube.com/results?search_query={}",
        urlencoding::encode(query)
    );

    if new_tab_mode {
        new_tab(&url).await?;
    } else {
        navigate_active_tab(&url).await?;
    }

    Ok(())
}

pub async fn click_selector(selector: &str) -> Result<String, String> {
    let tab = get_active_tab().await?;
    let safe_selector = serde_json::to_string(selector).map_err(|e| e.to_string())?;

    let script = format!(
        r#"
(() => {{
  const el = document.querySelector({safe_selector});
  if (!el) return {{ ok: false }};
  el.click();
  return {{ ok: true }};
}})()
"#
    );

    let result = eval_js(&tab, &script).await?;
    if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok("Element geklickt.".into())
    } else {
        Err("Element nicht gefunden.".into())
    }
}