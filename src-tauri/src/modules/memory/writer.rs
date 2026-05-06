use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::OnceLock;

use chrono::{DateTime, Duration, Utc};
use tokio::sync::mpsc::{channel, error::TrySendError, Receiver, Sender};

use crate::modules::memory::embeddings::{
    embedding_for_event, insert_embedding, log_embedding_failure_once,
};
use crate::modules::memory::events::{MemoryEvent, PrivacyTier};
use crate::modules::memory::extraction::{
    extract_facts_for_event_async, store_extracted_facts_for_event,
};
use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database_at};
use crate::modules::storage::paths::memory_database_path;

pub const DEFAULT_MEMORY_EVENT_QUEUE_CAPACITY: usize = 256;

static MEMORY_WRITER: OnceLock<MemoryWriterHandle> = OnceLock::new();
static PRIVATE_MODE_UNTIL_TS: AtomicI64 = AtomicI64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueMemoryEventResult {
    Queued,
    SkippedTransient,
    NotStarted,
    Full,
    Disconnected,
}

#[derive(Clone)]
pub struct MemoryWriterHandle {
    sender: Sender<MemoryEvent>,
}

impl MemoryWriterHandle {
    fn new(sender: Sender<MemoryEvent>) -> Self {
        Self { sender }
    }

    pub fn enqueue(&self, event: MemoryEvent) -> EnqueueMemoryEventResult {
        if is_memory_private_mode_active() {
            return EnqueueMemoryEventResult::SkippedTransient;
        }

        if event.privacy_tier == PrivacyTier::Transient {
            return EnqueueMemoryEventResult::SkippedTransient;
        }

        match self.sender.try_send(event) {
            Ok(()) => EnqueueMemoryEventResult::Queued,
            Err(TrySendError::Full(_)) => EnqueueMemoryEventResult::Full,
            Err(TrySendError::Closed(_)) => EnqueueMemoryEventResult::Disconnected,
        }
    }
}

pub fn start_memory_writer() -> Result<(), String> {
    if MEMORY_WRITER.get().is_some() {
        return Ok(());
    }

    let db_path = memory_database_path()?;
    let (sender, receiver) = channel(DEFAULT_MEMORY_EVENT_QUEUE_CAPACITY);
    let handle = MemoryWriterHandle::new(sender);

    tauri::async_runtime::spawn(run_memory_writer_at_path(receiver, db_path));

    let _ = MEMORY_WRITER.set(handle);
    Ok(())
}

pub fn enqueue_memory_event(event: MemoryEvent) -> EnqueueMemoryEventResult {
    match MEMORY_WRITER.get() {
        Some(writer) => writer.enqueue(event),
        None => EnqueueMemoryEventResult::NotStarted,
    }
}

pub fn enable_memory_private_mode(minutes: u32) -> String {
    let minutes = minutes.clamp(1, 24 * 60) as i64;
    let until = Utc::now() + Duration::minutes(minutes);
    PRIVATE_MODE_UNTIL_TS.store(until.timestamp(), Ordering::Relaxed);
    until.to_rfc3339()
}

pub fn clear_memory_private_mode() {
    PRIVATE_MODE_UNTIL_TS.store(0, Ordering::Relaxed);
}

pub fn memory_private_mode_until() -> Option<String> {
    let timestamp = PRIVATE_MODE_UNTIL_TS.load(Ordering::Relaxed);
    if timestamp <= Utc::now().timestamp() {
        if timestamp != 0 {
            clear_memory_private_mode();
        }
        return None;
    }

    DateTime::<Utc>::from_timestamp(timestamp, 0).map(|value| value.to_rfc3339())
}

pub fn is_memory_private_mode_active() -> bool {
    memory_private_mode_until().is_some()
}

async fn run_memory_writer_at_path(mut receiver: Receiver<MemoryEvent>, db_path: PathBuf) {
    while let Some(event) = receiver.recv().await {
        if std::env::var_os("RUST_BACKTRACE").is_some() {
            println!(
                "[openblob] Memory write start; event_id={}; kind={}",
                event.id,
                event.kind.as_str()
            );
        }

        match open_memory_database_at(db_path.as_path()) {
            Ok(conn) => {
                if let Err(err) = insert_memory_event(&conn, &event) {
                    eprintln!("[openblob] Memory writer failed to persist event: {err}");
                }
            }
            Err(err) => {
                eprintln!("[openblob] Memory writer could not open SQLite database: {err}");
                continue;
            }
        }

        match embedding_for_event(&event).await {
            Ok(Some((model, vector))) => match open_memory_database_at(db_path.as_path()) {
                Ok(conn) => {
                    if let Err(err) =
                        insert_embedding(&conn, event.id.as_str(), "event", &model, &vector)
                    {
                        eprintln!("[openblob] Memory writer failed to persist embedding: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("[openblob] Memory writer could not open SQLite database: {err}");
                }
            },
            Ok(None) => {}
            Err(err) => log_embedding_failure_once(&err),
        }

        match extract_facts_for_event_async(&event).await {
            Ok(facts) if !facts.is_empty() => match open_memory_database_at(db_path.as_path()) {
                Ok(conn) => {
                    if let Err(err) = store_extracted_facts_for_event(&conn, &event, facts) {
                        eprintln!("[openblob] Memory writer skipped semantic extraction: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("[openblob] Memory writer could not open SQLite database: {err}");
                }
            },
            Ok(_) => {}
            Err(err) => eprintln!("[openblob] Memory writer skipped semantic extraction: {err}"),
        }

        if std::env::var_os("RUST_BACKTRACE").is_some() {
            println!("[openblob] Memory write end; event_id={}", event.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::events::MemoryEventKind;
    use crate::modules::profile::companion_config::PrivacyConfig;

    #[test]
    fn enqueue_reports_full_without_blocking() {
        let (sender, _receiver) = channel(1);
        let writer = MemoryWriterHandle::new(sender);

        let first = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Redacted);
        let second = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Redacted);

        assert_eq!(writer.enqueue(first), EnqueueMemoryEventResult::Queued);
        assert_eq!(writer.enqueue(second), EnqueueMemoryEventResult::Full);
    }

    #[test]
    fn enqueue_skips_transient_events() {
        let (sender, mut receiver) = channel(1);
        let writer = MemoryWriterHandle::new(sender);
        let event = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Transient);

        assert_eq!(
            writer.enqueue(event),
            EnqueueMemoryEventResult::SkippedTransient
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn writer_persists_raw_events() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("memory.db");
        let conn = open_memory_database_at(&db_path).expect("open db");
        let event = MemoryEvent::successful_command(
            "Spotify",
            "desktop",
            "open spotify",
            "Opened Spotify.",
            "success",
            &PrivacyConfig::default(),
        );
        let event_id = event.id.clone();

        insert_memory_event(&conn, &event).expect("event inserts");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM memory_events WHERE id = ?1",
                [event_id.as_str()],
                |row| row.get(0),
            )
            .expect("event count");

        assert_eq!(count, 1);
    }
}
