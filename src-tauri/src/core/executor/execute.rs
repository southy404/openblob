use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::core::capabilities::registry::{
    CAP_BROWSER_SEARCH_GOOGLE,
    CAP_BROWSER_SEARCH_YOUTUBE,
    CAP_MEDIA_PLAY_PAUSE,
    CAP_SYSTEM_CANCEL_PENDING,
    CAP_SYSTEM_CONFIRM_PENDING,
    CAP_SYSTEM_LOCK_SCREEN,
    CAP_SYSTEM_OPEN_APP,
    CAP_SYSTEM_OPEN_DOWNLOADS,
    CAP_SYSTEM_OPEN_EXPLORER,
    CAP_SYSTEM_OPEN_SETTINGS,
    CAP_SYSTEM_RESTART,
    CAP_SYSTEM_SHUTDOWN,
    CAP_VISION_CAPTURE_SCREEN,
};
use crate::core::capabilities::types::CapabilityRequest;
use crate::core::executor::result::CapabilityResult;
use crate::core::permissions::policy::is_allowed;
use crate::modules::browser_automations;
use crate::modules::context::ActiveContext;
use crate::modules::system;
use crate::core::legacy::app_open_runtime;


const PENDING_ACTION_TTL: Duration = Duration::from_secs(12);

#[derive(Debug, Clone)]
struct PendingAction {
    capability_id: String,
    created_at: Instant,
}

static PENDING_ACTION: LazyLock<Mutex<Option<PendingAction>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Deserialize)]
struct SearchPayload {
    query: String,
}

#[derive(Debug, Deserialize)]
struct OpenAppPayload {
    target: String,
    #[serde(default)]
    prefer_browser: bool,
}

fn ok(id: impl Into<String>, message: impl Into<String>) -> CapabilityResult {
    CapabilityResult::ok(id.into(), message.into())
}

fn err(id: impl Into<String>, message: impl Into<String>) -> CapabilityResult {
    CapabilityResult::err(id.into(), message.into())
}

fn set_pending_action(capability_id: &str) -> Result<(), String> {
    let mut pending = PENDING_ACTION
        .lock()
        .map_err(|_| "Failed to lock pending action state.".to_string())?;

    *pending = Some(PendingAction {
        capability_id: capability_id.to_string(),
        created_at: Instant::now(),
    });

    Ok(())
}

fn take_pending_action() -> Result<Option<PendingAction>, String> {
    let mut pending = PENDING_ACTION
        .lock()
        .map_err(|_| "Failed to lock pending action state.".to_string())?;

    if let Some(current) = pending.as_ref() {
        if current.created_at.elapsed() > PENDING_ACTION_TTL {
            *pending = None;
            return Ok(None);
        }
    }

    Ok(pending.take())
}

fn confirm_message_for(capability_id: &str) -> &'static str {
    match capability_id {
        CAP_SYSTEM_SHUTDOWN => "Are you sure? Say 'yes' to shut down the PC.",
        CAP_SYSTEM_RESTART => "Are you sure? Say 'yes' to restart the PC.",
        _ => "Are you sure? Say 'yes' to continue.",
    }
}

fn execute_confirmed_pending_action(
    pending: PendingAction,
) -> Result<CapabilityResult, String> {
    match pending.capability_id.as_str() {
        CAP_SYSTEM_SHUTDOWN => {
            system::shutdown_pc()?;
            Ok(ok(
                CAP_SYSTEM_CONFIRM_PENDING,
                "Shutting down the PC.",
            ))
        }
        CAP_SYSTEM_RESTART => {
            system::restart_pc()?;
            Ok(ok(
                CAP_SYSTEM_CONFIRM_PENDING,
                "Restarting the PC.",
            ))
        }
        _ => Ok(err(
            CAP_SYSTEM_CONFIRM_PENDING,
            "The pending action is not executable.",
        )),
    }
}

fn clear_pending_action() -> Result<bool, String> {
    let mut pending = PENDING_ACTION
        .lock()
        .map_err(|_| "Failed to lock pending action state.".to_string())?;

    let had_pending = pending.is_some();
    *pending = None;

    Ok(had_pending)
}

/// Executes a capability request against existing backend modules.
pub async fn execute_capability(
    request: CapabilityRequest,
    context: &ActiveContext,
) -> Result<CapabilityResult, String> {
    let capability_id = request.capability_id.clone();

    if !is_allowed(&capability_id, context) {
        return Ok(err(
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
                return Ok(err(
                    CAP_BROWSER_SEARCH_GOOGLE,
                    "Search query was empty.",
                ));
            }

            let url = format!(
                "https://www.google.com/search?q={}",
                urlencoding::encode(query)
            );

            browser_automations::navigate_best_tab(&url).await?;

            Ok(ok(
                CAP_BROWSER_SEARCH_GOOGLE,
                format!("Searching Google for '{}'.", query),
            ))
        }

        CAP_BROWSER_SEARCH_YOUTUBE => {
            let payload: SearchPayload = serde_json::from_value(request.payload)
                .map_err(|e| format!("Invalid payload for browser.search_youtube: {e}"))?;

            let query = payload.query.trim();
            if query.is_empty() {
                return Ok(err(
                    CAP_BROWSER_SEARCH_YOUTUBE,
                    "Search query was empty.",
                ));
            }

            let url = format!(
                "https://www.youtube.com/results?search_query={}",
                urlencoding::encode(query)
            );

            browser_automations::navigate_best_tab(&url).await?;

            Ok(ok(
                CAP_BROWSER_SEARCH_YOUTUBE,
                format!("Searching YouTube for '{}'.", query),
            ))
        }

        CAP_SYSTEM_OPEN_APP => {
            let payload: OpenAppPayload = serde_json::from_value(request.payload)
                .map_err(|e| format!("Invalid payload for system.open_app: {e}"))?;

            let target = payload.target.trim();
            if target.is_empty() {
                return Ok(err(
                    CAP_SYSTEM_OPEN_APP,
                    "Open-app target was empty.",
                ));
            }

            let message = app_open_runtime::open_app_target(
                target,
                payload.prefer_browser,
            )?;

            Ok(ok(CAP_SYSTEM_OPEN_APP, message))
        }

        CAP_SYSTEM_OPEN_DOWNLOADS => {
            system::open_downloads()?;
            Ok(ok(
                CAP_SYSTEM_OPEN_DOWNLOADS,
                "Opened the Downloads folder.",
            ))
        }

        CAP_SYSTEM_OPEN_SETTINGS => {
            system::open_settings()?;
            Ok(ok(
                CAP_SYSTEM_OPEN_SETTINGS,
                "Opened Windows Settings.",
            ))
        }

        CAP_SYSTEM_OPEN_EXPLORER => {
            system::open_explorer()?;
            Ok(ok(
                CAP_SYSTEM_OPEN_EXPLORER,
                "Opened File Explorer.",
            ))
        }

        CAP_SYSTEM_LOCK_SCREEN => {
            system::lock_screen()?;
            Ok(ok(
                CAP_SYSTEM_LOCK_SCREEN,
                "Locking the screen.",
            ))
        }

        CAP_SYSTEM_SHUTDOWN | CAP_SYSTEM_RESTART => {
            let pending_id = request.capability_id.clone();
            let message = confirm_message_for(pending_id.as_str());

            set_pending_action(pending_id.as_str())?;
            Ok(ok(pending_id, message))
        }

        CAP_SYSTEM_CONFIRM_PENDING => {
            let pending = take_pending_action()?;

            match pending {
                Some(pending) => execute_confirmed_pending_action(pending),
                None => Ok(err(
                    CAP_SYSTEM_CONFIRM_PENDING,
                    "There is no pending action to confirm.",
                )),
            }
        }

        CAP_SYSTEM_CANCEL_PENDING => {
            let had_pending = clear_pending_action()?;

            if had_pending {
                Ok(ok(
                    CAP_SYSTEM_CANCEL_PENDING,
                    "Cancelled the pending action.",
                ))
            } else {
                Ok(err(
                    CAP_SYSTEM_CANCEL_PENDING,
                    "There is no pending action to cancel.",
                ))
            }
        }

        CAP_VISION_CAPTURE_SCREEN => Ok(ok(
            CAP_VISION_CAPTURE_SCREEN,
            "Screenshot capability requested.",
        )),

        CAP_MEDIA_PLAY_PAUSE => {
            system::media_play_pause()?;
            Ok(ok(
                CAP_MEDIA_PLAY_PAUSE,
                "Toggled media playback.",
            ))
        }

        _ => Ok(err(
            capability_id,
            "Capability is not implemented in executor yet.",
        )),
    }
}