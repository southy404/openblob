use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::modules::storage::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptPrereqs {
    pub ok: bool,
    pub default_input_device: Option<String>,
    pub whisper_exe: Option<String>,
    pub whisper_model: Option<String>,
    pub needs_virtual_audio_routing: bool,
    pub message: String,
}

fn which(cmd: &str) -> Option<String> {
    let out = std::process::Command::new("which").arg(cmd).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text
        .lines()
        .find(|l| !l.trim().is_empty())?
        .trim()
        .to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

fn resolve_whisper_exe() -> Option<String> {
    if let Ok(v) = std::env::var("OPENBLOB_WHISPER_EXE") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }

    // Local dev paths.
    #[cfg(target_os = "windows")]
    {
        let candidate = "voice/bin/whisper-cli.exe";
        if Path::new(candidate).exists() {
            return Some(candidate.into());
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        for candidate in ["voice/bin/whisper-cli", "voice/bin/whisper-cpp"] {
            if Path::new(candidate).exists() {
                return Some(candidate.into());
            }
        }
    }

    // System PATH candidates.
    for name in ["whisper-cpp", "whisper-cli", "whisper"] {
        if let Some(p) = which(name) {
            return Some(p);
        }
    }

    None
}

fn resolve_whisper_model() -> Option<String> {
    if let Ok(v) = std::env::var("OPENBLOB_WHISPER_MODEL") {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }

    let candidates = [
        "voice/models/ggml-base.en.bin",
        "voice/models/ggml-base.bin",
        "voice/models/ggm-base.bin",
    ];

    for c in candidates {
        if Path::new(c).exists() {
            return Some(c.into());
        }
    }

    if let Ok(app_data) = paths::app_data_dir() {
        let app_candidates = [
            app_data
                .join("models")
                .join("whisper")
                .join("ggml-base.en.bin"),
            app_data
                .join("models")
                .join("whisper")
                .join("ggml-base.bin"),
        ];
        for c in app_candidates {
            if c.exists() {
                return Some(c.display().to_string());
            }
        }
    }

    None
}

#[cfg(target_os = "macos")]
fn default_input_device_name() -> Option<String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let device = host.default_input_device()?;
    device.name().ok()
}

#[cfg(not(target_os = "macos"))]
fn default_input_device_name() -> Option<String> {
    None
}

pub fn check_prereqs() -> TranscriptPrereqs {
    let input = default_input_device_name();
    let exe = resolve_whisper_exe();
    let model = resolve_whisper_model();

    let needs_virtual = cfg!(target_os = "macos");
    let ok = exe.is_some() && model.is_some();

    let message = if cfg!(target_os = "macos") {
        if !ok {
            "macOS transcript needs: (1) Whisper CLI installed (e.g. `brew install whisper-cpp`) and (2) a ggml model file. For system-audio transcription you must route system output into an input device (e.g. BlackHole) and set it as default input."
                .to_string()
        } else {
            "Whisper CLI + model found. For system-audio transcription on macOS, ensure your default input is the routed system audio device (e.g. BlackHole)."
                .to_string()
        }
    } else if !ok {
        "Transcript needs Whisper CLI + model configured.".to_string()
    } else {
        "Whisper CLI + model found.".to_string()
    };

    TranscriptPrereqs {
        ok,
        default_input_device: input,
        whisper_exe: exe,
        whisper_model: model,
        needs_virtual_audio_routing: needs_virtual,
        message,
    }
}

fn default_model_url() -> &'static str {
    // Hosted on Hugging Face (ggerganov/whisper.cpp). This is the base English model in ggml format.
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin?download=true"
}

pub fn ensure_default_model_downloaded() -> Result<PathBuf, String> {
    let app_data = paths::app_data_dir()?;
    let out_dir = app_data.join("models").join("whisper");
    fs::create_dir_all(&out_dir).map_err(|e| format!("Failed to create model dir: {e}"))?;

    let out_path = out_dir.join("ggml-base.en.bin");
    if out_path.exists() {
        return Ok(out_path);
    }

    let mut resp = reqwest::blocking::get(default_model_url())
        .map_err(|e| format!("Failed to download model: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Model download failed with status {}",
            resp.status()
        ));
    }

    let mut tmp_path = out_path.clone();
    tmp_path.set_extension("bin.part");

    let mut file =
        fs::File::create(&tmp_path).map_err(|e| format!("Failed to create file: {e}"))?;

    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| format!("Failed to read download stream: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Failed to write model file: {e}"))?;
    }

    file.flush()
        .map_err(|e| format!("Failed to flush model file: {e}"))?;

    fs::rename(&tmp_path, &out_path).map_err(|e| format!("Failed to finalize model file: {e}"))?;

    Ok(out_path)
}
