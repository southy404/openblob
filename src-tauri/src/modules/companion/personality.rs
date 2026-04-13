use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::personality_state_path;

pub const CURRENT_PERSONALITY_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityState {
    pub version: u32,
    pub energy: f32,
    pub curiosity: f32,
    pub affection: f32,
    pub playfulness: f32,
    pub focus_bias: f32,
    pub last_updated: Option<String>,
}

impl Default for PersonalityState {
    fn default() -> Self {
        Self {
            version: CURRENT_PERSONALITY_STATE_VERSION,
            energy: 0.72,
            curiosity: 0.63,
            affection: 0.36,
            playfulness: 0.48,
            focus_bias: 0.62,
            last_updated: None,
        }
    }
}

impl PersonalityState {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_PERSONALITY_STATE_VERSION;
        clamp_unit(&mut self.energy);
        clamp_unit(&mut self.curiosity);
        clamp_unit(&mut self.affection);
        clamp_unit(&mut self.playfulness);
        clamp_unit(&mut self.focus_bias);
        self
    }

    pub fn mood_hint(&self) -> &'static str {
        if self.energy < 0.28 {
            "sleepy"
        } else if self.affection > 0.78 {
            "love"
        } else if self.playfulness > 0.72 {
            "happy"
        } else {
            "idle"
        }
    }
}

pub fn load_personality_state() -> Result<PersonalityState, String> {
    let path = personality_state_path()?;
    let state = load_json_or_default::<PersonalityState>(&path)?.normalized();
    Ok(state)
}

pub fn save_personality_state(state: &PersonalityState) -> Result<(), String> {
    let path = personality_state_path()?;
    save_json(&path, &state.clone().normalized())
}

pub fn load_or_create_personality_state() -> Result<PersonalityState, String> {
    let state = load_personality_state()?;
    save_personality_state(&state)?;
    Ok(state)
}

fn clamp_unit(value: &mut f32) {
    *value = value.clamp(0.0, 1.0);
}