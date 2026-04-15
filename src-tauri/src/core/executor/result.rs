use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub success: bool,
    pub message: String,
    pub route: String,
    pub data: Option<Value>,
}

impl CapabilityResult {
    pub fn ok(route: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            route: route.into(),
            data: None,
        }
    }

    pub fn ok_with_data(
        route: impl Into<String>,
        message: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            success: true,
            message: message.into(),
            route: route.into(),
            data: Some(data),
        }
    }

    pub fn err(route: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            route: route.into(),
            data: None,
        }
    }
}