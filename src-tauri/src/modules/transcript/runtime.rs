use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError};
use tauri::{AppHandle, Emitter};

use super::{
    audio_capture::{start_system_loopback_capture, AudioChunk, CaptureHandle},
    session,
    transcript_engine::{transcribe_with_whisper_cli, write_chunk_wav},
    types::{TranscriptSegment, TranscriptSession, TranscriptSourceKind},
};

const CHUNK_TARGET_MS: u64 = 8_000;
const CHUNK_MAX_MS: u64 = 12_000;
const RECV_TIMEOUT_MS: u64 = 250;

pub struct TranscriptRuntimeHandle {
    stop_flag: Arc<AtomicBool>,
    capture_handle: CaptureHandle,
}

impl TranscriptRuntimeHandle {
    pub fn stop(self) -> Result<(), String> {
        println!("[transcript] stopping runtime...");
        self.stop_flag.store(true, Ordering::SeqCst);
        self.capture_handle.stop();
        Ok(())
    }
}

pub fn start_runtime(
    app: AppHandle,
    session: TranscriptSession,
) -> Result<TranscriptRuntimeHandle, String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_worker = stop_flag.clone();

    let (tx, rx) = unbounded::<AudioChunk>();

    println!("[transcript] starting runtime for session {}", session.id);

    let capture_handle = match session.source {
        TranscriptSourceKind::SystemAudio => {
            println!("[transcript] using SYSTEM AUDIO capture");
            start_system_loopback_capture(tx)?
        }
        _ => {
            return Err("Only system audio supported right now".into());
        }
    };

    let app_for_worker = app.clone();
    let session_id = session.id.clone();

    thread::spawn(move || {
        println!("[transcript] worker thread spawned for {}", session_id);

        if let Err(err) = run_transcript_worker(
            app_for_worker,
            session_id,
            rx,
            stop_flag_worker,
        ) {
            eprintln!("[transcript] worker crashed: {}", err);
        }
    });

    Ok(TranscriptRuntimeHandle {
        stop_flag,
        capture_handle,
    })
}

fn run_transcript_worker(
    app: AppHandle,
    session_id: String,
    rx: Receiver<AudioChunk>,
    stop_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let base_dir = PathBuf::from("D:\\openblob-data")
        .join("transcripts")
        .join(&session_id)
        .join("audio");

    println!("[transcript] audio dir = {}", base_dir.display());

    let whisper_exe = r"D:\openblob\voice\bin\whisper-cli.exe";
    let whisper_model = r"D:\openblob\voice\models\ggml-base.en.bin";
    let language = "en";

    let mut file_idx = 0usize;
    let mut pending_chunks: Vec<AudioChunk> = Vec::new();

    loop {
        let should_break = stop_flag.load(Ordering::SeqCst);

        match rx.recv_timeout(Duration::from_millis(RECV_TIMEOUT_MS)) {
            Ok(chunk) => {
                println!(
                    "[transcript] received chunk {}-{} ms",
                    chunk.start_ms, chunk.end_ms
                );
                pending_chunks.push(chunk);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                println!("[transcript] audio channel disconnected");
                break;
            }
        }

        let pending_ms = pending_duration_ms(&pending_chunks);

        let should_flush =
            pending_ms >= CHUNK_TARGET_MS || pending_ms >= CHUNK_MAX_MS || (should_break && !pending_chunks.is_empty());

        if should_flush {
            flush_pending_chunks(
                &app,
                &base_dir,
                whisper_exe,
                whisper_model,
                language,
                &mut file_idx,
                &mut pending_chunks,
            )?;
        }

        if should_break {
            break;
        }
    }

    if !pending_chunks.is_empty() {
        flush_pending_chunks(
            &app,
            &base_dir,
            whisper_exe,
            whisper_model,
            language,
            &mut file_idx,
            &mut pending_chunks,
        )?;
    }

    let _ = fs::remove_dir_all(&base_dir);

    println!("[transcript] worker stopped");
    Ok(())
}

fn flush_pending_chunks(
    app: &AppHandle,
    base_dir: &PathBuf,
    whisper_exe: &str,
    whisper_model: &str,
    language: &str,
    file_idx: &mut usize,
    pending_chunks: &mut Vec<AudioChunk>,
) -> Result<(), String> {
    if pending_chunks.is_empty() {
        return Ok(());
    }

    let merged = merge_audio_chunks(pending_chunks)
        .ok_or_else(|| "Failed to merge pending transcript chunks".to_string())?;

    pending_chunks.clear();

    let wav_path = match write_chunk_wav(base_dir, *file_idx, &merged) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("[transcript] wav error: {}", err);
            let _ = app.emit("transcript://error", err.clone());
            *file_idx += 1;
            return Ok(());
        }
    };

    println!(
        "[transcript] wrote merged wav {} ({}-{} ms)",
        wav_path.display(),
        merged.start_ms,
        merged.end_ms
    );

    let result = transcribe_with_whisper_cli(
        whisper_exe,
        whisper_model,
        wav_path.to_string_lossy().as_ref(),
        language,
    );

    let _ = fs::remove_file(&wav_path);

    match result {
        Ok(text) => {
            let clean = normalize_asr_text(&text);

            if clean.is_empty() {
                println!("[transcript] empty result");
            } else {
                println!("[transcript] result: {}", clean);

                let segment = TranscriptSegment {
                    start_ms: merged.start_ms,
                    end_ms: merged.end_ms,
                    speaker: None,
                    text: clean,
                    confidence: None,
                };

                match session::append_segment(segment.clone()) {
                    Ok(_) => {
                        let _ = app.emit("transcript://segment", &segment);
                    }
                    Err(err) => {
                        let _ = app.emit("transcript://error", err);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("[transcript] whisper error: {}", err);
            let _ = app.emit("transcript://error", err);
        }
    }

    *file_idx += 1;
    Ok(())
}

fn pending_duration_ms(chunks: &[AudioChunk]) -> u64 {
    match (chunks.first(), chunks.last()) {
        (Some(first), Some(last)) if last.end_ms >= first.start_ms => last.end_ms - first.start_ms,
        _ => 0,
    }
}

fn merge_audio_chunks(chunks: &[AudioChunk]) -> Option<AudioChunk> {
    let first = chunks.first()?;
    let last = chunks.last()?;

    let sample_rate = first.sample_rate;
    let channels = first.channels;

    let compatible = chunks.iter().all(|chunk| {
        chunk.sample_rate == sample_rate && chunk.channels == channels
    });

    if !compatible {
        return None;
    }

    let total_samples: usize = chunks.iter().map(|chunk| chunk.samples_i16.len()).sum();
    let mut samples = Vec::with_capacity(total_samples);

    for chunk in chunks {
        samples.extend_from_slice(&chunk.samples_i16);
    }

    Some(AudioChunk {
        sample_rate,
        channels,
        samples_i16: samples,
        start_ms: first.start_ms,
        end_ms: last.end_ms,
    })
}

fn normalize_asr_text(input: &str) -> String {
    let mut text = input.trim().replace('\n', " ");

    while text.contains("  ") {
        text = text.replace("  ", " ");
    }

    text = text
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(" !", "!")
        .replace(" ?", "?")
        .replace(" :", ":")
        .replace(" ;", ";");

    text.trim().to_string()
}