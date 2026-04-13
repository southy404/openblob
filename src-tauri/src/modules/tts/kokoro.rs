use reqwest::Client;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::NamedTempFile;

#[derive(Debug, Serialize)]
struct KokoroRequest<'a> {
    text: &'a str,
    voice: &'a str,
}

fn play_wav_windows(path: &Path) -> Result<(), String> {
    let wav = path
        .to_str()
        .ok_or_else(|| "Ungültiger WAV-Pfad.".to_string())?
        .replace('\'', "''");

    let script = format!(
        r#"
$player = New-Object System.Media.SoundPlayer '{}'
$player.Load()
$player.PlaySync()
"#,
        wav
    );

    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|e| format!("Konnte Audio nicht abspielen: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("PowerShell-Audiowiedergabe fehlgeschlagen.".to_string())
    }
}

fn save_temp_wav(bytes: &[u8]) -> Result<NamedTempFile, String> {
    let mut wav_file = NamedTempFile::new()
        .map_err(|e| format!("Temporäre WAV-Datei konnte nicht erstellt werden: {e}"))?;

    std::io::Write::write_all(&mut wav_file, bytes)
        .map_err(|e| format!("Kokoro-Audio konnte nicht gespeichert werden: {e}"))?;

    Ok(wav_file)
}

pub async fn speak(text: &str, base_url: &str, voice: &str) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let trimmed_base_url = base_url.trim().trim_end_matches('/');
    if trimmed_base_url.is_empty() {
        return Err("Kokoro-Base-URL ist leer.".to_string());
    }

    let trimmed_voice = voice.trim();
    if trimmed_voice.is_empty() {
        return Err("Kokoro-Voice ist leer.".to_string());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Kokoro-Client konnte nicht erstellt werden: {e}"))?;

    let url = format!("{trimmed_base_url}/tts");

    let response = client
        .post(&url)
        .json(&KokoroRequest {
            text: trimmed,
            voice: trimmed_voice,
        })
        .send()
        .await
        .map_err(|e| format!("Kokoro-Request fehlgeschlagen: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Kokoro HTTP Fehler {}: {}", status, body));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Kokoro-Audio konnte nicht gelesen werden: {e}"))?;

    if bytes.is_empty() {
        return Err("Kokoro hat leere Audiodaten zurückgegeben.".to_string());
    }

    let wav_file = save_temp_wav(&bytes)?;
    let wav_path: PathBuf = wav_file.path().to_path_buf();

    play_wav_windows(&wav_path)?;

    Ok(())
}