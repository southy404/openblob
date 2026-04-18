use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use strsim::jaro_winkler;
use tauri::{AppHandle, Emitter};

const JARO_WINKLER_THRESHOLD: f64 = 0.88;
const RESTART_DELAY_SECS: u64 = 3;
const COOLDOWN_SECS: u64 = 2;

pub struct WakeWordHandle {
    stop_flag: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl WakeWordHandle {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(child) = guard.as_mut() {
                let _ = child.kill();
            }
            *guard = None;
        }
    }
}

fn whisper_stream_path() -> String {
    "voice/bin/whisper-stream.exe".to_string()
}

fn model_path() -> String {
    "voice/models/ggml-base.bin".to_string()
}

fn phrase_matches(line: &str, phrase: &str) -> bool {
    let haystack = line.trim().to_lowercase();
    let needle = phrase.trim().to_lowercase();

    if haystack.contains(&needle) {
        return true;
    }

    // Slide a window of the same word-length as the needle over the haystack
    // and fuzzy-match each window.
    let needle_words: Vec<&str> = needle.split_whitespace().collect();
    let haystack_words: Vec<&str> = haystack.split_whitespace().collect();
    let wlen = needle_words.len();

    if haystack_words.len() < wlen {
        let similarity = jaro_winkler(&haystack, &needle);
        return similarity >= JARO_WINKLER_THRESHOLD;
    }

    for i in 0..=(haystack_words.len() - wlen) {
        let window = haystack_words[i..i + wlen].join(" ");
        let similarity = jaro_winkler(&window, &needle);
        if similarity >= JARO_WINKLER_THRESHOLD {
            return true;
        }
    }

    false
}

pub fn start_wake_word_listener(app: AppHandle, phrase: String) -> Result<WakeWordHandle, String> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    let child_arc: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    let stop_flag_clone = Arc::clone(&stop_flag);
    let child_arc_clone = Arc::clone(&child_arc);
    let phrase_clone = phrase.clone();

    std::thread::spawn(move || {
        listener_loop(app, phrase_clone, stop_flag_clone, child_arc_clone);
    });

    Ok(WakeWordHandle {
        stop_flag,
        child: child_arc,
    })
}

fn listener_loop(
    app: AppHandle,
    phrase: String,
    stop_flag: Arc<AtomicBool>,
    child_arc: Arc<Mutex<Option<Child>>>,
) {
    while !stop_flag.load(Ordering::SeqCst) {
        match spawn_whisper_stream() {
            Ok(mut child) => {
                let stdout = match child.stdout.take() {
                    Some(s) => s,
                    None => {
                        eprintln!("[wake_word] whisper-stream stdout unavailable, retrying...");
                        std::thread::sleep(Duration::from_secs(RESTART_DELAY_SECS));
                        continue;
                    }
                };

                // Store child so it can be killed on stop()
                {
                    let mut guard = child_arc.lock().unwrap();
                    *guard = Some(child);
                }

                let reader = BufReader::new(stdout);
                // Use Option<Instant> to track last match; None means "never matched"
                let mut last_match: Option<std::time::Instant> = None;

                for line in reader.lines() {
                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }

                    let line = match line {
                        Ok(l) => l,
                        Err(_) => break,
                    };

                    if line.trim().is_empty() {
                        continue;
                    }

                    println!("[wake_word] whisper: {}", line);

                    if last_match
                        .map(|t| t.elapsed() < Duration::from_secs(COOLDOWN_SECS))
                        .unwrap_or(false)
                    {
                        continue;
                    }

                    if phrase_matches(&line, &phrase) {
                        println!("[wake_word] phrase detected: '{}'", phrase);
                        last_match = Some(std::time::Instant::now());
                        let _ = app.emit("wake-word-detected", ());
                    }
                }

                // Process exited or pipe closed
                {
                    let mut guard = child_arc.lock().unwrap();
                    if let Some(child) = guard.as_mut() {
                        let _ = child.wait();
                    }
                    *guard = None;
                }

                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                eprintln!(
                    "[wake_word] whisper-stream exited, restarting in {}s...",
                    RESTART_DELAY_SECS
                );
                std::thread::sleep(Duration::from_secs(RESTART_DELAY_SECS));
            }
            Err(e) => {
                eprintln!("[wake_word] failed to spawn whisper-stream: {e}, retrying...");
                std::thread::sleep(Duration::from_secs(RESTART_DELAY_SECS));
            }
        }
    }

    println!("[wake_word] listener loop exited.");
}

fn spawn_whisper_stream() -> Result<Child, String> {
    Command::new(whisper_stream_path())
        .args([
            "-m",
            &model_path(),
            "--step",
            "2000", // audio step size in ms (slide window by 2 seconds)
            "--length",
            "4000", // audio window length in ms (4-second context)
            "-vth",
            "0.4", // voice activity detection threshold (0–1)
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn error: {e}"))
}
