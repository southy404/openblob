use serde::Deserialize;

use crate::core::capabilities::registry::{
    CAP_BROWSER_SEARCH_GOOGLE, CAP_BROWSER_SEARCH_YOUTUBE, CAP_MEDIA_PLAY_PAUSE,
    CAP_SYSTEM_OPEN_APP, CAP_VISION_CAPTURE_SCREEN,
};
use crate::core::capabilities::types::CapabilityRequest;
use crate::core::executor::result::CapabilityResult;
use crate::core::permissions::policy::is_allowed;
use crate::modules::browser_automations;
use crate::modules::context::ActiveContext;
use crate::modules::system;

#[derive(Debug, Deserialize)]
struct SearchPayload {
    query: String,
}

#[derive(Debug, Deserialize)]
struct OpenAppPayload {
    target: String,
    prefer_browser: bool,
}

/// Executes a capability request against existing backend modules.
/// This is intentionally small at first: only a few capabilities are wired.
pub async fn execute_capability(
    request: CapabilityRequest,
    context: &ActiveContext,
) -> Result<CapabilityResult, String> {
    let capability_id = request.capability_id.clone();

    if !is_allowed(&capability_id, context) {
        return Ok(CapabilityResult::err(
            capability_id,
            "Capability blocked by current permission policy.",
        ));
    }

    match request.capability_id.as_str() {
        CAP_BROWSER_SEARCH_GOOGLE => {
            let payload: SearchPayload = serde_json::from_value(request.payload)
                .map_err(|e| format!("Invalid payload for browser.search_google: {e}"))?;

            let query = payload.query.trim();
            if query.is_empty() {
                return Ok(CapabilityResult::err(
                    CAP_BROWSER_SEARCH_GOOGLE,
                    "Search query was empty.",
                ));
            }

            let url = format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(query)
            );

            browser_automations::navigate_best_tab(&url).await?;

            Ok(CapabilityResult::ok(
                CAP_BROWSER_SEARCH_GOOGLE,
                format!("Searching Google for '{}'.", query),
            ))
        }

        CAP_BROWSER_SEARCH_YOUTUBE => {
            let payload: SearchPayload = serde_json::from_value(request.payload)
                .map_err(|e| format!("Invalid payload for browser.search_youtube: {e}"))?;

            let query = payload.query.trim();
            if query.is_empty() {
                return Ok(CapabilityResult::err(
                    CAP_BROWSER_SEARCH_YOUTUBE,
                    "Search query was empty.",
                ));
            }

            let url = format!(
                "https://www.youtube.com/results?search_query={}",
                urlencoding::encode(query)
            );

            browser_automations::navigate_best_tab(&url).await?;

            Ok(CapabilityResult::ok(
                CAP_BROWSER_SEARCH_YOUTUBE,
                format!("Searching YouTube for '{}'.", query),
            ))
        }

        CAP_SYSTEM_OPEN_APP => {
            let payload: OpenAppPayload = serde_json::from_value(request.payload)
                .map_err(|e| format!("Invalid payload for system.open_app: {e}"))?;

            let target = payload.target.trim();
            if target.is_empty() {
                return Ok(CapabilityResult::err(
                    CAP_SYSTEM_OPEN_APP,
                    "Open-app target was empty.",
                ));
            }

            // Placeholder execution path for now.
            // Keep this conservative until we move existing app launching
            // into a dedicated executor/backend adapter.
            let mode = if payload.prefer_browser {
                "browser-preferred"
            } else {
                "direct"
            };

            Ok(CapabilityResult::ok(
                CAP_SYSTEM_OPEN_APP,
                format!("Open app requested for '{}' ({mode}).", target),
            ))
        }

        CAP_VISION_CAPTURE_SCREEN => Ok(CapabilityResult::ok(
            CAP_VISION_CAPTURE_SCREEN,
            "Screenshot capability requested.",
        )),

        CAP_MEDIA_PLAY_PAUSE => {
            system::media_play_pause()?;

            Ok(CapabilityResult::ok(
                CAP_MEDIA_PLAY_PAUSE,
                "Toggled media playback.",
            ))
        }

        _ => Ok(CapabilityResult::err(
            capability_id,
            "Capability is not implemented in executor yet.",
        )),
    }
}