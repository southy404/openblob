use crate::core::capabilities::registry::find_capability;
use crate::core::capabilities::types::{CapabilityContext, PermissionLevel};
use crate::modules::context::ActiveContext;

/// Minimal first-pass permission policy.
/// This is intentionally simple:
/// - disabled capabilities are blocked
/// - context compatibility is checked loosely
/// - sensitive/confirm are allowed for now, but the hook exists
///
/// Later this becomes the place for:
/// - remembered user consent
/// - per-capability prompts
/// - app-specific trust rules
pub fn is_allowed(capability_id: &str, context: &ActiveContext) -> bool {
    let Some(capability) = find_capability(capability_id) else {
        return false;
    };

    if capability.permission == PermissionLevel::Disabled {
        return false;
    }

    context_matches(capability.contexts, context)
}

fn context_matches(expected: &[CapabilityContext], context: &ActiveContext) -> bool {
    if expected.iter().any(|c| matches!(c, CapabilityContext::Any)) {
        return true;
    }

    let domain = context.domain.trim().to_lowercase();

    expected.iter().any(|cap_ctx| match cap_ctx {
        CapabilityContext::Any => true,
        CapabilityContext::Desktop => domain == "desktop",
        CapabilityContext::Browser => domain == "browser",
        CapabilityContext::Editor => domain == "editor",
        CapabilityContext::Media => domain == "media",
        CapabilityContext::Game => domain == "game",
        CapabilityContext::Companion => domain == "companion",
    })
}