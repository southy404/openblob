use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[tauri::command]
pub fn record_and_transcribe_voice(seconds: Option<u64>) -> Result<String, String> {
    let duration_secs = seconds.unwrap_or(5).clamp(1, 15);

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("Kein Standard-Mikrofon gefunden.")?;

    let config = device
        .default_input_config()
        .map_err(|e| format!("Input config Fehler: {e}"))?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();

    let temp_dir = std::env::temp_dir();
    let wav_path: PathBuf = temp_dir.join("companion_voice_input.wav");
    let out_base: PathBuf = temp_dir.join("companion_voice_output");
    let out_txt_path: PathBuf = temp_dir.join("companion_voice_output.txt");

    let spec = WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let writer = WavWriter::create(&wav_path, spec)
        .map_err(|e| format!("WAV create Fehler: {e}"))?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let writer = Arc::clone(&writer);
            device
                .build_input_stream(
                    &config.clone().into(),
                    move |data: &[f32], _| {
                        if let Ok(mut guard) = writer.lock() {
                            if let Some(writer) = guard.as_mut() {
                                for &sample in data {
                                    let s = (sample * i16::MAX as f32) as i16;
                                    let _ = writer.write_sample(s);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Stream build Fehler: {e}"))?
        }
        cpal::SampleFormat::I16 => {
            let writer = Arc::clone(&writer);
            device
                .build_input_stream(
                    &config.clone().into(),
                    move |data: &[i16], _| {
                        if let Ok(mut guard) = writer.lock() {
                            if let Some(writer) = guard.as_mut() {
                                for &sample in data {
                                    let _ = writer.write_sample(sample);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Stream build Fehler: {e}"))?
        }
        cpal::SampleFormat::U16 => {
            let writer = Arc::clone(&writer);
            device
                .build_input_stream(
                    &config.clone().into(),
                    move |data: &[u16], _| {
                        if let Ok(mut guard) = writer.lock() {
                            if let Some(writer) = guard.as_mut() {
                                for &sample in data {
                                    let s = (sample as i32 - 32768) as i16;
                                    let _ = writer.write_sample(s);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Stream build Fehler: {e}"))?
        }
        other => {
            return Err(format!("Nicht unterstütztes Sample-Format: {other:?}"));
        }
    };

    stream
        .play()
        .map_err(|e| format!("Stream play Fehler: {e}"))?;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(duration_secs) {
        std::thread::sleep(Duration::from_millis(50));
    }

    drop(stream);

    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer
                .finalize()
                .map_err(|e| format!("WAV finalize Fehler: {e}"))?;
        }
    }

    let status = Command::new("voice/bin/whisper-cli.exe")
        .args([
            "-m",
            "voice/models/ggml-base.bin",
            "-f",
            wav_path.to_string_lossy().as_ref(),
            "-otxt",
            "-of",
            out_base.to_string_lossy().as_ref(),
            "-l",
            "de",
        ])
        .status()
        .map_err(|e| format!("Whisper Start Fehler: {e}"))?;

    if !status.success() {
        return Err("Whisper CLI fehlgeschlagen.".into());
    }

    let text = std::fs::read_to_string(&out_txt_path)
        .map_err(|e| format!("Transkript lesen Fehler: {e}"))?;

    Ok(text.trim().to_string())
}