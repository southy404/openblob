use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration as StdDuration, Instant};
use tauri::Emitter;

use crate::modules::profile::companion_config::{
    default_wake_word_phrase, default_wake_word_provider, default_wake_word_sensitivity,
    load_or_create_companion_config, normalize_wake_word_provider, save_companion_config,
};
use crate::modules::storage::paths::app_data_dir;

const LOG_PREFIX: &str = "[openblob:wake-word]";
const FRAME_QUEUE_CAPACITY: usize = 8;
const MOCK_DETECTION_COOLDOWN_MS: u64 = 2_000;
const DETECTED_STATE_MS: u64 = 1_500;
const WAKE_WORD_MODEL_EXTENSIONS: [&str; 4] = ["onnx", "tflite", "bin", "json"];

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
    #[serde(default)]
    pub wake_word_model_path: Option<String>,
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
    pub provider_state: String,
    pub model_path: Option<String>,
    pub model_missing: bool,
    pub detection_count: u64,
    pub last_detected_at: Option<String>,
    pub last_detection_score: Option<f32>,
    pub wake_phrase_matched: Option<String>,
    pub provider_ready: bool,
    pub provider_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordModelStatus {
    pub configured_model_path: Option<String>,
    pub resolved_model_path: Option<String>,
    pub model_exists: bool,
    pub model_missing: bool,
    pub discovered_models: Vec<String>,
    pub search_paths: Vec<String>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WakeWordDetectedEvent {
    phrase: String,
    provider: String,
    score: f32,
    detected_at: String,
}

#[derive(Debug, Clone)]
struct WakeWordProviderAvailability {
    provider_state: String,
    provider_ready: bool,
    provider_configured: bool,
    model_path: Option<String>,
    model_missing: bool,
    message: String,
    last_error: Option<String>,
    start_allowed: bool,
    start_state: String,
}

enum WakeWordProviderResult {
    NoMatch,
    Detected { score: f32, phrase: String },
    ModelMissing(String),
    ProviderMissing(String),
    Error(String),
}

trait WakeWordProvider: Send {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> WakeWordProviderAvailability;
    fn process_audio_frame(&mut self, frame: &[f32], sample_rate: u32) -> WakeWordProviderResult;
}

struct WakeWordRuntime {
    stop_tx: mpsc::Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
    selected_input_device: String,
}

struct MicTestProvider;

struct MockWakeWordProvider {
    phrase: String,
    threshold: f32,
    last_detection: Option<Instant>,
}

struct LocalWakeWordProvider {
    provider_name: &'static str,
    model_status: WakeWordModelStatus,
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
    settings.wake_word_model_path = settings.wake_word_model_path.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    settings
}

fn settings_from_config() -> Result<WakeWordSettings, String> {
    let config = load_or_create_companion_config()?;
    Ok(normalize_settings(WakeWordSettings {
        wake_word_enabled: config.wake_word_enabled,
        wake_word_phrase: config.wake_word_phrase,
        wake_word_sensitivity: config.wake_word_sensitivity,
        wake_word_provider: config.wake_word_provider,
        wake_word_model_path: config.wake_word_model_path,
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
    let availability = provider_availability(settings);

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
        provider_configured: availability.provider_configured,
        selected_input_device: None,
        available_input_devices: input_device_names(),
        last_error: None,
        last_started_at: None,
        last_stopped_at: None,
        last_audio_at: None,
        audio_chunks_seen: 0,
        input_level: None,
        provider_state: availability.provider_state,
        model_path: availability.model_path,
        model_missing: availability.model_missing,
        detection_count: 0,
        last_detected_at: None,
        last_detection_score: None,
        wake_phrase_matched: None,
        provider_ready: availability.provider_ready,
        provider_error: availability.last_error,
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
            wake_word_model_path: None,
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
    status.listening = state == "listening" || state == "starting" || state == "detected";
    status.detected = state == "detected";
    status.last_error = last_error;
    status
}

fn apply_settings_to_status(status: &mut WakeWordStatus, settings: &WakeWordSettings) {
    let availability = provider_availability(settings);

    status.enabled = settings.wake_word_enabled;
    status.phrase = settings.wake_word_phrase.clone();
    status.provider = settings.wake_word_provider.clone();
    status.sensitivity = settings.wake_word_sensitivity;
    status.provider_configured = availability.provider_configured;
    status.available_input_devices = input_device_names();
    status.provider_state = availability.provider_state;
    status.model_path = availability.model_path;
    status.model_missing = availability.model_missing;
    status.provider_ready = availability.provider_ready;
    status.provider_error = availability.last_error;
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

fn is_local_provider(provider: &str) -> bool {
    matches!(provider, "local-openwakeword" | "local-wakeword")
}

fn wake_word_model_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(app_dir) = app_data_dir() {
        paths.push(app_dir.join("voice").join("models").join("wake-word"));
    }

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("voice").join("models").join("wake-word"));

        if let Some(parent) = cwd.parent() {
            paths.push(parent.join("voice").join("models").join("wake-word"));
        }
    }

    let mut seen = Vec::new();
    paths
        .into_iter()
        .filter(|path| {
            let display = path.display().to_string();
            if seen.iter().any(|item| item == &display) {
                false
            } else {
                seen.push(display);
                true
            }
        })
        .collect()
}

fn is_supported_model_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| {
                WAKE_WORD_MODEL_EXTENSIONS
                    .iter()
                    .any(|ext| value.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
}

fn discover_wake_word_model_paths() -> Vec<PathBuf> {
    let mut models = Vec::new();

    for search_path in wake_word_model_search_paths() {
        let Ok(entries) = fs::read_dir(&search_path) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if is_supported_model_file(&path) {
                models.push(path);
            }
        }
    }

    models.sort();
    models.dedup();
    models
}

fn path_string(path: &Path) -> String {
    path.display().to_string()
}

fn resolve_configured_model_path(raw_path: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() || candidate.exists() {
        return Some(candidate);
    }

    for search_path in search_paths {
        let joined = search_path.join(&candidate);
        if joined.exists() {
            return Some(joined);
        }
    }

    Some(candidate)
}

fn model_status_for_settings(settings: &WakeWordSettings) -> WakeWordModelStatus {
    let search_paths = wake_word_model_search_paths();
    let discovered_paths = discover_wake_word_model_paths();

    let configured_path = settings.wake_word_model_path.as_deref();
    let resolved_path = configured_path
        .and_then(|raw| resolve_configured_model_path(raw, &search_paths))
        .or_else(|| discovered_paths.first().cloned());

    let model_exists = resolved_path
        .as_deref()
        .map(is_supported_model_file)
        .unwrap_or(false);
    let model_missing = is_local_provider(&settings.wake_word_provider) && !model_exists;

    WakeWordModelStatus {
        configured_model_path: settings.wake_word_model_path.clone(),
        resolved_model_path: resolved_path.as_deref().map(path_string),
        model_exists,
        model_missing,
        discovered_models: discovered_paths
            .iter()
            .map(|path| path_string(path))
            .collect(),
        search_paths: search_paths.iter().map(|path| path_string(path)).collect(),
        provider: settings.wake_word_provider.clone(),
    }
}

fn provider_availability(settings: &WakeWordSettings) -> WakeWordProviderAvailability {
    make_provider(settings).is_available()
}

fn make_provider(settings: &WakeWordSettings) -> Box<dyn WakeWordProvider> {
    match settings.wake_word_provider.as_str() {
        "mic-test" => Box::new(MicTestProvider),
        "mock" => Box::new(MockWakeWordProvider::new(settings)),
        "local-openwakeword" => Box::new(LocalWakeWordProvider::new(
            "local-openwakeword",
            model_status_for_settings(settings),
        )),
        "local-wakeword" => Box::new(LocalWakeWordProvider::new(
            "local-wakeword",
            model_status_for_settings(settings),
        )),
        _ => Box::new(LocalWakeWordProvider::disabled(
            settings.wake_word_provider.clone(),
        )),
    }
}

impl WakeWordProvider for MicTestProvider {
    fn name(&self) -> &'static str {
        "mic-test"
    }

    fn is_available(&self) -> WakeWordProviderAvailability {
        WakeWordProviderAvailability {
            provider_state: "mic_test_active".into(),
            provider_ready: true,
            provider_configured: true,
            model_path: None,
            model_missing: false,
            message: "Mic test is active. No wake-word model is running.".into(),
            last_error: None,
            start_allowed: true,
            start_state: "listening".into(),
        }
    }

    fn process_audio_frame(&mut self, _frame: &[f32], _sample_rate: u32) -> WakeWordProviderResult {
        WakeWordProviderResult::NoMatch
    }
}

impl MockWakeWordProvider {
    fn new(settings: &WakeWordSettings) -> Self {
        let sensitivity = settings.wake_word_sensitivity.clamp(0.0, 1.0);
        let threshold = (0.82 - sensitivity * 0.22).clamp(0.60, 0.82);

        Self {
            phrase: settings.wake_word_phrase.clone(),
            threshold,
            last_detection: None,
        }
    }
}

impl WakeWordProvider for MockWakeWordProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn is_available(&self) -> WakeWordProviderAvailability {
        WakeWordProviderAvailability {
            provider_state: "mock_active".into(),
            provider_ready: true,
            provider_configured: true,
            model_path: None,
            model_missing: false,
            message: "Mock wake-word detection is active for development only.".into(),
            last_error: None,
            start_allowed: true,
            start_state: "listening".into(),
        }
    }

    fn process_audio_frame(&mut self, frame: &[f32], _sample_rate: u32) -> WakeWordProviderResult {
        if frame.is_empty() {
            return WakeWordProviderResult::NoMatch;
        }

        let level = rms(frame);
        if !level.is_finite() {
            return WakeWordProviderResult::Error(
                "Wake-word provider received invalid audio input level.".into(),
            );
        }

        let now = Instant::now();
        let cooldown_ready = self
            .last_detection
            .map(|last| {
                now.duration_since(last) >= StdDuration::from_millis(MOCK_DETECTION_COOLDOWN_MS)
            })
            .unwrap_or(true);

        if level >= self.threshold && cooldown_ready {
            self.last_detection = Some(now);
            return WakeWordProviderResult::Detected {
                score: level,
                phrase: self.phrase.clone(),
            };
        }

        WakeWordProviderResult::NoMatch
    }
}

impl LocalWakeWordProvider {
    fn new(provider_name: &'static str, model_status: WakeWordModelStatus) -> Self {
        Self {
            provider_name,
            model_status,
        }
    }

    fn disabled(provider: String) -> Self {
        Self {
            provider_name: if provider == "disabled" {
                "disabled"
            } else {
                "none"
            },
            model_status: WakeWordModelStatus {
                configured_model_path: None,
                resolved_model_path: None,
                model_exists: false,
                model_missing: false,
                discovered_models: Vec::new(),
                search_paths: wake_word_model_search_paths()
                    .iter()
                    .map(|path| path_string(path))
                    .collect(),
                provider,
            },
        }
    }
}

impl WakeWordProvider for LocalWakeWordProvider {
    fn name(&self) -> &'static str {
        self.provider_name
    }

    fn is_available(&self) -> WakeWordProviderAvailability {
        if self.provider_name == "none" || self.provider_name == "disabled" {
            let message = "Wake word provider not configured yet.".to_string();
            return WakeWordProviderAvailability {
                provider_state: "provider_missing".into(),
                provider_ready: false,
                provider_configured: false,
                model_path: None,
                model_missing: false,
                message: message.clone(),
                last_error: Some(message),
                start_allowed: false,
                start_state: "provider_missing".into(),
            };
        }

        if self.model_status.model_missing {
            let message =
                "Local wake-word provider selected, but no model is installed.".to_string();
            return WakeWordProviderAvailability {
                provider_state: "model_missing".into(),
                provider_ready: false,
                provider_configured: false,
                model_path: self.model_status.resolved_model_path.clone(),
                model_missing: true,
                message: message.clone(),
                last_error: Some(message),
                start_allowed: false,
                start_state: "model_missing".into(),
            };
        }

        let message = "Local wake-word model found, but runtime inference is not implemented yet."
            .to_string();
        WakeWordProviderAvailability {
            provider_state: "model_found_runtime_not_implemented".into(),
            provider_ready: false,
            provider_configured: false,
            model_path: self.model_status.resolved_model_path.clone(),
            model_missing: false,
            message: message.clone(),
            last_error: Some(message),
            start_allowed: false,
            start_state: "provider_not_implemented".into(),
        }
    }

    fn process_audio_frame(&mut self, _frame: &[f32], _sample_rate: u32) -> WakeWordProviderResult {
        // TODO: Plug in a free local openWakeWord-compatible inference backend here.
        if self.model_status.model_missing {
            WakeWordProviderResult::ModelMissing(
                "Local wake-word provider selected, but no model is installed.".into(),
            )
        } else {
            WakeWordProviderResult::ProviderMissing(
                "Local wake-word model found, but runtime inference is not implemented yet.".into(),
            )
        }
    }
}

fn unavailable_status(settings: &WakeWordSettings) -> WakeWordStatus {
    let availability = provider_availability(settings);
    status_for_state_with_settings(
        settings,
        &availability.start_state,
        availability.message.clone(),
        availability.last_error.clone(),
    )
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

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum = samples
        .iter()
        .map(|sample| {
            let value = sample.clamp(-1.0, 1.0);
            value * value
        })
        .sum::<f32>();

    (sum / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

fn record_audio_chunk_f32(samples: &[f32], frame_tx: &mpsc::SyncSender<Vec<f32>>) {
    if samples.is_empty() {
        return;
    }

    let frame = samples
        .iter()
        .map(|sample| sample.clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    let level = rms(&frame);
    record_audio_metrics(level);
    let _ = frame_tx.try_send(frame);
}

fn record_audio_chunk_i16(samples: &[i16], frame_tx: &mpsc::SyncSender<Vec<f32>>) {
    if samples.is_empty() {
        return;
    }

    let frame = samples
        .iter()
        .map(|sample| (*sample as f32 / i16::MAX as f32).clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    let level = rms(&frame);
    record_audio_metrics(level);
    let _ = frame_tx.try_send(frame);
}

fn record_audio_chunk_u16(samples: &[u16], frame_tx: &mpsc::SyncSender<Vec<f32>>) {
    if samples.is_empty() {
        return;
    }

    let frame = samples
        .iter()
        .map(|sample| ((*sample as f32 - 32768.0) / 32768.0).clamp(-1.0, 1.0))
        .collect::<Vec<_>>();
    let level = rms(&frame);
    record_audio_metrics(level);
    let _ = frame_tx.try_send(frame);
}

fn record_audio_metrics(level: f32) {
    let Ok(mut guard) = status_store().lock() else {
        return;
    };

    if guard.state != "listening" && guard.state != "detected" {
        return;
    }

    guard.audio_chunks_seen = guard.audio_chunks_seen.saturating_add(1);
    guard.last_audio_at = Some(now_iso());
    guard.input_level = Some(level);

    if std::env::var_os("RUST_BACKTRACE").is_some() && guard.audio_chunks_seen % 100 == 0 {
        log(format!(
            "mic chunks={}, provider={}, level={level:.3}",
            guard.audio_chunks_seen, guard.provider
        ));
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    frame_tx: mpsc::SyncSender<Vec<f32>>,
) -> Result<cpal::Stream, String> {
    let error_callback = |err| {
        let message = format!("Microphone stream error: {err}");
        eprintln!("{LOG_PREFIX} {message}");

        if let Ok(mut guard) = status_store().lock() {
            guard.status = "error".into();
            guard.state = "error".into();
            guard.message = message.clone();
            guard.last_error = Some(message.clone());
            guard.provider_error = Some(message);
            guard.listening = false;
        }
    };

    match sample_format {
        cpal::SampleFormat::F32 => {
            let frame_tx = frame_tx.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[f32], _| record_audio_chunk_f32(data, &frame_tx),
                    error_callback,
                    None,
                )
                .map_err(|err| err.to_string())
        }
        cpal::SampleFormat::I16 => {
            let frame_tx = frame_tx.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[i16], _| record_audio_chunk_i16(data, &frame_tx),
                    error_callback,
                    None,
                )
                .map_err(|err| err.to_string())
        }
        cpal::SampleFormat::U16 => {
            let frame_tx = frame_tx.clone();
            device
                .build_input_stream(
                    config,
                    move |data: &[u16], _| record_audio_chunk_u16(data, &frame_tx),
                    error_callback,
                    None,
                )
                .map_err(|err| err.to_string())
        }
        other => Err(format!("Unsupported microphone sample format: {other:?}")),
    }
}

fn start_microphone_runtime(
    app: tauri::AppHandle,
    settings: &WakeWordSettings,
) -> Result<WakeWordStatus, String> {
    if current_status()
        .map(|status| status.listening || status.state == "starting")
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
        .spawn(move || run_microphone_thread(app, settings_for_thread, stop_rx, ready_tx))
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
    app: tauri::AppHandle,
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
    let sample_rate = stream_config.sample_rate.0;
    let (frame_tx, frame_rx) = mpsc::sync_channel::<Vec<f32>>(FRAME_QUEUE_CAPACITY);
    let stream = match build_stream(&device, &stream_config, sample_format, frame_tx.clone()) {
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

    let detection_settings = settings.clone();
    let provider_thread = thread::Builder::new()
        .name("openblob-wake-word-provider".into())
        .spawn(move || run_provider_worker(app, detection_settings, sample_rate, frame_rx));

    let mut status = status_for_state_with_settings(
        &settings,
        "listening",
        listening_message(&settings.wake_word_provider),
        None,
    );
    status.provider_configured = true;
    status.provider_ready = true;
    status.selected_input_device = Some(device_name.clone());
    status.last_started_at = Some(now_iso());
    let _ = set_status(status.clone());
    let _ = ready_tx.send(status);

    log(format!(
        "mic listener started; provider={}, device={device_name}, sample_rate={sample_rate}",
        settings.wake_word_provider
    ));

    let stream = stream;
    let _ = stop_rx.recv();
    drop(stream);
    drop(frame_tx);

    if let Ok(handle) = provider_thread {
        if handle.join().is_err() {
            log("provider worker thread exited with an error");
        }
    }

    log("mic listener thread stopping");
}

fn listening_message(provider: &str) -> &'static str {
    match provider {
        "mic-test" => "Mic test is active. No wake-word model is running.",
        "mock" => "Mock wake-word detection is active for development only.",
        _ => "Local wake word listener is active.",
    }
}

fn run_provider_worker(
    app: tauri::AppHandle,
    settings: WakeWordSettings,
    sample_rate: u32,
    frame_rx: mpsc::Receiver<Vec<f32>>,
) {
    let mut provider = make_provider(&settings);
    let provider_name = provider.name().to_string();

    log(format!(
        "provider worker started; provider={provider_name}, sample_rate={sample_rate}"
    ));

    for frame in frame_rx {
        match provider.process_audio_frame(&frame, sample_rate) {
            WakeWordProviderResult::NoMatch => {}
            WakeWordProviderResult::Detected { score, phrase } => {
                handle_wake_word_detected(&app, &settings, &provider_name, phrase, score);
            }
            WakeWordProviderResult::ModelMissing(message) => {
                set_runtime_error_state(&settings, "model_missing", message);
                break;
            }
            WakeWordProviderResult::ProviderMissing(message) => {
                set_runtime_error_state(&settings, "provider_missing", message);
                break;
            }
            WakeWordProviderResult::Error(message) => {
                set_runtime_error_state(&settings, "error", message);
                break;
            }
        }
    }

    log(format!("provider worker stopped; provider={provider_name}"));
}

fn set_runtime_error_state(settings: &WakeWordSettings, state: &str, message: String) {
    let mut status =
        status_for_state_with_settings(settings, state, message.clone(), Some(message));
    status.listening = false;
    let _ = set_status(status);
}

fn handle_wake_word_detected(
    app: &tauri::AppHandle,
    settings: &WakeWordSettings,
    provider: &str,
    phrase: String,
    score: f32,
) {
    let detected_at = now_iso();
    let event = WakeWordDetectedEvent {
        phrase: phrase.clone(),
        provider: provider.to_string(),
        score,
        detected_at: detected_at.clone(),
    };

    {
        let Ok(mut guard) = status_store().lock() else {
            return;
        };

        guard.status = "detected".into();
        guard.state = "detected".into();
        guard.message = "Wake word detected.".into();
        guard.listening = true;
        guard.detected = true;
        guard.provider_ready = true;
        guard.provider_error = None;
        guard.provider_state = "detected".into();
        guard.detection_count = guard.detection_count.saturating_add(1);
        guard.last_detected_at = Some(detected_at.clone());
        guard.last_detection_score = Some(score);
        guard.wake_phrase_matched = Some(phrase.clone());
    }

    if let Err(err) = app.emit("wake-word-detected", event) {
        log(format!("could not emit wake-word-detected event: {err}"));
    }

    // TODO: Wire this event to the existing voice-capture flow once there is a safe shared helper.
    log(format!(
        "wake word detected; provider={provider}, phrase={}, score={score:.3}",
        settings.wake_word_phrase
    ));
    schedule_detected_reset();
}

fn schedule_detected_reset() {
    let _ = thread::Builder::new()
        .name("openblob-wake-word-detected-reset".into())
        .spawn(|| {
            thread::sleep(StdDuration::from_millis(DETECTED_STATE_MS));

            let Ok(mut guard) = status_store().lock() else {
                return;
            };

            if guard.state == "detected" {
                guard.status = "listening".into();
                guard.state = "listening".into();
                guard.message = listening_message(&guard.provider).into();
                guard.detected = false;
                guard.listening = true;
                guard.provider_state = if guard.provider == "mock" {
                    "mock_active".into()
                } else {
                    "mic_test_active".into()
                };
            }
        });
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
    config.wake_word_model_path = settings.wake_word_model_path.clone();
    save_companion_config(&config)?;

    if provider_changed || disabled {
        let _ = stop_runtime();
    }

    log(format!(
        "settings updated; enabled={}, provider={}, sensitivity={:.2}",
        settings.wake_word_enabled, settings.wake_word_provider, settings.wake_word_sensitivity
    ));

    let availability = provider_availability(&settings);
    let (state, message) = if disabled {
        ("disabled", "Wake word is disabled.".to_string())
    } else if availability.start_allowed {
        ("stopped", "Wake word listener is stopped.".to_string())
    } else {
        (
            availability.start_state.as_str(),
            availability.message.clone(),
        )
    };
    let _ = set_status(status_for_state_with_settings(
        &settings,
        state,
        message,
        availability.last_error,
    ));
    Ok(settings)
}

#[tauri::command]
pub fn start_wake_word_listener(app: tauri::AppHandle) -> Result<WakeWordStatus, String> {
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

    let availability = provider_availability(&settings);
    if !availability.start_allowed {
        let status = unavailable_status(&settings);
        log(format!(
            "start skipped; provider={}, provider_state={}",
            settings.wake_word_provider, status.provider_state
        ));
        return set_status(status);
    }

    start_microphone_runtime(app, &settings)
}

#[tauri::command]
pub fn stop_wake_word_listener() -> Result<WakeWordStatus, String> {
    let settings = settings_from_config()?;
    let selected_input_device = stop_runtime()?;

    let mut status = if settings.wake_word_enabled {
        let availability = provider_availability(&settings);
        if availability.start_allowed {
            status_for_state_with_settings(
                &settings,
                "stopped",
                "Wake word listener stopped.",
                None,
            )
        } else {
            unavailable_status(&settings)
        }
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
    apply_settings_to_status(&mut current, &settings);

    if !settings.wake_word_enabled {
        current.status = "disabled".into();
        current.state = "disabled".into();
        current.message = "Wake word is disabled.".into();
        current.listening = false;
        current.detected = false;
        return Ok(current);
    }

    if current.state == "listening" || current.state == "starting" || current.state == "detected" {
        return Ok(current);
    }

    let availability = provider_availability(&settings);
    if !availability.start_allowed {
        current.status = availability.start_state.clone();
        current.state = availability.start_state;
        current.message = availability.message;
        current.last_error = availability.last_error;
        current.listening = false;
        current.detected = false;
        return Ok(current);
    }

    Ok(current)
}

#[tauri::command]
pub fn get_wake_word_model_status() -> Result<WakeWordModelStatus, String> {
    let settings = settings_from_config()?;
    Ok(model_status_for_settings(&settings))
}

#[tauri::command]
pub fn list_wake_word_models() -> Result<Vec<String>, String> {
    Ok(discover_wake_word_model_paths()
        .iter()
        .map(|path| path_string(path))
        .collect())
}

#[tauri::command]
pub fn set_wake_word_model_path(path: Option<String>) -> Result<WakeWordModelStatus, String> {
    let mut config = load_or_create_companion_config()?;
    config.wake_word_model_path = path.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    save_companion_config(&config)?;

    let settings = settings_from_config()?;
    let status = model_status_for_settings(&settings);

    if let Ok(mut current) = status_store().lock() {
        current.model_path = status.resolved_model_path.clone();
        current.model_missing = status.model_missing;
        current.provider_state = provider_availability(&settings).provider_state;
    }

    Ok(status)
}
