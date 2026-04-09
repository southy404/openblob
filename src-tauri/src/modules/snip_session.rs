use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct SnipSession {
    pub image_path: String,
    pub comment: String,
    pub context_app: String,
    pub context_domain: String,
    pub window_title: String,
}

static SNIP_STATE: OnceLock<Mutex<Option<SnipSession>>> = OnceLock::new();

fn snip_store() -> &'static Mutex<Option<SnipSession>> {
    SNIP_STATE.get_or_init(|| Mutex::new(None))
}

pub fn set_snip(session: SnipSession) {
    let mut state = snip_store().lock().unwrap();
    *state = Some(session);
}

pub fn get_snip() -> Option<SnipSession> {
    snip_store().lock().unwrap().clone()
}

pub fn clear_snip() {
    let mut state = snip_store().lock().unwrap();
    *state = None;
}