use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "OpenBlob";

fn fallback_base_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let home = home.trim();
            if !home.is_empty() {
                return PathBuf::from(home)
                    .join("Library")
                    .join("Application Support");
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            let xdg = xdg.trim();
            if !xdg.is_empty() {
                return PathBuf::from(xdg);
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let home = home.trim();
            if !home.is_empty() {
                return PathBuf::from(home).join(".local").join("share");
            }
        }
    }

    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata);
    }

    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(user_profile)
            .join("AppData")
            .join("Roaming");
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn app_data_dir() -> Result<PathBuf, String> {
    let dir = fallback_base_dir().join(APP_DIR_NAME);
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn config_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("config");
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn memory_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("memory");
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn cache_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("cache");
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn media_dir() -> Result<PathBuf, String> {
    let dir = app_data_dir()?.join("media");
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn snips_dir() -> Result<PathBuf, String> {
    let dir = media_dir()?.join("snips");
    ensure_dir(&dir)?;
    Ok(dir)
}

pub fn companion_config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("companion_config.json"))
}

pub fn onboarding_state_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("onboarding_state.json"))
}

pub fn user_profile_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("user_profile.json"))
}

pub fn personality_state_path() -> Result<PathBuf, String> {
    Ok(memory_dir()?.join("personality_state.json"))
}

pub fn bonding_state_path() -> Result<PathBuf, String> {
    Ok(memory_dir()?.join("bonding_state.json"))
}

pub fn episodic_memory_path() -> Result<PathBuf, String> {
    Ok(memory_dir()?.join("episodic_memory.jsonl"))
}

pub fn semantic_memory_path() -> Result<PathBuf, String> {
    Ok(memory_dir()?.join("semantic_memory.json"))
}

fn ensure_dir(path: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|e| format!("Could not create directory '{}': {e}", path.display()))
}
