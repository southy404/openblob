use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TranscriptSourceKind {
    SystemAudio,
    Microphone,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TranscriptState {
    Idle,
    Recording,
    Stopping,
    Summarizing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker: Option<String>,
    pub text: String,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptContext {
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSession {
    pub id: String,
    pub source: TranscriptSourceKind,
    pub state: TranscriptState,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub context: TranscriptContext,
    pub segments: Vec<TranscriptSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptStatus {
    pub state: TranscriptState,
    pub active_session_id: Option<String>,
    pub segment_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTranscriptRequest {
    pub source: TranscriptSourceKind,
    pub app_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TranscriptSummary {
    pub summary: String,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeakerBlock {
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessedTranscriptResult {
    pub faithful_transcript: String,
    pub speaker_blocks: Vec<SpeakerBlock>,
    pub summary: String,
    pub action_items: Vec<String>,
}