use serde::{Deserialize, Serialize};

use crate::modules::storage::json_store::{load_json_or_default, save_json};
use crate::modules::storage::paths::onboarding_state_path;

pub const CURRENT_ONBOARDING_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    pub version: u32,
    pub completed: bool,
    pub current_step: String,
    pub profile_step_done: bool,
    pub voice_step_done: bool,
    pub appearance_step_done: bool,
    pub boundaries_step_done: bool,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            version: CURRENT_ONBOARDING_STATE_VERSION,
            completed: false,
            current_step: "profile".into(),
            profile_step_done: false,
            voice_step_done: false,
            appearance_step_done: false,
            boundaries_step_done: false,
        }
    }
}

impl OnboardingState {
    pub fn normalized(mut self) -> Self {
        self.version = CURRENT_ONBOARDING_STATE_VERSION;

        if self.completed {
            self.profile_step_done = true;
            self.voice_step_done = true;
            self.appearance_step_done = true;
            self.boundaries_step_done = true;
            self.current_step = "done".into();
            return self;
        }

        self.current_step = match self.current_step.trim() {
            "profile" | "voice" | "appearance" | "boundaries" | "done" => {
                self.current_step.clone()
            }
            _ => infer_current_step(&self),
        };

        self
    }

    pub fn mark_step_done(&mut self, step: &str) {
        match step {
            "profile" => self.profile_step_done = true,
            "voice" => self.voice_step_done = true,
            "appearance" => self.appearance_step_done = true,
            "boundaries" => self.boundaries_step_done = true,
            _ => {}
        }

        self.current_step = infer_current_step(self);

        if self.current_step == "done" {
            self.completed = true;
        }
    }
}

pub fn load_onboarding_state() -> Result<OnboardingState, String> {
    let path = onboarding_state_path()?;
    let state = load_json_or_default::<OnboardingState>(&path)?.normalized();
    Ok(state)
}

pub fn save_onboarding_state(state: &OnboardingState) -> Result<(), String> {
    let path = onboarding_state_path()?;
    save_json(&path, &state.clone().normalized())
}

pub fn load_or_create_onboarding_state() -> Result<OnboardingState, String> {
    let state = load_onboarding_state()?;
    save_onboarding_state(&state)?;
    Ok(state)
}

fn infer_current_step(state: &OnboardingState) -> String {
    if !state.profile_step_done {
        "profile".into()
    } else if !state.voice_step_done {
        "voice".into()
    } else if !state.appearance_step_done {
        "appearance".into()
    } else if !state.boundaries_step_done {
        "boundaries".into()
    } else {
        "done".into()
    }
}