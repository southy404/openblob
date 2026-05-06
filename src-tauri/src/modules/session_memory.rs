use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledTargetKind {
    Browser,
    App,
    WebService,
    MediaService,
}

impl ControlledTargetKind {
    pub fn is_browser_like(self) -> bool {
        matches!(self, Self::Browser | Self::WebService)
    }
}

impl Default for ControlledTargetKind {
    fn default() -> Self {
        Self::App
    }
}

#[derive(Debug, Clone, Default)]
pub struct ControlledSession {
    pub id: String,
    pub kind: ControlledTargetKind,
    pub app_name: String,
    pub service: String,
    pub window_title: String,
    pub process_name: String,
    pub url: String,
    pub created_by: String,
    pub last_command: String,
    pub last_updated_at: u64,
    pub is_active_controlled_target: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub last_external_app: String,
    pub last_browser_url: String,
    pub last_browser_title: String,
    pub last_browser_page_kind: String,
    pub last_command: String,
    pub last_search_query: String,
    pub last_clicked_label: String,

    pub last_suggested_title: String,
    pub last_suggested_service: String,
    pub last_suggested_url: String,
    pub last_recommendation_query: String,

    pub active_controlled_target: Option<ControlledSession>,
}

pub fn set_last_suggestion(title: &str, service: &str, url: &str, query: &str) {
    update(|s| {
        s.last_suggested_title = title.to_string();
        s.last_suggested_service = service.to_string();
        s.last_suggested_url = url.to_string();
        s.last_recommendation_query = query.to_string();
    });
}

pub fn clear_last_suggestion() {
    update(|s| {
        s.last_suggested_title.clear();
        s.last_suggested_service.clear();
        s.last_suggested_url.clear();
        s.last_recommendation_query.clear();
    });
}

static SESSION: OnceLock<Mutex<SessionState>> = OnceLock::new();

fn store() -> &'static Mutex<SessionState> {
    SESSION.get_or_init(|| Mutex::new(SessionState::default()))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn get_state() -> SessionState {
    store().lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn update<F>(f: F)
where
    F: FnOnce(&mut SessionState),
{
    if let Ok(mut state) = store().lock() {
        f(&mut state);
    }
}

pub fn set_last_external_app(app: &str) {
    update(|s| s.last_external_app = app.to_string());
}

pub fn set_last_command(cmd: &str) {
    update(|s| s.last_command = cmd.to_string());
}

pub fn set_browser_context(url: &str, title: &str, page_kind: &str) {
    update(|s| {
        s.last_browser_url = url.to_string();
        s.last_browser_title = title.to_string();
        s.last_browser_page_kind = page_kind.to_string();

        if let Some(target) = s.active_controlled_target.as_mut() {
            if target.kind.is_browser_like() && target.is_active_controlled_target {
                target.url = url.to_string();
                target.window_title = title.to_string();
                target.last_updated_at = now_millis();
            }
        }
    });
}

pub fn set_last_search_query(query: &str) {
    update(|s| s.last_search_query = query.to_string());
}

pub fn set_last_clicked_label(label: &str) {
    update(|s| s.last_clicked_label = label.to_string());
}

pub fn set_active_controlled_target(
    kind: ControlledTargetKind,
    app_name: Option<&str>,
    service: Option<&str>,
    window_title: Option<&str>,
    process_name: Option<&str>,
    url: Option<&str>,
    last_command: &str,
) {
    let updated_at = now_millis();
    let service = service.unwrap_or_default().trim().to_lowercase();
    let app_name = app_name.unwrap_or_default().trim().to_string();
    let window_title = window_title.unwrap_or_default().trim().to_string();
    let process_name = process_name.unwrap_or_default().trim().to_string();
    let url = url.unwrap_or_default().trim().to_string();

    update(|s| {
        if kind.is_browser_like() && !url.is_empty() {
            s.last_browser_url = url.clone();
        }

        if kind.is_browser_like() && !window_title.is_empty() {
            s.last_browser_title = window_title.clone();
        }

        if matches!(
            kind,
            ControlledTargetKind::App | ControlledTargetKind::MediaService
        ) && !app_name.is_empty()
        {
            s.last_external_app = app_name.clone();
        }

        s.active_controlled_target = Some(ControlledSession {
            id: format!("openblob-{updated_at}"),
            kind,
            app_name,
            service,
            window_title,
            process_name,
            url,
            created_by: "openblob".to_string(),
            last_command: last_command.trim().to_string(),
            last_updated_at: updated_at,
            is_active_controlled_target: true,
        });
    });
}

pub fn set_controlled_app(app_name: &str, last_command: &str) {
    set_active_controlled_target(
        ControlledTargetKind::App,
        Some(app_name),
        None,
        Some(app_name),
        Some(app_name),
        None,
        last_command,
    );
}

pub fn set_controlled_media_service(service: &str, app_name: &str, last_command: &str) {
    set_active_controlled_target(
        ControlledTargetKind::MediaService,
        Some(app_name),
        Some(service),
        Some(app_name),
        Some(app_name),
        None,
        last_command,
    );
}

pub fn set_controlled_browser(url: &str, title: &str, last_command: &str) {
    set_active_controlled_target(
        ControlledTargetKind::Browser,
        Some("browser"),
        None,
        Some(title),
        None,
        Some(url),
        last_command,
    );
}

pub fn set_controlled_web_service(service: &str, url: &str, title: &str, last_command: &str) {
    set_active_controlled_target(
        ControlledTargetKind::WebService,
        Some("browser"),
        Some(service),
        Some(title),
        None,
        Some(url),
        last_command,
    );
}

pub fn touch_active_controlled_target(last_command: &str) {
    update(|s| {
        if let Some(target) = s.active_controlled_target.as_mut() {
            target.last_command = last_command.trim().to_string();
            target.last_updated_at = now_millis();
        }
    });
}

pub fn active_controlled_target() -> Option<ControlledSession> {
    get_state()
        .active_controlled_target
        .filter(|target| target.is_active_controlled_target)
}

pub fn clear_active_controlled_target() {
    update(|s| s.active_controlled_target = None);
}
