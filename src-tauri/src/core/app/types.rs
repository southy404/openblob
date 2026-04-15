use crate::core::capabilities::types::CapabilityRequest;
use crate::core::executor::result::CapabilityResult;
use crate::modules::command_router::CompanionAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRoute {
    NoMatch,
    NewCore,
    LegacyFallback,
    NewCoreThenFallback,
}

#[derive(Debug)]
pub struct CommandPipelineResult {
    pub route: CommandRoute,
    pub action: Option<CompanionAction>,
    pub capability: Option<CapabilityRequest>,
    pub result: Option<CapabilityResult>,
}