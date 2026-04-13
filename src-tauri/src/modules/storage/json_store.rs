use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn load_json<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Could not read JSON file '{}': {e}", path.display()))?;

    serde_json::from_str::<T>(&raw)
        .map_err(|e| format!("Could not parse JSON file '{}': {e}", path.display()))
}

pub fn load_json_or_default<T>(path: &Path) -> Result<T, String>
where
    T: DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }

    load_json(path)
}

pub fn save_json<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not create parent directory '{}': {e}",
                parent.display()
            )
        })?;
    }

    let raw = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Could not serialize JSON for '{}': {e}", path.display()))?;

    fs::write(path, raw)
        .map_err(|e| format!("Could not write JSON file '{}': {e}", path.display()))
}

pub fn append_jsonl<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not create parent directory '{}': {e}",
                parent.display()
            )
        })?;
    }

    let line = serde_json::to_string(value)
        .map_err(|e| format!("Could not serialize JSONL for '{}': {e}", path.display()))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("Could not open JSONL file '{}': {e}", path.display()))?;

    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|e| format!("Could not append JSONL file '{}': {e}", path.display()))
}