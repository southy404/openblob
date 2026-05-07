use chrono::Utc;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ort::session::Session;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
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
const DEFAULT_WAKE_WORD_SAMPLE_RATE: u32 = 16_000;
const DEFAULT_WAKE_WORD_FRAME_MS: u32 = 80;
const DEFAULT_WAKE_WORD_THRESHOLD: f32 = 0.5;
const ONNX_RUNTIME_PATH_ENV: &str = "OPENBLOB_ONNX_RUNTIME_PATH";

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
    #[serde(default)]
    pub wake_word_auto_listen_enabled: bool,
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
    pub runtime_state: String,
    pub manifest_path: Option<String>,
    pub manifest_valid: bool,
    pub missing_files: Vec<String>,
    pub sample_rate: Option<u32>,
    pub threshold: Option<f32>,
    pub detection_count: u64,
    pub last_detected_at: Option<String>,
    pub last_detection_score: Option<f32>,
    pub wake_phrase_matched: Option<String>,
    pub provider_ready: bool,
    pub provider_error: Option<String>,
    pub loaded_model_count: u32,
    pub classifier_input_shape: Option<String>,
    pub classifier_output_shape: Option<String>,
    pub runtime_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordModelStatus {
    pub configured_model_path: Option<String>,
    pub resolved_model_path: Option<String>,
    pub model_exists: bool,
    pub model_missing: bool,
    pub manifest_path: Option<String>,
    pub manifest_valid: bool,
    pub missing_files: Vec<String>,
    pub runtime: Option<String>,
    pub phrase: Option<String>,
    pub sample_rate: Option<u32>,
    pub threshold: Option<f32>,
    pub discovered_models: Vec<String>,
    pub search_paths: Vec<String>,
    pub provider: String,
    pub runtime_state: String,
    pub loaded_model_count: u32,
    pub classifier_input_shape: Option<String>,
    pub classifier_output_shape: Option<String>,
    pub runtime_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WakeWordBundleManifest {
    id: Option<String>,
    provider: Option<String>,
    phrase: Option<String>,
    runtime: Option<String>,
    #[serde(rename = "sampleRate")]
    sample_rate: Option<u32>,
    #[serde(rename = "frameMs")]
    frame_ms: Option<u32>,
    threshold: Option<f32>,
    models: WakeWordBundleModels,
}

#[derive(Debug, Clone, Deserialize)]
struct WakeWordBundleModels {
    melspectrogram: Option<String>,
    embedding: Option<String>,
    classifier: Option<String>,
}

#[derive(Debug, Clone)]
struct WakeWordModelBundle {
    bundle_dir: PathBuf,
    manifest_path: PathBuf,
    manifest: WakeWordBundleManifest,
    missing_files: Vec<String>,
    validation_errors: Vec<String>,
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
    runtime_state: String,
    provider_ready: bool,
    provider_configured: bool,
    model_path: Option<String>,
    model_missing: bool,
    manifest_path: Option<String>,
    manifest_valid: bool,
    missing_files: Vec<String>,
    sample_rate: Option<u32>,
    threshold: Option<f32>,
    message: String,
    last_error: Option<String>,
    start_allowed: bool,
    start_state: String,
    loaded_model_count: u32,
    classifier_input_shape: Option<String>,
    classifier_output_shape: Option<String>,
    runtime_error: Option<String>,
}

enum WakeWordProviderResult {
    NoMatch,
    Detected { score: f32, phrase: String },
    ModelMissing(String),
    ProviderMissing(String),
    RuntimeMissing(String),
    InvalidModelBundle(String),
    PipelineIncomplete(String),
    Error(String),
}

trait WakeWordProvider: Send {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> WakeWordProviderAvailability;
    fn target_sample_rate(&self) -> u32 {
        DEFAULT_WAKE_WORD_SAMPLE_RATE
    }
    fn frame_ms(&self) -> u32 {
        DEFAULT_WAKE_WORD_FRAME_MS
    }
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
    runtime: Option<OnnxWakeWordRuntime>,
    runtime_error: Option<String>,
}

struct OnnxSession {
    session: Session,
    input_shape: Option<String>,
    output_shape: Option<String>,
}

struct OnnxWakeWordRuntime {
    manifest: WakeWordBundleManifest,
    melspectrogram: Option<OnnxSession>,
    embedding: Option<OnnxSession>,
    classifier: OnnxSession,
}

#[derive(Debug, Clone)]
struct WakeWordInferenceResult {
    score: f32,
    phrase: String,
}

#[derive(Debug, Clone)]
enum WakeWordRuntimeError {
    RuntimeMissing(String),
    RuntimeLoadFailed(String),
    UnsupportedRuntime(String),
    InvalidBundle(String),
    PipelineIncomplete(String),
}

impl OnnxWakeWordRuntime {
    fn load(bundle: &WakeWordModelBundle) -> Result<Self, WakeWordRuntimeError> {
        load_onnx_wake_word_runtime(bundle)
    }

    #[allow(dead_code)]
    fn run_inference_frame(
        &mut self,
        frame: &[f32],
        sample_rate: u32,
    ) -> Result<WakeWordInferenceResult, WakeWordRuntimeError> {
        run_wake_word_inference(self, frame, sample_rate)
    }

    fn loaded_model_count(&self) -> u32 {
        1 + if self.melspectrogram.is_some() { 1 } else { 0 }
            + if self.embedding.is_some() { 1 } else { 0 }
    }

    fn classifier_input_shape(&self) -> Option<String> {
        self.classifier.input_shape.clone()
    }

    fn classifier_output_shape(&self) -> Option<String> {
        self.classifier.output_shape.clone()
    }
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
        wake_word_auto_listen_enabled: config.wake_word_auto_listen_enabled,
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
        runtime_state: availability.runtime_state,
        manifest_path: availability.manifest_path,
        manifest_valid: availability.manifest_valid,
        missing_files: availability.missing_files,
        sample_rate: availability.sample_rate,
        threshold: availability.threshold,
        detection_count: 0,
        last_detected_at: None,
        last_detection_score: None,
        wake_phrase_matched: None,
        provider_ready: availability.provider_ready,
        provider_error: availability.last_error,
        loaded_model_count: availability.loaded_model_count,
        classifier_input_shape: availability.classifier_input_shape,
        classifier_output_shape: availability.classifier_output_shape,
        runtime_error: availability.runtime_error,
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
            wake_word_auto_listen_enabled: false,
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
    status.runtime_state = availability.runtime_state;
    status.manifest_path = availability.manifest_path;
    status.manifest_valid = availability.manifest_valid;
    status.missing_files = availability.missing_files;
    status.sample_rate = availability.sample_rate;
    status.threshold = availability.threshold;
    status.provider_ready = availability.provider_ready;
    status.provider_error = availability.last_error;
    status.loaded_model_count = availability.loaded_model_count;
    status.classifier_input_shape = availability.classifier_input_shape;
    status.classifier_output_shape = availability.classifier_output_shape;
    status.runtime_error = availability.runtime_error;
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

fn manifest_path_for_bundle(path: &Path) -> Option<PathBuf> {
    if path.is_dir() {
        let manifest_path = path.join("manifest.json");
        if manifest_path.is_file() {
            return Some(manifest_path);
        }
    }

    if path
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("manifest.json"))
        .unwrap_or(false)
    {
        return Some(path.to_path_buf());
    }

    None
}

fn is_model_candidate(path: &Path) -> bool {
    is_supported_model_file(path) || manifest_path_for_bundle(path).is_some()
}

fn discover_wake_word_model_paths() -> Vec<PathBuf> {
    let mut models = Vec::new();

    for search_path in wake_word_model_search_paths() {
        let Ok(entries) = fs::read_dir(&search_path) else {
            continue;
        };

        let mut search_path_models = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if is_model_candidate(&path) {
                search_path_models.push(path);
            }
        }

        search_path_models.sort();
        models.extend(search_path_models);
    }

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

fn load_model_bundle(path: &Path) -> Result<WakeWordModelBundle, String> {
    let manifest_path = manifest_path_for_bundle(path).ok_or_else(|| {
        "Local wake-word model bundle is invalid: manifest.json is missing.".to_string()
    })?;
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("Could not read wake-word manifest: {err}"))?;
    let manifest = serde_json::from_str::<WakeWordBundleManifest>(&manifest_text)
        .map_err(|err| format!("Wake-word manifest is invalid JSON: {err}"))?;
    let root_path = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| path.to_path_buf());
    let mut missing_files = Vec::new();
    let mut validation_errors = Vec::new();

    for (label, file_name) in [
        ("melspectrogram", manifest.models.melspectrogram.as_deref()),
        ("embedding", manifest.models.embedding.as_deref()),
        ("classifier", manifest.models.classifier.as_deref()),
    ] {
        match file_name {
            Some(file_name) if !file_name.trim().is_empty() => {
                if !root_path.join(file_name).is_file() {
                    missing_files.push(format!("{label}: {file_name}"));
                }
            }
            _ => missing_files.push(format!("{label}: missing from manifest")),
        }
    }

    let runtime = manifest.runtime.as_deref().unwrap_or("onnx");
    if !runtime.eq_ignore_ascii_case("onnx") {
        validation_errors.push(format!("runtime: unsupported runtime '{runtime}'"));
    }

    if manifest
        .sample_rate
        .unwrap_or(DEFAULT_WAKE_WORD_SAMPLE_RATE)
        != DEFAULT_WAKE_WORD_SAMPLE_RATE
    {
        validation_errors.push(format!(
            "sampleRate: expected {DEFAULT_WAKE_WORD_SAMPLE_RATE} Hz for the current local provider"
        ));
    }

    if manifest.frame_ms.unwrap_or(DEFAULT_WAKE_WORD_FRAME_MS) == 0 {
        validation_errors.push("frameMs: must be greater than 0".into());
    }

    let threshold = manifest.threshold.unwrap_or(DEFAULT_WAKE_WORD_THRESHOLD);
    if !(0.0..=1.0).contains(&threshold) {
        validation_errors.push("threshold: must be between 0.0 and 1.0".into());
    }

    Ok(WakeWordModelBundle {
        bundle_dir: root_path,
        manifest_path,
        manifest,
        missing_files,
        validation_errors,
    })
}

fn resolve_onnx_runtime_path(bundle: &WakeWordModelBundle) -> Option<PathBuf> {
    std::env::var_os(ONNX_RUNTIME_PATH_ENV)
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| {
            let candidates = [
                bundle.bundle_dir.join("onnxruntime.dll"),
                bundle.bundle_dir.join("runtime").join("onnxruntime.dll"),
            ];
            candidates.into_iter().find(|path| path.is_file())
        })
}

fn ensure_onnx_runtime_loaded(
    bundle: &WakeWordModelBundle,
) -> Result<PathBuf, WakeWordRuntimeError> {
    static ONNX_RUNTIME_INIT: OnceLock<Result<PathBuf, String>> = OnceLock::new();

    ONNX_RUNTIME_INIT
        .get_or_init(|| {
            let runtime_path = resolve_onnx_runtime_path(bundle).ok_or_else(|| {
                format!(
                    "Local wake-word runtime is missing or could not be loaded. Set {ONNX_RUNTIME_PATH_ENV} to onnxruntime.dll."
                )
            })?;
            ort::init_from(&runtime_path)
                .map_err(|err| format!("Could not load ONNX Runtime from {}: {err}", runtime_path.display()))?
                .commit();
            Ok(runtime_path)
        })
        .clone()
        .map_err(WakeWordRuntimeError::RuntimeMissing)
}

fn load_onnx_session(path: &Path) -> Result<OnnxSession, WakeWordRuntimeError> {
    let session = Session::builder()
        .map_err(|err| {
            WakeWordRuntimeError::RuntimeLoadFailed(format!(
                "Could not create ONNX session builder: {err}"
            ))
        })?
        .commit_from_file(path)
        .map_err(|err| {
            WakeWordRuntimeError::RuntimeLoadFailed(format!(
                "Could not load ONNX model {}: {err}",
                path.display()
            ))
        })?;
    let input_shape = session
        .inputs()
        .first()
        .map(|input| format!("{}: {:?}", input.name(), input.dtype()));
    let output_shape = session
        .outputs()
        .first()
        .map(|output| format!("{}: {:?}", output.name(), output.dtype()));

    Ok(OnnxSession {
        session,
        input_shape,
        output_shape,
    })
}

fn model_path_from_manifest(
    bundle: &WakeWordModelBundle,
    file_name: Option<&str>,
) -> Result<Option<PathBuf>, WakeWordRuntimeError> {
    file_name
        .filter(|file_name| !file_name.trim().is_empty())
        .map(|file_name| {
            let path = bundle.bundle_dir.join(file_name);
            if path.is_file() {
                Ok(path)
            } else {
                Err(WakeWordRuntimeError::InvalidBundle(format!(
                    "Wake-word model file is missing: {}",
                    path.display()
                )))
            }
        })
        .transpose()
}

fn load_onnx_wake_word_runtime(
    bundle: &WakeWordModelBundle,
) -> Result<OnnxWakeWordRuntime, WakeWordRuntimeError> {
    if !bundle.missing_files.is_empty() || !bundle.validation_errors.is_empty() {
        let mut errors = bundle.missing_files.clone();
        errors.extend(bundle.validation_errors.clone());
        return Err(WakeWordRuntimeError::InvalidBundle(format!(
            "Local wake-word model bundle is invalid. {}",
            errors.join(", ")
        )));
    }

    let runtime = bundle.manifest.runtime.as_deref().unwrap_or("onnx");
    if !runtime.eq_ignore_ascii_case("onnx") {
        return Err(WakeWordRuntimeError::UnsupportedRuntime(format!(
            "Unsupported wake-word runtime: {runtime}"
        )));
    }

    let runtime_path = ensure_onnx_runtime_loaded(bundle)?;
    log(format!(
        "loading ONNX wake-word bundle; manifest={}, runtime={}",
        bundle.manifest_path.display(),
        runtime_path.display()
    ));

    let melspectrogram =
        model_path_from_manifest(bundle, bundle.manifest.models.melspectrogram.as_deref())?
            .map(|path| load_onnx_session(&path))
            .transpose()?;
    let embedding = model_path_from_manifest(bundle, bundle.manifest.models.embedding.as_deref())?
        .map(|path| load_onnx_session(&path))
        .transpose()?;
    let classifier_path =
        model_path_from_manifest(bundle, bundle.manifest.models.classifier.as_deref())?
            .ok_or_else(|| {
                WakeWordRuntimeError::InvalidBundle("Wake-word classifier model is missing.".into())
            })?;
    let classifier = load_onnx_session(&classifier_path)?;

    Ok(OnnxWakeWordRuntime {
        manifest: bundle.manifest.clone(),
        melspectrogram,
        embedding,
        classifier,
    })
}

fn run_wake_word_inference(
    runtime: &mut OnnxWakeWordRuntime,
    _frame: &[f32],
    _sample_rate: u32,
) -> Result<WakeWordInferenceResult, WakeWordRuntimeError> {
    let phrase = runtime
        .manifest
        .phrase
        .clone()
        .unwrap_or_else(default_wake_word_phrase);
    let _classifier_inputs = runtime.classifier.session.inputs().len();
    Err(WakeWordRuntimeError::PipelineIncomplete(format!(
        "Local wake-word inference pipeline is not complete yet. Loaded ONNX bundle for '{phrase}', but the openWakeWord mel/embedding/classifier chaining is not wired."
    )))
}

fn model_bundle_for_status(status: &WakeWordModelStatus) -> Option<WakeWordModelBundle> {
    status
        .resolved_model_path
        .as_deref()
        .and_then(|path| load_model_bundle(Path::new(path)).ok())
}

fn model_bundle_for_path(path: Option<&Path>) -> Option<WakeWordModelBundle> {
    path.and_then(|path| load_model_bundle(path).ok())
}

fn model_status_for_settings(settings: &WakeWordSettings) -> WakeWordModelStatus {
    let search_paths = wake_word_model_search_paths();
    let discovered_paths = discover_wake_word_model_paths();

    let configured_path = settings.wake_word_model_path.as_deref();
    let resolved_path = configured_path
        .and_then(|raw| resolve_configured_model_path(raw, &search_paths))
        .or_else(|| discovered_paths.first().cloned());

    let bundle = resolved_path
        .as_deref()
        .and_then(|path| load_model_bundle(path).ok());
    let manifest_path = bundle
        .as_ref()
        .map(|bundle| path_string(&bundle.manifest_path))
        .or_else(|| {
            resolved_path
                .as_deref()
                .and_then(manifest_path_for_bundle)
                .as_deref()
                .map(path_string)
        });
    let missing_files = bundle
        .as_ref()
        .map(|bundle| {
            let mut errors = bundle.missing_files.clone();
            errors.extend(bundle.validation_errors.clone());
            errors
        })
        .unwrap_or_else(|| {
            if resolved_path
                .as_deref()
                .map(|path| path.exists() && !manifest_path_for_bundle(path).is_some())
                .unwrap_or(false)
                && is_local_provider(&settings.wake_word_provider)
            {
                vec!["manifest.json".into()]
            } else if manifest_path.is_some() && bundle.is_none() {
                vec!["manifest.json: invalid or unreadable".into()]
            } else {
                Vec::new()
            }
        });
    let manifest_valid = bundle
        .as_ref()
        .map(|bundle| bundle.missing_files.is_empty() && bundle.validation_errors.is_empty())
        .unwrap_or(false);
    let model_exists = resolved_path
        .as_deref()
        .map(|path| {
            if manifest_path_for_bundle(path).is_some() {
                manifest_valid
            } else {
                is_supported_model_file(path)
            }
        })
        .unwrap_or(false);
    let model_missing = is_local_provider(&settings.wake_word_provider)
        && resolved_path
            .as_deref()
            .map(|path| !path.exists())
            .unwrap_or(true);
    let runtime = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.runtime.clone())
        .or_else(|| {
            resolved_path.as_deref().and_then(|path| {
                path.extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.to_lowercase())
            })
        });
    let _manifest_id = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.id.clone());
    let _manifest_provider = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.provider.clone());
    let phrase = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.phrase.clone())
        .or_else(|| Some(settings.wake_word_phrase.clone()));
    let sample_rate = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.sample_rate)
        .or(Some(DEFAULT_WAKE_WORD_SAMPLE_RATE));
    let threshold = bundle
        .as_ref()
        .and_then(|bundle| bundle.manifest.threshold)
        .or(Some(
            settings
                .wake_word_sensitivity
                .max(DEFAULT_WAKE_WORD_THRESHOLD),
        ));
    let runtime_state = if model_missing {
        "model_missing".to_string()
    } else if !manifest_valid && is_local_provider(&settings.wake_word_provider) {
        "invalid_model_bundle".to_string()
    } else if runtime
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("onnx"))
        .unwrap_or(false)
    {
        model_bundle_for_path(resolved_path.as_deref())
            .and_then(|bundle| resolve_onnx_runtime_path(&bundle))
            .map(|_| "runtime_configured".to_string())
            .unwrap_or_else(|| "runtime_missing".to_string())
    } else {
        "unsupported_runtime".to_string()
    };
    let runtime_error = if runtime_state == "runtime_missing"
        && is_local_provider(&settings.wake_word_provider)
        && manifest_valid
    {
        Some(format!(
            "Local wake-word runtime is missing or could not be loaded. Set {ONNX_RUNTIME_PATH_ENV} to onnxruntime.dll."
        ))
    } else if runtime_state == "unsupported_runtime" {
        runtime
            .as_ref()
            .map(|runtime| format!("Unsupported wake-word runtime: {runtime}"))
    } else {
        None
    };

    WakeWordModelStatus {
        configured_model_path: settings.wake_word_model_path.clone(),
        resolved_model_path: resolved_path.as_deref().map(path_string),
        model_exists,
        model_missing,
        manifest_path,
        manifest_valid,
        missing_files,
        runtime,
        phrase,
        sample_rate,
        threshold,
        discovered_models: discovered_paths
            .iter()
            .map(|path| path_string(path))
            .collect(),
        search_paths: search_paths.iter().map(|path| path_string(path)).collect(),
        provider: settings.wake_word_provider.clone(),
        runtime_state,
        loaded_model_count: 0,
        classifier_input_shape: None,
        classifier_output_shape: None,
        runtime_error,
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
            runtime_state: "not_required".into(),
            provider_ready: true,
            provider_configured: true,
            model_path: None,
            model_missing: false,
            manifest_path: None,
            manifest_valid: false,
            missing_files: Vec::new(),
            sample_rate: Some(DEFAULT_WAKE_WORD_SAMPLE_RATE),
            threshold: None,
            message: "Mic test is active. No wake-word model is running.".into(),
            last_error: None,
            start_allowed: true,
            start_state: "listening".into(),
            loaded_model_count: 0,
            classifier_input_shape: None,
            classifier_output_shape: None,
            runtime_error: None,
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
            runtime_state: "mock".into(),
            provider_ready: true,
            provider_configured: true,
            model_path: None,
            model_missing: false,
            manifest_path: None,
            manifest_valid: false,
            missing_files: Vec::new(),
            sample_rate: Some(DEFAULT_WAKE_WORD_SAMPLE_RATE),
            threshold: Some(self.threshold),
            message: "Mock wake-word detection is active for development only.".into(),
            last_error: None,
            start_allowed: true,
            start_state: "listening".into(),
            loaded_model_count: 0,
            classifier_input_shape: None,
            classifier_output_shape: None,
            runtime_error: None,
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
            runtime: None,
            runtime_error: None,
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
                manifest_path: None,
                manifest_valid: false,
                missing_files: Vec::new(),
                runtime: None,
                phrase: None,
                sample_rate: None,
                threshold: None,
                discovered_models: Vec::new(),
                search_paths: wake_word_model_search_paths()
                    .iter()
                    .map(|path| path_string(path))
                    .collect(),
                provider,
                runtime_state: "not_configured".into(),
                loaded_model_count: 0,
                classifier_input_shape: None,
                classifier_output_shape: None,
                runtime_error: None,
            },
            runtime: None,
            runtime_error: None,
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
                runtime_state: "not_configured".into(),
                provider_ready: false,
                provider_configured: false,
                model_path: None,
                model_missing: false,
                manifest_path: None,
                manifest_valid: false,
                missing_files: Vec::new(),
                sample_rate: None,
                threshold: None,
                message: message.clone(),
                last_error: Some(message.clone()),
                start_allowed: false,
                start_state: "provider_missing".into(),
                loaded_model_count: 0,
                classifier_input_shape: None,
                classifier_output_shape: None,
                runtime_error: Some(message),
            };
        }

        if self.model_status.model_missing {
            let message =
                "Local wake-word provider selected, but no model is installed.".to_string();
            return WakeWordProviderAvailability {
                provider_state: "model_missing".into(),
                runtime_state: "model_missing".into(),
                provider_ready: false,
                provider_configured: false,
                model_path: self.model_status.resolved_model_path.clone(),
                model_missing: true,
                manifest_path: self.model_status.manifest_path.clone(),
                manifest_valid: self.model_status.manifest_valid,
                missing_files: self.model_status.missing_files.clone(),
                sample_rate: self.model_status.sample_rate,
                threshold: self.model_status.threshold,
                message: message.clone(),
                last_error: Some(message.clone()),
                start_allowed: false,
                start_state: "model_missing".into(),
                loaded_model_count: self.model_status.loaded_model_count,
                classifier_input_shape: self.model_status.classifier_input_shape.clone(),
                classifier_output_shape: self.model_status.classifier_output_shape.clone(),
                runtime_error: Some(message),
            };
        }

        if !self.model_status.manifest_valid || !self.model_status.missing_files.is_empty() {
            let message = if self.model_status.missing_files.is_empty() {
                "Local wake-word model bundle is invalid.".to_string()
            } else {
                format!(
                    "Local wake-word model bundle is invalid. Missing files: {}",
                    self.model_status.missing_files.join(", ")
                )
            };
            return WakeWordProviderAvailability {
                provider_state: "invalid_model_bundle".into(),
                runtime_state: "invalid_model_bundle".into(),
                provider_ready: false,
                provider_configured: false,
                model_path: self.model_status.resolved_model_path.clone(),
                model_missing: false,
                manifest_path: self.model_status.manifest_path.clone(),
                manifest_valid: self.model_status.manifest_valid,
                missing_files: self.model_status.missing_files.clone(),
                sample_rate: self.model_status.sample_rate,
                threshold: self.model_status.threshold,
                message: message.clone(),
                last_error: Some(message.clone()),
                start_allowed: false,
                start_state: "invalid_model_bundle".into(),
                loaded_model_count: self.model_status.loaded_model_count,
                classifier_input_shape: self.model_status.classifier_input_shape.clone(),
                classifier_output_shape: self.model_status.classifier_output_shape.clone(),
                runtime_error: Some(message),
            };
        }

        let runtime_path = model_bundle_for_status(&self.model_status)
            .and_then(|bundle| resolve_onnx_runtime_path(&bundle));
        if runtime_path.is_none() {
            let message = format!(
                "Local wake-word runtime is missing or could not be loaded. Set {ONNX_RUNTIME_PATH_ENV} to onnxruntime.dll."
            );
            return WakeWordProviderAvailability {
                provider_state: "runtime_missing".into(),
                runtime_state: "runtime_missing".into(),
                provider_ready: false,
                provider_configured: true,
                model_path: self.model_status.resolved_model_path.clone(),
                model_missing: false,
                manifest_path: self.model_status.manifest_path.clone(),
                manifest_valid: self.model_status.manifest_valid,
                missing_files: self.model_status.missing_files.clone(),
                sample_rate: self.model_status.sample_rate,
                threshold: self.model_status.threshold,
                message: message.clone(),
                last_error: Some(message.clone()),
                start_allowed: false,
                start_state: "runtime_missing".into(),
                loaded_model_count: self.model_status.loaded_model_count,
                classifier_input_shape: self.model_status.classifier_input_shape.clone(),
                classifier_output_shape: self.model_status.classifier_output_shape.clone(),
                runtime_error: Some(message),
            };
        }

        let message =
            "Local wake-word model bundle is ready; ONNX runtime will load when listening starts."
                .to_string();
        WakeWordProviderAvailability {
            provider_state: "runtime_configured".into(),
            runtime_state: self.model_status.runtime_state.clone(),
            provider_ready: true,
            provider_configured: true,
            model_path: self.model_status.resolved_model_path.clone(),
            model_missing: false,
            manifest_path: self.model_status.manifest_path.clone(),
            manifest_valid: self.model_status.manifest_valid,
            missing_files: self.model_status.missing_files.clone(),
            sample_rate: self.model_status.sample_rate,
            threshold: self.model_status.threshold,
            message: message.clone(),
            last_error: None,
            start_allowed: true,
            start_state: "listening".into(),
            loaded_model_count: self.model_status.loaded_model_count,
            classifier_input_shape: self.model_status.classifier_input_shape.clone(),
            classifier_output_shape: self.model_status.classifier_output_shape.clone(),
            runtime_error: None,
        }
    }

    fn target_sample_rate(&self) -> u32 {
        self.model_status
            .sample_rate
            .unwrap_or(DEFAULT_WAKE_WORD_SAMPLE_RATE)
    }

    fn frame_ms(&self) -> u32 {
        model_bundle_for_status(&self.model_status)
            .and_then(|bundle| bundle.manifest.frame_ms)
            .unwrap_or(DEFAULT_WAKE_WORD_FRAME_MS)
    }

    fn process_audio_frame(&mut self, frame: &[f32], sample_rate: u32) -> WakeWordProviderResult {
        if self.model_status.model_missing {
            WakeWordProviderResult::ModelMissing(
                "Local wake-word provider selected, but no model is installed.".into(),
            )
        } else if !self.model_status.manifest_valid || !self.model_status.missing_files.is_empty() {
            WakeWordProviderResult::InvalidModelBundle(
                "Local wake-word model bundle is invalid.".into(),
            )
        } else {
            let Some(bundle) = model_bundle_for_status(&self.model_status) else {
                return WakeWordProviderResult::InvalidModelBundle(
                    "Local wake-word model bundle is invalid.".into(),
                );
            };

            if self.runtime.is_none() {
                match OnnxWakeWordRuntime::load(&bundle) {
                    Ok(runtime) => {
                        update_runtime_loaded_status(&self.model_status, &runtime);
                        self.runtime = Some(runtime);
                        self.runtime_error = None;
                    }
                    Err(WakeWordRuntimeError::RuntimeMissing(message))
                    | Err(WakeWordRuntimeError::RuntimeLoadFailed(message)) => {
                        self.runtime_error = Some(message.clone());
                        return WakeWordProviderResult::RuntimeMissing(message);
                    }
                    Err(WakeWordRuntimeError::UnsupportedRuntime(message))
                    | Err(WakeWordRuntimeError::InvalidBundle(message)) => {
                        self.runtime_error = Some(message.clone());
                        return WakeWordProviderResult::InvalidModelBundle(message);
                    }
                    Err(WakeWordRuntimeError::PipelineIncomplete(message)) => {
                        self.runtime_error = Some(message.clone());
                        return WakeWordProviderResult::PipelineIncomplete(message);
                    }
                }
            }

            let Some(runtime) = self.runtime.as_mut() else {
                return WakeWordProviderResult::RuntimeMissing(
                    "Local wake-word runtime is missing or could not be loaded.".into(),
                );
            };

            match runtime.run_inference_frame(frame, sample_rate) {
                Ok(result) => {
                    let threshold = runtime
                        .manifest
                        .threshold
                        .or(self.model_status.threshold)
                        .unwrap_or(DEFAULT_WAKE_WORD_THRESHOLD);
                    if result.score >= threshold {
                        WakeWordProviderResult::Detected {
                            score: result.score,
                            phrase: result.phrase,
                        }
                    } else {
                        WakeWordProviderResult::NoMatch
                    }
                }
                Err(WakeWordRuntimeError::PipelineIncomplete(message)) => {
                    self.runtime_error = Some(message.clone());
                    WakeWordProviderResult::PipelineIncomplete(message)
                }
                Err(WakeWordRuntimeError::RuntimeMissing(message))
                | Err(WakeWordRuntimeError::RuntimeLoadFailed(message)) => {
                    self.runtime_error = Some(message.clone());
                    WakeWordProviderResult::RuntimeMissing(message)
                }
                Err(WakeWordRuntimeError::UnsupportedRuntime(message))
                | Err(WakeWordRuntimeError::InvalidBundle(message)) => {
                    self.runtime_error = Some(message.clone());
                    WakeWordProviderResult::InvalidModelBundle(message)
                }
            }
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

struct WakeWordFrameQueue {
    samples: VecDeque<f32>,
    frame_samples: usize,
}

impl WakeWordFrameQueue {
    fn new(sample_rate: u32, frame_ms: u32) -> Self {
        let frame_samples = ((sample_rate as usize * frame_ms as usize) / 1_000).max(1);
        Self {
            samples: VecDeque::with_capacity(frame_samples * 2),
            frame_samples,
        }
    }

    fn push(&mut self, samples: Vec<f32>) {
        self.samples.extend(samples);
        let max_len = self.frame_samples * 8;
        while self.samples.len() > max_len {
            let _ = self.samples.pop_front();
        }
    }

    fn next_frame(&mut self) -> Option<Vec<f32>> {
        if self.samples.len() < self.frame_samples {
            return None;
        }

        Some(self.samples.drain(..self.frame_samples).collect())
    }
}

fn normalize_audio_frame(
    samples: &[f32],
    source_sample_rate: u32,
    source_channels: u16,
    target_sample_rate: u32,
) -> Vec<f32> {
    if samples.is_empty() || source_sample_rate == 0 || target_sample_rate == 0 {
        return Vec::new();
    }

    let channels = usize::from(source_channels.max(1));
    let mono = if channels == 1 {
        samples
            .iter()
            .map(|sample| sample.clamp(-1.0, 1.0))
            .collect::<Vec<_>>()
    } else {
        samples
            .chunks(channels)
            .map(|chunk| {
                let sum = chunk
                    .iter()
                    .map(|sample| sample.clamp(-1.0, 1.0))
                    .sum::<f32>();
                (sum / chunk.len() as f32).clamp(-1.0, 1.0)
            })
            .collect::<Vec<_>>()
    };

    if source_sample_rate == target_sample_rate {
        return mono;
    }

    let target_len =
        ((mono.len() as u64 * target_sample_rate as u64) / source_sample_rate as u64) as usize;
    if target_len == 0 {
        return Vec::new();
    }

    let ratio = source_sample_rate as f32 / target_sample_rate as f32;
    (0..target_len)
        .map(|index| {
            let source_pos = index as f32 * ratio;
            let low = source_pos.floor() as usize;
            let high = (low + 1).min(mono.len().saturating_sub(1));
            let frac = source_pos - low as f32;
            let low_value = mono.get(low).copied().unwrap_or(0.0);
            let high_value = mono.get(high).copied().unwrap_or(low_value);
            (low_value + (high_value - low_value) * frac).clamp(-1.0, 1.0)
        })
        .collect()
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
    let channels = stream_config.channels;
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
        .spawn(move || {
            run_provider_worker(app, detection_settings, sample_rate, channels, frame_rx)
        });

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
        "mic listener started; provider={}, device={device_name}, sample_rate={sample_rate}, channels={channels}",
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
    channels: u16,
    frame_rx: mpsc::Receiver<Vec<f32>>,
) {
    let mut provider = make_provider(&settings);
    let provider_name = provider.name().to_string();
    let target_sample_rate = provider.target_sample_rate();
    let frame_ms = provider.frame_ms();
    let mut queue = WakeWordFrameQueue::new(target_sample_rate, frame_ms);

    log(format!(
        "provider worker started; provider={provider_name}, sample_rate={sample_rate}, channels={channels}, target_sample_rate={target_sample_rate}, frame_ms={frame_ms}"
    ));

    for input_frame in frame_rx {
        let normalized =
            normalize_audio_frame(&input_frame, sample_rate, channels, target_sample_rate);
        queue.push(normalized);

        while let Some(frame) = queue.next_frame() {
            match provider.process_audio_frame(&frame, target_sample_rate) {
                WakeWordProviderResult::NoMatch => {}
                WakeWordProviderResult::Detected { score, phrase } => {
                    handle_wake_word_detected(&app, &settings, &provider_name, phrase, score);
                }
                WakeWordProviderResult::ModelMissing(message) => {
                    set_runtime_error_state(&settings, "model_missing", message);
                    return;
                }
                WakeWordProviderResult::ProviderMissing(message) => {
                    set_runtime_error_state(&settings, "provider_missing", message);
                    return;
                }
                WakeWordProviderResult::RuntimeMissing(message) => {
                    set_runtime_error_state(&settings, "runtime_missing", message);
                    return;
                }
                WakeWordProviderResult::InvalidModelBundle(message) => {
                    set_runtime_error_state(&settings, "invalid_model_bundle", message);
                    return;
                }
                WakeWordProviderResult::PipelineIncomplete(message) => {
                    set_runtime_error_state(
                        &settings,
                        "model_loaded_runtime_pipeline_incomplete",
                        message,
                    );
                    return;
                }
                WakeWordProviderResult::Error(message) => {
                    set_runtime_error_state(&settings, "error", message);
                    return;
                }
            }
        }
    }

    log(format!("provider worker stopped; provider={provider_name}"));
}

fn set_runtime_error_state(settings: &WakeWordSettings, state: &str, message: String) {
    let mut status =
        status_for_state_with_settings(settings, state, message.clone(), Some(message));
    status.listening = false;
    status.runtime_error = status.last_error.clone();
    let _ = set_status(status);
}

fn update_runtime_loaded_status(model_status: &WakeWordModelStatus, runtime: &OnnxWakeWordRuntime) {
    let Ok(mut guard) = status_store().lock() else {
        return;
    };

    guard.provider_state = "model_loaded".into();
    guard.runtime_state = "model_loaded".into();
    guard.provider_ready = true;
    guard.loaded_model_count = runtime.loaded_model_count();
    guard.classifier_input_shape = runtime.classifier_input_shape();
    guard.classifier_output_shape = runtime.classifier_output_shape();
    guard.runtime_error = None;
    guard.provider_error = None;
    guard.model_path = model_status.resolved_model_path.clone();
    guard.manifest_path = model_status.manifest_path.clone();
    guard.manifest_valid = model_status.manifest_valid;
    guard.message = "Local wake-word model loaded.".into();
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
    config.wake_word_auto_listen_enabled = settings.wake_word_auto_listen_enabled;
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
        let availability = provider_availability(&settings);
        current.model_path = status.resolved_model_path.clone();
        current.model_missing = status.model_missing;
        current.manifest_path = status.manifest_path.clone();
        current.manifest_valid = status.manifest_valid;
        current.missing_files = status.missing_files.clone();
        current.sample_rate = status.sample_rate;
        current.threshold = status.threshold;
        current.provider_state = availability.provider_state;
        current.runtime_state = availability.runtime_state;
        current.provider_ready = availability.provider_ready;
        current.provider_error = availability.last_error;
        current.loaded_model_count = availability.loaded_model_count;
        current.classifier_input_shape = availability.classifier_input_shape;
        current.classifier_output_shape = availability.classifier_output_shape;
        current.runtime_error = availability.runtime_error;
    }

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_manifest(dir: &Path, threshold: f32) {
        let manifest = format!(
            r#"{{
  "id": "hey-openblob",
  "provider": "local-openwakeword",
  "phrase": "hey openblob",
  "runtime": "onnx",
  "sampleRate": 16000,
  "frameMs": 80,
  "threshold": {threshold},
  "models": {{
    "melspectrogram": "melspectrogram.onnx",
    "embedding": "embedding.onnx",
    "classifier": "hey-openblob.onnx"
  }}
}}"#
        );
        fs::write(dir.join("manifest.json"), manifest).unwrap();
    }

    #[test]
    fn load_model_bundle_accepts_complete_manifest() {
        let dir = tempdir().unwrap();
        write_manifest(dir.path(), 0.5);
        for file in ["melspectrogram.onnx", "embedding.onnx", "hey-openblob.onnx"] {
            fs::write(dir.path().join(file), b"placeholder").unwrap();
        }

        let bundle = load_model_bundle(dir.path()).unwrap();

        assert!(bundle.missing_files.is_empty());
        assert!(bundle.validation_errors.is_empty());
        assert_eq!(
            bundle.manifest.sample_rate,
            Some(DEFAULT_WAKE_WORD_SAMPLE_RATE)
        );
        assert_eq!(bundle.manifest.threshold, Some(0.5));
    }

    #[test]
    fn load_model_bundle_reports_missing_model_files() {
        let dir = tempdir().unwrap();
        write_manifest(dir.path(), 0.5);

        let bundle = load_model_bundle(dir.path()).unwrap();

        assert_eq!(bundle.missing_files.len(), 3);
        assert!(bundle
            .missing_files
            .iter()
            .any(|entry| entry.contains("classifier")));
    }

    #[test]
    fn load_model_bundle_rejects_invalid_threshold() {
        let dir = tempdir().unwrap();
        write_manifest(dir.path(), 1.5);
        for file in ["melspectrogram.onnx", "embedding.onnx", "hey-openblob.onnx"] {
            fs::write(dir.path().join(file), b"placeholder").unwrap();
        }

        let bundle = load_model_bundle(dir.path()).unwrap();

        assert!(bundle
            .validation_errors
            .iter()
            .any(|entry| entry.contains("threshold")));
    }

    #[test]
    fn local_provider_reports_model_missing_for_missing_configured_path() {
        let dir = tempdir().unwrap();
        let settings = WakeWordSettings {
            wake_word_enabled: true,
            wake_word_phrase: "hey openblob".into(),
            wake_word_sensitivity: DEFAULT_WAKE_WORD_THRESHOLD,
            wake_word_provider: "local-openwakeword".into(),
            wake_word_model_path: Some(path_string(&dir.path().join("missing"))),
            wake_word_auto_listen_enabled: false,
        };

        let availability = provider_availability(&settings);

        assert_eq!(availability.start_state, "model_missing");
        assert!(availability.model_missing);
        assert!(!availability.start_allowed);
    }
}
