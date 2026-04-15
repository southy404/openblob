use crate::core::app::fallback::execute_legacy_fallback;
use crate::core::app::types::{CommandPipelineResult, CommandRoute};
use crate::core::capabilities::action_mapper::action_to_capability;
use crate::core::executor::execute::execute_capability;
use crate::core::executor::result::CapabilityResult;
use crate::modules::command_router::{parse_voice_command_with_context, CompanionAction};
use crate::modules::context::ActiveContext;

/// Full pipeline:
/// input -> parser -> action -> capability -> executor -> optional legacy fallback
pub async fn run_command_pipeline(
    app: &tauri::AppHandle,
    input: &str,
    context: &ActiveContext,
) -> Result<CommandPipelineResult, String> {
    let action = parse_voice_command_with_context(
        input,
        &context.app_name,
        &context.window_title,
        &context.domain,
    );

    if matches!(action, CompanionAction::None) {
        return Ok(CommandPipelineResult {
            route: CommandRoute::NoMatch,
            action: Some(action),
            capability: None,
            result: None,
        });
    }

    let capability = action_to_capability(&action);

    match capability.clone() {
        Some(req) => {
            let result = match execute_capability(req.clone(), context).await {
                Ok(res) => res,
                Err(err) => CapabilityResult::err(req.capability_id.clone(), err),
            };

            Ok(CommandPipelineResult {
                route: CommandRoute::NewCore,
                action: Some(action),
                capability: Some(req),
                result: Some(result),
            })
        }

        None => {
            let legacy_message = execute_legacy_fallback(app, input, &action, context).await?;

            Ok(CommandPipelineResult {
                route: CommandRoute::LegacyFallback,
                action: Some(action),
                capability: None,
                result: Some(CapabilityResult::ok(
                    "legacy.fallback",
                    legacy_message,
                )),
            })
        }
    }
}