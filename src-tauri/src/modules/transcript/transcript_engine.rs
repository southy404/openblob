use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::audio_capture::AudioChunk;

pub fn write_chunk_wav(
    base_dir: &PathBuf,
    idx: usize,
    chunk: &AudioChunk,
) -> Result<PathBuf, String> {
    fs::create_dir_all(base_dir).map_err(|e| e.to_string())?;

    let path = base_dir.join(format!("chunk_{idx:04}.wav"));

    let spec = hound::WavSpec {
        channels: chunk.channels,
        sample_rate: chunk.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(&path, spec).map_err(|e| e.to_string())?;

    for sample in &chunk.samples_i16 {
        writer.write_sample(*sample).map_err(|e| e.to_string())?;
    }

    writer.finalize().map_err(|e| e.to_string())?;

    Ok(path)
}

pub fn transcribe_with_whisper_cli(
    whisper_exe: &str,
    model_path: &str,
    wav_path: &str,
    language: &str,
) -> Result<String, String> {
    ensure_file_exists(whisper_exe, "whisper executable")?;
    ensure_file_exists(model_path, "whisper model")?;
    ensure_file_exists(wav_path, "wav input")?;

    let wav = Path::new(wav_path);
    let parent_dir = wav
        .parent()
        .ok_or_else(|| "Failed to resolve wav parent directory".to_string())?;

    let stem = wav
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "Failed to resolve wav file stem".to_string())?;

    let output = Command::new(whisper_exe)
        .args([
            "-m",
            model_path,
            "-f",
            wav_path,
            "-l",
            language,
            "-otxt",
            "-nt",
            "-of",
            stem,
        ])
        .current_dir(parent_dir)
        .output()
        .map_err(|e| format!("Failed to run whisper executable: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let details = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };

        return Err(format!("Whisper failed: {}", details));
    }

    let txt_path = parent_dir.join(format!("{stem}.txt"));

    if !txt_path.exists() {
        return Err(format!(
            "Whisper finished without creating transcript file: {}. stdout: {} stderr: {}",
            txt_path.display(),
            stdout.trim(),
            stderr.trim()
        ));
    }

    let raw_text =
        fs::read_to_string(&txt_path).map_err(|e| format!("Failed to read transcript file: {}", e))?;

    let _ = fs::remove_file(&txt_path);

    Ok(normalize_whisper_text(&raw_text))
}

fn ensure_file_exists(path: &str, label: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(format!("Missing {} at path: {}", label, path))
    }
}

fn normalize_whisper_text(input: &str) -> String {
    let mut text = input.replace("\r\n", "\n").replace('\r', "\n");
    text = text.trim().to_string();

    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let mut merged = lines.join(" ");

    while merged.contains("  ") {
        merged = merged.replace("  ", " ");
    }

    merged = merged
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" :", ":")
        .replace(" ;", ";");

    merged.trim().to_string()
}