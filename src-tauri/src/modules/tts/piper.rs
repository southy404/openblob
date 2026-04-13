use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

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

fn resolve_model_path(models_dir: &str, voice: &str) -> Result<PathBuf, String> {
    let trimmed_models_dir = models_dir.trim();
    if trimmed_models_dir.is_empty() {
        return Err("Piper models_dir ist leer.".to_string());
    }

    let trimmed_voice = voice.trim();
    if trimmed_voice.is_empty() {
        return Err("Piper voice ist leer.".to_string());
    }

    let model_path = resolve_path(trimmed_models_dir).join(format!("{trimmed_voice}.onnx"));

    if !model_path.exists() {
        return Err(format!(
            "Piper-Modell nicht gefunden: {}",
            model_path.display()
        ));
    }

    Ok(model_path)
}

fn resolve_path(path: &str) -> PathBuf {
    let p = PathBuf::from(path);

    if p.is_absolute() {
        return p;
    }

    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(p)
}

fn resolve_config_path(model_path: &Path) -> PathBuf {
    let model_str = model_path.to_string_lossy().to_string();
    PathBuf::from(format!("{model_str}.json"))
}

pub fn speak(
    text: &str,
    piper_exe: &str,
    models_dir: &str,
    voice: &str,
) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let trimmed_piper_exe = piper_exe.trim();
    if trimmed_piper_exe.is_empty() {
        return Err("Piper executable ist leer.".to_string());
    }

    let piper_path = resolve_path(trimmed_piper_exe);
    if !piper_path.exists() {
        return Err(format!(
            "Piper executable nicht gefunden: {}",
            piper_path.display()
        ));
    }

    let model_path = resolve_model_path(models_dir, voice)?;
    let config_path = resolve_config_path(&model_path);

    if !config_path.exists() {
        return Err(format!(
            "Piper-Konfig nicht gefunden: {}",
            config_path.display()
        ));
    }

    let wav_file = NamedTempFile::new()
        .map_err(|e| format!("Temporäre WAV-Datei konnte nicht erstellt werden: {e}"))?;
    let wav_path = wav_file.path().to_path_buf();

    let mut child = Command::new(&piper_path)
        .arg("--model")
        .arg(&model_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--output_file")
        .arg(&wav_path)
        .arg("--sentence_silence")
        .arg("0.15")
        .arg("--length_scale")
        .arg("1.0")
        .arg("--noise_scale")
        .arg("0.667")
        .arg("--noise_w")
        .arg("0.8")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Piper konnte nicht gestartet werden: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(trimmed.as_bytes())
            .map_err(|e| format!("Text konnte nicht an Piper gesendet werden: {e}"))?;
        stdin
            .write_all(b"\n")
            .map_err(|e| format!("Zeilenende konnte nicht an Piper gesendet werden: {e}"))?;
    } else {
        return Err("Piper stdin konnte nicht geöffnet werden.".to_string());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Piper-Ausgabe konnte nicht gelesen werden: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let details = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "Unbekannter Fehler".to_string()
        };

        return Err(format!("Piper fehlgeschlagen: {details}"));
    }

    if !wav_path.exists() {
        return Err("Piper hat keine WAV-Datei erzeugt.".to_string());
    }

    play_wav_windows(&wav_path)
}