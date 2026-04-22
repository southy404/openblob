use serde_json::json;

use crate::modules::command_router::CompanionAction;

use super::registry::{
    CAP_BROWSER_SEARCH_GOOGLE,
    CAP_BROWSER_SEARCH_YOUTUBE,
    CAP_MEDIA_PLAY_PAUSE,
    CAP_SYSTEM_CONFIRM_PENDING,
    CAP_SYSTEM_LOCK_SCREEN,
    CAP_SYSTEM_OPEN_APP,
    CAP_SYSTEM_OPEN_DOWNLOADS,
    CAP_SYSTEM_OPEN_EXPLORER,
    CAP_SYSTEM_OPEN_SETTINGS,
    CAP_SYSTEM_RESTART,
    CAP_SYSTEM_SHUTDOWN,
    CAP_VISION_CAPTURE_SCREEN,
    CAP_SYSTEM_CANCEL_PENDING,
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

        CompanionAction::OpenDownloads => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_OPEN_DOWNLOADS))
        }

        CompanionAction::OpenSettings => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_OPEN_SETTINGS))
        }

        CompanionAction::OpenExplorer => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_OPEN_EXPLORER))
        }

        CompanionAction::LockScreen => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_LOCK_SCREEN))
        }

        CompanionAction::Shutdown => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_SHUTDOWN))
        }

        CompanionAction::Restart => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_RESTART))
        }

        CompanionAction::ConfirmPendingAction => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_CONFIRM_PENDING))
        }

        CompanionAction::CancelPendingAction => {
            Some(CapabilityRequest::empty(CAP_SYSTEM_CANCEL_PENDING))
        }

        _ => None,
    }
}