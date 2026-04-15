use crate::core::legacy::voice_command_executor::execute_legacy_voice_command;
use crate::modules::command_router::CompanionAction;
use crate::modules::context::ActiveContext;

/// Legacy fallback hook.
/// Delegates to extracted legacy execution layer.
pub async fn execute_legacy_fallback(
    app: &tauri::AppHandle,
    input: &str,
    action: &CompanionAction,
    context: &ActiveContext,
) -> Result<String, String> {
    execute_legacy_voice_command(app, input, action, context).await
}