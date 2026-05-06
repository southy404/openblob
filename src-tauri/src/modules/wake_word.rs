use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::Duration as StdDuration;

use crate::modules::profile::companion_config::{
    default_wake_word_phrase, default_wake_word_provider, default_wake_word_sensitivity,
    load_or_create_companion_config, normalize_wake_word_provider, save_companion_config,
};

const LOG_PREFIX: &str = "[openblob:wake-word]";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordSettings {
    #[serde(default)]
    pub wake_word_enabled: bool,
    #[serde(default = "default_wake_word_phrase")]
    pub wake_word_phrase: String,
    #[serde(default = "default_wake_word_sensitivity")]
    pub wake_word_sensitivity: f32,
    #[serde(default = "default_wake_word_provider")]
    pub wake_word_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordStatus {
    pub status: String,
    pub state: String,
    pub message: String,
    pub enabled: bool,
    pub phrase: String,
    pub provider: String,
    pub sensitivity: f32,
    pub listening: bool,
    pub detected: bool,
    pub provider_configured: bool,
    pub selected_input_device: Option<String>,
    pub available_input_devices: Vec<String>,
    pub last_error: Option<String>,
    pub last_started_at: Option<String>,
    pub last_stopped_at: Option<String>,
    pub last_audio_at: Option<String>,
    pub audio_chunks_seen: u64,
    pub input_level: Option<f32>,
}

struct WakeWordRuntime {
    stop_tx: mpsc::Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
    selected_input_device: String,
}

static WAKE_WORD_STATUS: OnceLock<Mutex<WakeWordStatus>> = OnceLock::new();
static WAKE_WORD_RUNTIME: OnceLock<Mutex<Option<WakeWordRuntime>>> = OnceLock::new();

fn status_store() -> &'static Mutex<WakeWordStatus> {
    WAKE_WORD_STATUS
        .get_or_init(|| Mutex::new(status_for_state("disabled", "Wake word is disabled.", None)))
}

fn runtime_store() -> &'static Mutex<Option<WakeWordRuntime>> {
    WAKE_WORD_RUNTIME.get_or_init(|| Mutex::new(None))
}

fn log(message: impl AsRef<str>) {
    println!("{LOG_PREFIX} {}", message.as_ref());
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn normalize_settings(mut settings: WakeWordSettings) -> WakeWordSettings {
    settings.wake_word_phrase = if settings.wake_word_phrase.trim().is_empty() {
        default_wake_word_phrase()
    } else {
        settings.wake_word_phrase.trim().to_string()
    };
    settings.wake_word_sensitivity = settings.wake_word_sensitivity.clamp(0.0, 1.0);
    settings.wake_word_provider = normalize_wake_word_provider(&settings.wake_word_provider);
    settings
}

fn settings_from_config() -> Result<WakeWordSettings, String> {
    let config = load_or_create_companion_config()?;
    Ok(normalize_settings(WakeWordSettings {
        wake_word_enabled: config.wake_word_enabled,
        wake_word_phrase: config.wake_word_phrase,
        wake_word_sensitivity: config.wake_word_sensitivity,
        wake_word_provider: config.wake_word_provider,
    }))
}

fn input_device_names() -> Vec<String> {
    let host = cpal::default_host();
    match host.input_devices() {
        Ok(devices) => devices.filter_map(|device| device.name().ok()).collect(),
        Err(err) => {
            log(format!("could not enumerate input devices: {err}"));
            Vec::new()
        }
    }
}

fn base_status(settings: &WakeWordSettings) -> WakeWordStatus {
    WakeWordStatus {
        status: "stopped".into(),
        state: "stopped".into(),
        message: "Wake word listener is stopped.".into(),
        enabled: settings.wake_word_enabled,
        phrase: settings.wake_word_phrase.clone(),
        provider: settings.wake_word_provider.clone(),
        sensitivity: settings.wake_word_sensitivity,
        listening: false,
        detected: false,
        provider_configured: is_provider_configured(&settings.wake_word_provider),
        selected_input_device: None,
        available_input_devices: input_device_names(),
        last_error: None,
        last_started_at: None,
        last_stopped_at: None,
        last_audio_at: None,
        audio_chunks_seen: 0,
        input_level: None,
    }
}

fn status_for_state(
    state: &str,
    message: impl Into<String>,
    last_error: Option<String>,
) -> WakeWordStatus {
    let settings = settings_from_config().unwrap_or_else(|_| {
        normalize_settings(WakeWordSettings {
            wake_word_enabled: false,
            wake_word_phrase: default_wake_word_phrase(),
            wake_word_sensitivity: default_wake_word_sensitivity(),
            wake_word_provider: default_wake_word_provider(),
        })
    });
    status_for_state_with_settings(&settings, state, message, last_error)
}

fn status_for_state_with_settings(
    settings: &WakeWordSettings,
    state: &str,
    message: impl Into<String>,
    last_error: Option<String>,
) -> WakeWordStatus {
    let mut status = base_status(settings);
    status.status = state.into();
    status.state = state.into();
    status.message = message.into();
    status.listening = state == "listening" || state == "starting";
    status.detected = state == "detected";
    status.last_error = last_error;
    status
}

fn set_status(status: WakeWordStatus) -> Result<WakeWordStatus, String> {
    let mut guard = status_store()
        .lock()
        .map_err(|_| "Wake word status lock is poisoned".to_string())?;
    *guard = status.clone();
    Ok(status)
}

fn current_status() -> Result<WakeWordStatus, String> {
    status_store()
        .lock()
        .map_err(|_| "Wake word status lock is poisoned".to_string())
        .map(|guard| guard.clone())
}

fn is_provider_configured(provider: &str) -> bool {
    matches!(provider, "mic-test" | "mock")
}

fn provider_missing_status(settings: &WakeWordSettings) -> WakeWordStatus {
    let message = match settings.wake_word_provider.as_str() {
        "none" | "disabled" => "Wake word provider not configured yet.",
        "porcupine" => {
            if porcupine_configured() {
                "Porcupine wake word model is not wired yet. Use mic-test for local microphone testing."
            } else {
                "Wake word provider not configured yet."
            }
        }
        _ => "Wake word provider not configured yet.",
    };
    status_for_state_with_settings(settings, "provider_missing", message, Some(message.into()))
}

fn porcupine_configured() -> bool {
    let has_access_key = std::env::var("PICOVOICE_ACCESS_KEY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_keyword_path = std::env::var("OPENBLOB_PORCUPINE_KEYWORD_PATH")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    has_access_key && has_keyword_path
}

fn stop_runtime() -> Result<Option<String>, String> {
    let mut guard = runtime_store()
        .lock()
        .map_err(|_| "Wake word runtime lock is poisoned".to_string())?;
    let Some(mut runtime) = guard.take() else {
        return Ok(None);
    };

    let selected_input_device = runtime.selected_input_device.clone();
    let _ = runtime.stop_tx.send(());

    if let Some(join_handle) = runtime.join_handle.take() {
        if join_handle.join().is_err() {
            log("mic listener thread exited with an error");
        }
    }

    Ok(Some(selected_input_device))
}

fn classify_audio_error(message: String) -> &'static str {
    let lower = message.to_lowercase();
    if lower.contains("permission")
        || lower.contains("denied")
        || lower.contains("access")
        || lower.contains("unauthorized")
    {
        "permission_error"
    } else {
        "error"
    }
}

fn record_audio_chunk_f32(samples: &[f32]) {
    if samples.is_empty() {
        return;
    }

    let mut sum = 0.0_f32;
    for sample in samples {
        let value = sample.clamp(-1.0, 1.0);
        sum += value * value;
    }

    record_audio_metrics((sum / samples.len() as f32).sqrt().clamp(0.0, 1.0));
}

fn record_audio_chunk_i16(samples: &[i16]) {
    if samples.is_empty() {
        return;
    }

    let mut sum = 0.0_f32;
    for sample in samples {
        let value = (*sample as f32 / i16::MAX as f32).clamp(-1.0, 1.0);
        sum += value * value;
    }

    record_audio_metrics((sum / samples.len() as f32).sqrt().clamp(0.0, 1.0));
}

fn record_audio_chunk_u16(samples: &[u16]) {
    if samples.is_empty() {
        return;
    }

    let mut sum = 0.0_f32;
    for sample in samples {
        let value = ((*sample as f32 - 32768.0) / 32768.0).clamp(-1.0, 1.0);
        sum += value * value;
    }

    record_audio_metrics((sum / samples.len() as f32).sqrt().clamp(0.0, 1.0));
}

fn record_audio_metrics(level: f32) {
    let Ok(mut guard) = status_store().lock() else {
        return;
    };

    if guard.state != "listening" {
        return;
    }

    guard.audio_chunks_seen = guard.audio_chunks_seen.saturating_add(1);
    guard.last_audio_at = Some(now_iso());
    guard.input_level = Some(level);

    if std::env::var_os("RUST_BACKTRACE").is_some() && guard.audio_chunks_seen % 100 == 0 {
        log(format!(
            "mic-test chunks={}, level={level:.3}",
            guard.audio_chunks_seen
        ));
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
) -> Result<cpal::Stream, String> {
    let error_callback = |err| {
        let message = format!("Microphone stream error: {err}");
        eprintln!("{LOG_PREFIX} {message}");

        if let Ok(mut guard) = status_store().lock() {
            guard.status = "error".into();
            guard.state = "error".into();
            guard.message = message.clone();
            guard.last_error = Some(message);
            guard.listening = false;
        }
    };

    match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| record_audio_chunk_f32(data),
                error_callback,
                None,
            )
            .map_err(|err| err.to_string()),
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| record_audio_chunk_i16(data),
                error_callback,
                None,
            )
            .map_err(|err| err.to_string()),
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                config,
                move |data: &[u16], _| record_audio_chunk_u16(data),
                error_callback,
                None,
            )
            .map_err(|err| err.to_string()),
        other => Err(format!("Unsupported microphone sample format: {other:?}")),
    }
}

fn start_microphone_runtime(settings: &WakeWordSettings) -> Result<WakeWordStatus, String> {
    if current_status()
        .map(|status| status.state == "listening")
        .unwrap_or(false)
    {
        return current_status();
    }

    let _ = stop_runtime();

    let mut starting = status_for_state_with_settings(
        settings,
        "starting",
        "Starting local microphone listener.",
        None,
    );
    starting.last_started_at = Some(now_iso());
    let _ = set_status(starting);

    let (stop_tx, stop_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();
    let settings_for_thread = settings.clone();

    let join_handle = thread::Builder::new()
        .name("openblob-wake-word-mic".into())
        .spawn(move || run_microphone_thread(settings_for_thread, stop_rx, ready_tx))
        .map_err(|err| format!("Could not start wake word microphone thread: {err}"))?;

    let status = match ready_rx.recv_timeout(StdDuration::from_secs(3)) {
        Ok(status) => status,
        Err(err) => {
            let message = format!("Microphone listener did not become ready: {err}");
            let status =
                status_for_state_with_settings(settings, "error", message.clone(), Some(message));
            let _ = set_status(status.clone());
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            return Ok(status);
        }
    };

    if status.state != "listening" {
        let _ = stop_tx.send(());
        let _ = join_handle.join();
        return Ok(status);
    }

    {
        let mut guard = runtime_store()
            .lock()
            .map_err(|_| "Wake word runtime lock is poisoned".to_string())?;
        *guard = Some(WakeWordRuntime {
            stop_tx,
            join_handle: Some(join_handle),
            selected_input_device: status
                .selected_input_device
                .clone()
                .unwrap_or_else(|| "Default microphone".into()),
        });
    }

    Ok(status)
}

fn run_microphone_thread(
    settings: WakeWordSettings,
    stop_rx: mpsc::Receiver<()>,
    ready_tx: mpsc::Sender<WakeWordStatus>,
) {
    let host = cpal::default_host();
    let Some(device) = host.default_input_device() else {
        let status = status_for_state_with_settings(
            &settings,
            "no_input_device",
            "No microphone input device is available.",
            Some("No microphone input device is available.".into()),
        );
        let _ = set_status(status.clone());
        let _ = ready_tx.send(status);
        return;
    };

    let device_name = device
        .name()
        .unwrap_or_else(|_| "Default microphone".into());
    let supported_config = match device.default_input_config() {
        Ok(config) => config,
        Err(err) => {
            let message = format!("Could not read microphone input config: {err}");
            let state = classify_audio_error(message.clone());
            let status =
                status_for_state_with_settings(&settings, state, message.clone(), Some(message));
            let _ = set_status(status.clone());
            let _ = ready_tx.send(status);
            return;
        }
    };

    let sample_format = supported_config.sample_format();
    let stream_config = supported_config.config();
    let stream = match build_stream(&device, &stream_config, sample_format) {
        Ok(stream) => stream,
        Err(err) => {
            let state = classify_audio_error(err.clone());
            let status = status_for_state_with_settings(&settings, state, err.clone(), Some(err));
            let _ = set_status(status.clone());
            let _ = ready_tx.send(status);
            return;
        }
    };

    if let Err(err) = stream.play() {
        let message = format!("Could not start microphone stream: {err}");
        let state = classify_audio_error(message.clone());
        let status =
            status_for_state_with_settings(&settings, state, message.clone(), Some(message));
        let _ = set_status(status.clone());
        let _ = ready_tx.send(status);
        return;
    }

    let mut status = status_for_state_with_settings(
        &settings,
        "listening",
        "Local microphone test listener is active. No wake-word model is running yet.",
        None,
    );
    status.provider_configured = true;
    status.selected_input_device = Some(device_name.clone());
    status.last_started_at = Some(now_iso());
    let _ = set_status(status.clone());
    let _ = ready_tx.send(status);

    log(format!(
        "mic listener started; provider={}, device={device_name}",
        settings.wake_word_provider
    ));

    let _stream = stream;
    let _ = stop_rx.recv();
    log("mic listener thread stopping");
}

#[tauri::command]
pub fn get_wake_word_settings() -> Result<WakeWordSettings, String> {
    settings_from_config()
}

#[tauri::command]
pub fn update_wake_word_settings(settings: WakeWordSettings) -> Result<WakeWordSettings, String> {
    let settings = normalize_settings(settings);
    let mut config = load_or_create_companion_config()?;

    let provider_changed = config.wake_word_provider != settings.wake_word_provider;
    let disabled = !settings.wake_word_enabled;

    config.wake_word_enabled = settings.wake_word_enabled;
    config.wake_word_phrase = settings.wake_word_phrase.clone();
    config.wake_word_sensitivity = settings.wake_word_sensitivity;
    config.wake_word_provider = settings.wake_word_provider.clone();
    save_companion_config(&config)?;

    if provider_changed || disabled {
        let _ = stop_runtime();
    }

    log(format!(
        "settings updated; enabled={}, provider={}, sensitivity={:.2}",
        settings.wake_word_enabled, settings.wake_word_provider, settings.wake_word_sensitivity
    ));

    let state = if disabled { "disabled" } else { "stopped" };
    let message = if disabled {
        "Wake word is disabled."
    } else {
        "Wake word listener is stopped."
    };
    let _ = set_status(status_for_state_with_settings(
        &settings, state, message, None,
    ));
    Ok(settings)
}

#[tauri::command]
pub fn start_wake_word_listener() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;

    if !settings.wake_word_enabled {
        log("start skipped; wake word is disabled");
        return set_status(status_for_state_with_settings(
            &settings,
            "disabled",
            "Wake word is disabled.",
            None,
        ));
    }

    if !matches!(settings.wake_word_provider.as_str(), "mic-test" | "mock") {
        let status = provider_missing_status(&settings);
        log(format!(
            "start skipped; provider={} is not available for local mic-test",
            settings.wake_word_provider
        ));
        return set_status(status);
    }

    start_microphone_runtime(&settings)
}

#[tauri::command]
pub fn stop_wake_word_listener() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;
    let selected_input_device = stop_runtime()?;

    let mut status = if settings.wake_word_enabled {
        status_for_state_with_settings(&settings, "stopped", "Wake word listener stopped.", None)
    } else {
        status_for_state_with_settings(&settings, "disabled", "Wake word is disabled.", None)
    };
    status.selected_input_device = selected_input_device;
    status.last_stopped_at = Some(now_iso());

    log("listener stopped");
    set_status(status)
}

#[tauri::command]
pub fn get_wake_word_status() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;
    let mut current = current_status()?;

    current.enabled = settings.wake_word_enabled;
    current.phrase = settings.wake_word_phrase.clone();
    current.provider = settings.wake_word_provider.clone();
    current.sensitivity = settings.wake_word_sensitivity;
    current.provider_configured = is_provider_configured(&settings.wake_word_provider);
    current.available_input_devices = input_device_names();

    if !settings.wake_word_enabled {
        current.status = "disabled".into();
        current.state = "disabled".into();
        current.message = "Wake word is disabled.".into();
        current.listening = false;
        current.detected = false;
        return Ok(current);
    }

    if current.state == "listening" || current.state == "starting" {
        return Ok(current);
    }

    if !matches!(settings.wake_word_provider.as_str(), "mic-test" | "mock") {
        return Ok(provider_missing_status(&settings));
    }

    Ok(current)
}
