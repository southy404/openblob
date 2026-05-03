use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::modules::storage::paths;

const PIPER_MACOS_AARCH64_TGZ_URL: &str =
    "https://sourceforge.net/projects/piper-tts.mirror/files/2023.11.14-2/piper_macos_aarch64.tar.gz/download";

const DEFAULT_EN_VOICE: &str = "en_US-lessac-high";
const DEFAULT_EN_VOICE_ONNX_URL: &str =
    "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/high/en_US-lessac-high.onnx";
const DEFAULT_EN_VOICE_JSON_URL: &str =
    "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/high/en_US-lessac-high.onnx.json";

fn download_to(url: &str, out_path: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed with status {}", resp.status()));
    }

    let mut file = fs::File::create(out_path).map_err(|e| format!("Create failed: {e}"))?;

    let mut buf = [0u8; 1024 * 64];
    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| format!("Read failed: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Write failed: {e}"))?;
    }

    file.flush().map_err(|e| format!("Flush failed: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn extract_tgz(tgz_path: &Path, out_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("Create dir failed: {e}"))?;

    let status = Command::new("tar")
        .args(["-xzf"])
        .arg(tgz_path)
        .arg("-C")
        .arg(out_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run tar: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("Extract failed.".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn extract_tgz(_tgz_path: &Path, _out_dir: &Path) -> Result<(), String> {
    Err("Not supported on this OS.".to_string())
}

pub fn piper_dir() -> Result<PathBuf, String> {
    Ok(paths::app_data_dir()?.join("tts").join("piper"))
}

pub fn models_dir() -> Result<PathBuf, String> {
    Ok(paths::app_data_dir()?.join("tts").join("models"))
}

pub fn piper_exe_path() -> Result<PathBuf, String> {
    Ok(piper_dir()?.join("piper"))
}

fn is_piper_installed() -> bool {
    piper_exe_path().ok().map(|p| p.exists()).unwrap_or(false)
}

fn is_default_voice_installed() -> bool {
    let Ok(dir) = models_dir() else { return false; };
    dir.join(format!("{DEFAULT_EN_VOICE}.onnx")).exists()
        && dir.join(format!("{DEFAULT_EN_VOICE}.onnx.json")).exists()
}

pub fn tts_download_default_piper_assets() -> Result<String, String> {
    #[cfg(not(target_os = "macos"))]
    {
        return Err("This download helper is currently only implemented for macOS.".into());
    }

    #[cfg(target_os = "macos")]
    {
        let base = paths::app_data_dir()?;
        let tts_dir = base.join("tts");
        fs::create_dir_all(&tts_dir).map_err(|e| format!("Create dir failed: {e}"))?;

        // 1) Piper binary
        if !is_piper_installed() {
            let tmp_dir = tts_dir.join("tmp");
            fs::create_dir_all(&tmp_dir).map_err(|e| format!("Create tmp dir failed: {e}"))?;
            let tgz_path = tmp_dir.join("piper_macos_aarch64.tar.gz");
            download_to(PIPER_MACOS_AARCH64_TGZ_URL, &tgz_path)?;

            let out_dir = piper_dir()?;
            extract_tgz(&tgz_path, &out_dir)?;

            // Try to find the piper binary somewhere inside the extracted folder.
            if !piper_exe_path()?.exists() {
                let mut found: Option<PathBuf> = None;
                for entry in fs::read_dir(&out_dir).ok().into_iter().flatten() {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let path = entry.path();
                    if path.is_dir() {
                        let candidate = path.join("piper");
                        if candidate.exists() {
                            found = Some(candidate);
                            break;
                        }
                    }
                }

                if let Some(found) = found {
                    fs::copy(&found, piper_exe_path()?)
                        .map_err(|e| format!("Failed to place piper binary: {e}"))?;
                }
            }

            // Ensure executable bit.
            let _ = Command::new("chmod")
                .args(["+x"])
                .arg(piper_exe_path()?)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        // 2) Default voice
        if !is_default_voice_installed() {
            let models = models_dir()?;
            fs::create_dir_all(&models).map_err(|e| format!("Create models dir failed: {e}"))?;
            download_to(
                DEFAULT_EN_VOICE_ONNX_URL,
                &models.join(format!("{DEFAULT_EN_VOICE}.onnx")),
            )?;
            download_to(
                DEFAULT_EN_VOICE_JSON_URL,
                &models.join(format!("{DEFAULT_EN_VOICE}.onnx.json")),
            )?;
        }

        Ok(format!(
            "Installed Piper + voice in {}",
            paths::app_data_dir()?.display()
        ))
    }
}
