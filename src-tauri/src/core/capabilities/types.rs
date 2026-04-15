use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable identifier for a capability.
/// Keep these human-readable and namespace-like:
/// - browser.search_google
/// - system.open_app
/// - vision.capture_screen
pub type CapabilityId = &'static str;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionLevel {
    Safe,
    Confirm,
    Sensitive,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityContext {
    Any,
    Desktop,
    Browser,
    Editor,
    Media,
    Game,
    Companion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub capability_id: String,
    pub payload: Value,
}

impl CapabilityRequest {
    pub fn new(capability_id: impl Into<String>, payload: Value) -> Self {
        Self {
            capability_id: capability_id.into(),
            payload,
        }
    }

    pub fn empty(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            payload: Value::Null,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityDescriptor {
    pub id: CapabilityId,
    pub title: &'static str,
    pub description: &'static str,
    pub permission: PermissionLevel,
    pub contexts: &'static [CapabilityContext],
    pub unstable: bool,
}