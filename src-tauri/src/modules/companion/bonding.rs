use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::bonding_state_path;

pub const CURRENT_BONDING_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondingState {
    pub version: u32,
    pub relationship_level: f32,
    pub trust_score: f32,
    pub successful_help_count: u32,
    pub shared_sessions: u32,
    pub last_interaction_at: Option<String>,
}

impl Default for BondingState {
    fn default() -> Self {
        Self {
            version: CURRENT_BONDING_STATE_VERSION,
            relationship_level: 0.12,
            trust_score: 0.18,
            successful_help_count: 0,
            shared_sessions: 0,
            last_interaction_at: None,
        }
    }
}

impl BondingState {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_BONDING_STATE_VERSION;
        clamp_unit(&mut self.relationship_level);
        clamp_unit(&mut self.trust_score);
        self
    }

    pub fn register_helpful_interaction(&mut self) {
        self.successful_help_count = self.successful_help_count.saturating_add(1);
        self.relationship_level = (self.relationship_level + 0.015).clamp(0.0, 1.0);
        self.trust_score = (self.trust_score + 0.02).clamp(0.0, 1.0);
    }

    pub fn register_session_start(&mut self) {
        self.shared_sessions = self.shared_sessions.saturating_add(1);
        self.relationship_level = (self.relationship_level + 0.005).clamp(0.0, 1.0);
    }
}

pub fn load_bonding_state() -> Result<BondingState, String> {
    let path = bonding_state_path()?;
    let state = load_json_or_default::<BondingState>(&path)?.normalized();
    Ok(state)
}

pub fn save_bonding_state(state: &BondingState) -> Result<(), String> {
    let path = bonding_state_path()?;
    save_json(&path, &state.clone().normalized())
}

pub fn load_or_create_bonding_state() -> Result<BondingState, String> {
    let state = load_bonding_state()?;
    save_bonding_state(&state)?;
    Ok(state)
}

fn clamp_unit(value: &mut f32) {
    *value = value.clamp(0.0, 1.0);
}