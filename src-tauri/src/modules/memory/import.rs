use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::modules::memory::episodic_memory::EpisodicMemoryEntry;
use crate::modules::memory::events::{MemoryEvent, MemoryEventKind, PrivacyTier};
use crate::modules::memory::facts::{insert_memory_fact, MemoryFact};
use crate::modules::memory::semantic_memory::SemanticMemory;
use crate::modules::memory::sqlite_store::{insert_memory_event, open_memory_database};
use crate::modules::profile::user_profile::UserProfile;
use crate::modules::storage::json_store::load_json;
use crate::modules::storage::paths::{episodic_memory_path, semantic_memory_path, user_profile_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub source_path: PathBuf,
    pub migrated_path: Option<PathBuf>,
}

pub fn import_legacy_episodic_memory() -> Result<LegacyImportReport, String> {
    let source_path = episodic_memory_path()?;
    let mut conn = open_memory_database()?;
    import_legacy_episodic_memory_from_path(&mut conn, &source_path, true)
}

pub fn import_legacy_semantic_facts() -> Result<LegacyImportReport, String> {
    let conn = open_memory_database()?;
    let mut imported = 0;
    let mut skipped = 0;
    let semantic_path = semantic_memory_path()?;
    let profile_path = user_profile_path()?;

    if semantic_path.exists() {
        match load_json::<SemanticMemory>(&semantic_path) {
            Ok(memory) => {
                for fact in semantic_memory_to_facts(memory.normalized()) {
                    insert_memory_fact(&conn, &fact)?;
                    imported += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    if profile_path.exists() {
        match load_json::<UserProfile>(&profile_path) {
            Ok(profile) => {
                for fact in user_profile_to_facts(profile.normalized()) {
                    insert_memory_fact(&conn, &fact)?;
                    imported += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    Ok(LegacyImportReport {
        imported,
        skipped,
        source_path: semantic_path,
        migrated_path: None,
    })
}

pub fn import_legacy_episodic_memory_from_path(
    conn: &mut rusqlite::Connection,
    source_path: &Path,
    rename_after_import: bool,
) -> Result<LegacyImportReport, String> {
    if !source_path.exists() {
        return Ok(LegacyImportReport {
            imported: 0,
            skipped: 0,
            source_path: source_path.to_path_buf(),
            migrated_path: None,
        });
    }

    let file = fs::File::open(source_path).map_err(|e| {
        format!(
            "Could not open legacy episodic memory '{}': {e}",
            source_path.display()
        )
    })?;
    let reader = BufReader::new(file);
    let tx = conn
        .transaction()
        .map_err(|e| format!("Could not start legacy memory import transaction: {e}"))?;

    let mut imported = 0;
    let mut skipped = 0;

    for line in reader.lines() {
        let line = line.map_err(|e| {
            format!(
                "Could not read legacy episodic memory '{}': {e}",
                source_path.display()
            )
        })?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<EpisodicMemoryEntry>(trimmed) {
            Ok(entry) => {
                let event = legacy_episode_to_memory_event(entry);
                insert_memory_event(&tx, &event)?;
                imported += 1;
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    tx.commit()
        .map_err(|e| format!("Could not commit legacy memory import: {e}"))?;

    let migrated_path = if rename_after_import {
        let migrated_path = next_migrated_path(source_path);
        fs::rename(source_path, &migrated_path).map_err(|e| {
            format!(
                "Could not rename legacy episodic memory '{}' to '{}': {e}",
                source_path.display(),
                migrated_path.display()
            )
        })?;
        Some(migrated_path)
    } else {
        None
    };

    Ok(LegacyImportReport {
        imported,
        skipped,
        source_path: source_path.to_path_buf(),
        migrated_path,
    })
}

fn legacy_episode_to_memory_event(entry: EpisodicMemoryEntry) -> MemoryEvent {
    MemoryEvent {
        version: 1,
        id: format!("mem_legacy_{}", entry.id),
        timestamp: entry.timestamp,
        kind: legacy_kind(&entry.kind),
        source: "legacy_jsonl".to_string(),
        app_name: clean(entry.app_name),
        context_domain: clean(entry.context_domain),
        user_input: clean(entry.user_input),
        summary: clean(entry.summary),
        outcome: clean(entry.outcome),
        importance: entry.importance.clamp(0.0, 1.0),
        privacy_tier: PrivacyTier::Redacted,
        metadata: json!({
            "imported_from": "episodic_memory.jsonl",
            "legacy_kind": entry.kind,
            "legacy_version": entry.version
        }),
    }
}

fn legacy_kind(kind: &str) -> MemoryEventKind {
    match kind {
        "external_command" => MemoryEventKind::ConnectorMessage,
        "chat_turn" => MemoryEventKind::ChatTurn,
        "snip" => MemoryEventKind::Snip,
        "browser_visit" => MemoryEventKind::BrowserVisit,
        "transcript_segment" => MemoryEventKind::TranscriptSegment,
        _ => MemoryEventKind::Command,
    }
}

fn semantic_memory_to_facts(memory: SemanticMemory) -> Vec<MemoryFact> {
    let mut facts = Vec::new();

    for language in memory.preferred_languages {
        facts.push(legacy_fact(
            format!("legacy:semantic:preferred_language:{language}"),
            "user",
            "preferred_language",
            language,
        ));
    }

    for app in memory.favorite_apps {
        facts.push(legacy_fact(
            format!("legacy:semantic:favorite_app:{}", app.to_lowercase()),
            "user",
            "favorite_app",
            app,
        ));
    }

    for topic in memory.recurring_topics {
        facts.push(legacy_fact(
            format!("legacy:semantic:recurring_topic:{}", topic.to_lowercase()),
            "user",
            "recurring_topic",
            topic,
        ));
    }

    if let Some(style) = memory.inferred_user_style {
        facts.push(legacy_fact(
            "legacy:semantic:inferred_user_style",
            "user",
            "inferred_style",
            style,
        ));
    }

    for (index, note) in memory.notes.into_iter().enumerate() {
        facts.push(legacy_fact(
            format!("legacy:semantic:note:{index}:{}", note.to_lowercase()),
            "user",
            "note",
            note,
        ));
    }

    facts
}

fn user_profile_to_facts(profile: UserProfile) -> Vec<MemoryFact> {
    let mut facts = Vec::new();

    if let Some(name) = profile.display_name {
        facts.push(legacy_fact(
            "legacy:profile:display_name",
            "user",
            "name",
            name,
        ));
    }

    for language in profile.languages {
        facts.push(legacy_fact(
            format!("legacy:profile:language:{language}"),
            "user",
            "preferred_language",
            language,
        ));
    }

    if let Some(style) = profile.preferred_response_style {
        facts.push(legacy_fact(
            "legacy:profile:preferred_response_style",
            "user",
            "preferred_response_style",
            style,
        ));
    }

    for app in profile.favorite_apps {
        facts.push(legacy_fact(
            format!("legacy:profile:favorite_app:{}", app.to_lowercase()),
            "user",
            "favorite_app",
            app,
        ));
    }

    for topic in profile.recurring_topics {
        facts.push(legacy_fact(
            format!("legacy:profile:recurring_topic:{}", topic.to_lowercase()),
            "user",
            "recurring_topic",
            topic,
        ));
    }

    facts
}

fn legacy_fact(
    source_key: impl Into<String>,
    subject: impl Into<String>,
    predicate: impl Into<String>,
    object: impl Into<String>,
) -> MemoryFact {
    MemoryFact::new(source_key, subject, predicate, object, "legacy_import")
        .with_confidence(0.78)
        .with_metadata(json!({ "imported_from": "legacy_json" }))
}

fn clean(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn next_migrated_path(source_path: &Path) -> PathBuf {
    let base = source_path.with_file_name(format!(
        "{}.migrated",
        source_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("episodic_memory.jsonl")
    ));

    if !base.exists() {
        return base;
    }

    for index in 1.. {
        let candidate = source_path.with_file_name(format!(
            "{}.migrated.{}",
            source_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("episodic_memory.jsonl"),
            index
        ));

        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded migrated path search should always return")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::memory::sqlite_store::open_memory_database_in_memory;
    use std::io::Write;

    #[test]
    fn missing_legacy_file_is_noop() {
        let mut conn = open_memory_database_in_memory().expect("database opens");
        let dir = tempfile::tempdir().expect("temp dir");
        let source_path = dir.path().join("episodic_memory.jsonl");

        let report =
            import_legacy_episodic_memory_from_path(&mut conn, &source_path, true)
                .expect("import succeeds");

        assert_eq!(report.imported, 0);
        assert_eq!(report.skipped, 0);
        assert!(report.migrated_path.is_none());
    }

    #[test]
    fn imports_valid_legacy_lines_and_renames_source() {
        let mut conn = open_memory_database_in_memory().expect("database opens");
        let dir = tempfile::tempdir().expect("temp dir");
        let source_path = dir.path().join("episodic_memory.jsonl");
        let mut file = fs::File::create(&source_path).expect("legacy file");
        writeln!(
            file,
            "{}",
            serde_json::to_string(&EpisodicMemoryEntry {
                version: 1,
                id: "ep_1".to_string(),
                timestamp: "2026-05-06T10:00:00Z".to_string(),
                kind: "external_command".to_string(),
                app_name: "telegram".to_string(),
                context_domain: "external".to_string(),
                user_input: "open notes".to_string(),
                summary: "Opened notes.".to_string(),
                outcome: "success".to_string(),
                importance: 0.7,
            })
            .expect("serialize legacy entry")
        )
        .expect("write legacy line");
        writeln!(file, "not json").expect("write invalid line");

        let report =
            import_legacy_episodic_memory_from_path(&mut conn, &source_path, true)
                .expect("import succeeds");

        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped, 1);
        assert!(!source_path.exists());
        assert!(report.migrated_path.expect("migrated path").exists());

        let stored: (String, String, String) = conn
            .query_row(
                "SELECT kind, source, summary FROM memory_events WHERE id = 'mem_legacy_ep_1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("stored event");

        assert_eq!(stored.0, "connector_message");
        assert_eq!(stored.1, "legacy_jsonl");
        assert_eq!(stored.2, "Opened notes.");
    }

    #[test]
    fn can_import_without_renaming_for_dry_runs() {
        let mut conn = open_memory_database_in_memory().expect("database opens");
        let dir = tempfile::tempdir().expect("temp dir");
        let source_path = dir.path().join("episodic_memory.jsonl");
        fs::write(
            &source_path,
            r#"{"version":1,"id":"ep_2","timestamp":"2026-05-06T10:00:00Z","kind":"command","app_name":"terminal","context_domain":"desktop","user_input":"test","summary":"Tested memory.","outcome":"success","importance":0.5}"#,
        )
        .expect("write source");

        let report =
            import_legacy_episodic_memory_from_path(&mut conn, &source_path, false)
                .expect("import succeeds");

        assert_eq!(report.imported, 1);
        assert!(source_path.exists());
        assert!(report.migrated_path.is_none());
    }
}
