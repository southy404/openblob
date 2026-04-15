use serde_json::json;

use crate::modules::command_router::CompanionAction;

use super::registry::{
    CAP_BROWSER_SEARCH_GOOGLE, CAP_BROWSER_SEARCH_YOUTUBE, CAP_MEDIA_PLAY_PAUSE,
    CAP_SYSTEM_OPEN_APP, CAP_VISION_CAPTURE_SCREEN,
};
use super::types::CapabilityRequest;

/// Temporary compatibility bridge:
/// existing parser output -> new capability request.
///
/// This lets us keep the current command router untouched for now,
/// while gradually moving execution to the new core architecture.
pub fn action_to_capability(action: &CompanionAction) -> Option<CapabilityRequest> {
    match action {
        CompanionAction::GoogleSearch { query } => Some(CapabilityRequest::new(
            CAP_BROWSER_SEARCH_GOOGLE,
            json!({ "query": query }),
        )),

        CompanionAction::YouTubeSearch { query } => Some(CapabilityRequest::new(
            CAP_BROWSER_SEARCH_YOUTUBE,
            json!({ "query": query }),
        )),

        CompanionAction::OpenApp {
            target,
            prefer_browser,
        } => Some(CapabilityRequest::new(
            CAP_SYSTEM_OPEN_APP,
            json!({
                "target": target,
                "prefer_browser": prefer_browser
            }),
        )),

        CompanionAction::TakeScreenshot => {
            Some(CapabilityRequest::empty(CAP_VISION_CAPTURE_SCREEN))
        }

        CompanionAction::MediaPlayPause => {
            Some(CapabilityRequest::empty(CAP_MEDIA_PLAY_PAUSE))
        }

        _ => None,
    }
}