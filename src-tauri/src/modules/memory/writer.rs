use std::path::PathBuf;
use std::sync::OnceLock;
use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};

use crate::modules::memory::events::{MemoryEvent, PrivacyTier};
use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database_at};
use crate::modules::storage::paths::memory_database_path;

pub const DEFAULT_MEMORY_EVENT_QUEUE_CAPACITY: usize = 256;

static MEMORY_WRITER: OnceLock<MemoryWriterHandle> = OnceLock::new();

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
        if event.privacy_tier == PrivacyTier::Transient {
            return EnqueueMemoryEventResult::SkippedTransient;
        }

        match self.sender.try_send(event) {
            Ok(()) => EnqueueMemoryEventResult::Queued,
            Err(TrySendError::Full(_)) => EnqueueMemoryEventResult::Full,
            Err(TrySendError::Disconnected(_)) => EnqueueMemoryEventResult::Disconnected,
        }
    }
}

pub fn start_memory_writer() -> Result<(), String> {
    if MEMORY_WRITER.get().is_some() {
        return Ok(());
    }

    let db_path = memory_database_path()?;
    let (sender, receiver) = bounded(DEFAULT_MEMORY_EVENT_QUEUE_CAPACITY);
    let handle = MemoryWriterHandle::new(sender);

    thread::Builder::new()
        .name("openblob-memory-writer".into())
        .spawn(move || run_memory_writer_at_path(receiver, db_path))
        .map_err(|e| format!("Could not start memory writer: {e}"))?;

    let _ = MEMORY_WRITER.set(handle);
    Ok(())
}

pub fn enqueue_memory_event(event: MemoryEvent) -> EnqueueMemoryEventResult {
    match MEMORY_WRITER.get() {
        Some(writer) => writer.enqueue(event),
        None => EnqueueMemoryEventResult::NotStarted,
    }
}

fn run_memory_writer_at_path(receiver: Receiver<MemoryEvent>, db_path: PathBuf) {
    let conn = match open_memory_database_at(db_path.as_path()) {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("[openblob] Memory writer could not open SQLite database: {err}");
            return;
        }
    };

    for event in receiver {
        if let Err(err) = insert_memory_event(&conn, &event) {
            eprintln!("[openblob] Memory writer failed to persist event: {err}");
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
        let (sender, _receiver) = bounded(1);
        let writer = MemoryWriterHandle::new(sender);

        let first = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Redacted);
        let second = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Redacted);

        assert_eq!(writer.enqueue(first), EnqueueMemoryEventResult::Queued);
        assert_eq!(writer.enqueue(second), EnqueueMemoryEventResult::Full);
    }

    #[test]
    fn enqueue_skips_transient_events() {
        let (sender, receiver) = bounded(1);
        let writer = MemoryWriterHandle::new(sender);
        let event = MemoryEvent::new(MemoryEventKind::Command, "test", PrivacyTier::Transient);

        assert_eq!(
            writer.enqueue(event),
            EnqueueMemoryEventResult::SkippedTransient
        );
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn writer_loop_persists_queued_events() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("memory.db");
        let (sender, receiver) = bounded(4);
        let event = MemoryEvent::successful_command(
            "Spotify",
            "desktop",
            "open spotify",
            "Opened Spotify.",
            "success",
            &PrivacyConfig::default(),
        );
        let event_id = event.id.clone();

        sender.send(event).expect("send event");
        drop(sender);

        run_memory_writer_at_path(receiver, db_path.clone());

        let conn = open_memory_database_at(&db_path).expect("open db");
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
