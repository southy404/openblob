use std::sync::{Mutex, OnceLock};

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
    });
}

pub fn set_last_search_query(query: &str) {
    update(|s| s.last_search_query = query.to_string());
}

pub fn set_last_clicked_label(label: &str) {
    update(|s| s.last_clicked_label = label.to_string());
}