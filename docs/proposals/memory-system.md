# Persistent Memory System Proposal

Status: proposed  
Related issue: #13

## Summary

OpenBlob already writes memory, but the runtime does not yet close the loop by retrieving that memory and giving it back to the local model. This proposal defines a phased long-term memory system that keeps the current local-first behavior, migrates the JSON/JSONL memory files into a safer store, and adds bounded memory retrieval to Ollama prompts without blocking desktop commands.

The recommended direction is:

- Use SQLite as the durable memory store.
- Keep writes non-blocking through a bounded `MemoryEvent` channel.
- Keep the current JSON memory backend available during the transition.
- Use Ollama embeddings by default so memory remains local.
- Add retrieval behind a feature flag before replacing any existing behavior.

## Goals

- Make memory useful inside the companion prompt, not only persisted on disk.
- Preserve the current Windows experience while memory internals evolve.
- Avoid blocking command execution on database, embedding, or model work.
- Respect the existing privacy controls in `PrivacyConfig`.
- Provide a migration path from `episodic_memory.jsonl`, `semantic_memory.json`, `user_profile.json`, `personality_state.json`, and `bonding_state.json`.
- Give contributors a clear sequence of small PRs instead of one very large rewrite.

## Non-Goals

- Cross-device sync.
- Cloud embeddings or hosted memory services.
- Multi-profile UI in the first implementation.
- Complex graph reasoning beyond indexed fact retrieval.
- Removing the current JSON files before the SQLite path has shipped and been tested.

## Current State

Current memory lives in `src-tauri/src/modules/memory/` and `src-tauri/src/modules/storage/paths.rs`.

Today:

- `episodic_memory.rs` appends command history to `memory/episodic_memory.jsonl`.
- `semantic_memory.rs` rewrites `memory/semantic_memory.json`.
- `memory_runtime.rs` updates bonding, profile, semantic memory, and episodic memory after successful commands.
- `ollama_text_runtime.rs` builds a system prompt from identity and language, but does not retrieve episodic or semantic memory.
- Blob Connectors read the same local memory files directly.

That gives OpenBlob memory plumbing, but not yet a memory loop.

## Proposed Architecture

### Layers

| Layer | Purpose | First storage target |
| --- | --- | --- |
| Working memory | Current in-process session state such as last command, active app, browser state | Rust structs |
| Episodic memory | Immutable events from desktop commands, chat turns, snips, transcripts, connectors | SQLite `episodes` |
| Semantic memory | Extracted durable facts about user preferences, projects, apps, and recurring topics | SQLite `facts` |
| Reflective memory | Session, daily, and weekly summaries generated locally | SQLite `summaries` |
| Companion state | Personality, bonding, and profile state that is not event-shaped | SQLite `companion_state` |

### Write Path

Command handlers should not write directly to SQLite. They should emit a small event and return quickly.

```text
command / chat / connector / transcript
        |
        v
MemoryEvent
        |
        v
bounded channel
        |
        v
single writer task
        |
        v
SQLite transaction + optional embedding job
```

Rules:

- If the channel is full, commands still succeed.
- If SQLite is unavailable, commands still succeed.
- If embeddings fail, the event is stored without a vector and can be backfilled later.
- The writer owns the SQLite connection to avoid scattered locking.

### Read Path

Before an Ollama call, OpenBlob may build a small memory context block.

```text
user message + active context
        |
        v
retrieve candidates from episodes, facts, summaries
        |
        v
score with recency, importance, text match, vector match, active app/domain
        |
        v
bounded <memory> block
        |
        v
Ollama system prompt
```

The read path must have a strict time budget. A good first target is 150 ms. If retrieval times out, the prompt should be built without memory.

## Prompt Shape

The memory block should be compact and explicit:

```text
<memory>
Known user facts:
- user.preferred_language = en
- user.likes_app = Spotify

Recent activity:
- Today in desktop: opened Spotify successfully.
- Yesterday in browser: searched for Rust Tauri window focus.

Relevant past context:
- User has been testing OpenBlob transcript and voice flows.
</memory>
```

Guidelines:

- Keep it short.
- Prefer facts and summaries over raw logs.
- Do not include sensitive transcript, screen, or connector content unless privacy settings allow it.
- Include timestamps only when they help answer the current question.
- Never let memory override direct user instructions in the current turn.

## Initial SQLite Schema Sketch

This is intentionally a sketch for review, not final migration DDL.

```sql
create table memory_events (
  id text primary key,
  profile_id text not null default 'default',
  created_at text not null,
  source text not null,
  kind text not null,
  privacy_tier text not null default 'redacted',
  app_name text,
  context_domain text,
  user_input text,
  summary text,
  outcome text,
  importance real not null default 0.5,
  metadata_json text not null default '{}'
);

create table facts (
  id text primary key,
  profile_id text not null default 'default',
  subject text not null,
  predicate text not null,
  object text not null,
  confidence real not null default 0.5,
  valid_from text not null,
  valid_to text,
  superseded_by text,
  source_event_id text,
  metadata_json text not null default '{}'
);

create table summaries (
  id text primary key,
  profile_id text not null default 'default',
  period text not null,
  starts_at text not null,
  ends_at text not null,
  summary text not null,
  metadata_json text not null default '{}'
);

create virtual table memory_fts using fts5(
  summary,
  user_input,
  content='memory_events',
  content_rowid='rowid'
);
```

Vector storage can be added after the base schema lands. The first implementation can ship FTS and recency scoring without vector search if that lowers risk.

## Privacy Model

The existing `PrivacyConfig` should stay authoritative.

Suggested privacy tiers:

| Tier | Meaning |
| --- | --- |
| `transient` | Keep only in working memory; do not persist |
| `metadata_only` | Store time, source, kind, and outcome only |
| `redacted` | Store summary with basic PII redaction |
| `full` | Store full content when explicitly allowed |

Implementation rules:

- `store_episodic_memory = false` disables persisted episodes.
- `store_semantic_memory = false` disables new fact extraction.
- `allow_screen_history = false` prevents screen/snip content from being stored beyond transient metadata.
- `allow_voice_history = false` prevents transcript and voice content from being stored beyond transient metadata.
- Wipe/export commands should be part of the privacy phase, not deferred indefinitely.

## Migration Plan

Migration should be reversible during the rollout.

1. Add a config field such as `memory.backend = "legacy" | "sqlite" | "dual_write"`.
2. Start with `legacy` as the default.
3. Add SQLite migrations and schema creation.
4. Add `dual_write` for development builds and early testers.
5. Import existing JSON/JSONL files into SQLite without deleting them.
6. Rename imported legacy files only after a successful import, for example `.migrated`.
7. Keep a fallback path that can return to JSON if SQLite fails.

Blob Connectors currently read JSON files directly. They should keep working until either:

- connectors gain a SQLite reader, or
- the Rust backend exposes a local memory context endpoint.

## Phased Implementation

### Phase 0: Proposal and Constraints

- Land this proposal.
- Agree on SQLite as the first durable store.
- Confirm that Ollama-hosted embeddings are acceptable as the default.
- Confirm that memory PRs should stay small and staged.

### Phase 1: Storage Foundations

- Add SQLite dependency and migration runner.
- Add `MemoryEvent` types.
- Add bounded writer channel.
- Add `legacy`, `sqlite`, and `dual_write` config shape.
- Preserve current runtime behavior.

### Phase 2: Episodic Import and Dual Write

- Import `episodic_memory.jsonl`.
- Write new episodes to SQLite.
- Keep JSON writing enabled while `dual_write` is active.
- Add basic tests for migration and failure fallback.

### Phase 3: Basic Retrieval

- Add FTS/recency/importance retrieval.
- Build a bounded memory context block.
- Inject it into `ask_ollama` behind a config flag.
- Add tests that verify prompt size limits and timeout behavior.

### Phase 4: Embeddings

- Add `OllamaEmbedder` for `nomic-embed-text`.
- Store vectors for new events.
- Backfill missing vectors in the background.
- Add vector score as one part of the hybrid ranker.

### Phase 5: Semantic and Reflective Memory

- Extract facts locally from important episodes.
- Add bi-temporal fact updates instead of destructive overwrites.
- Generate session and daily summaries.
- Use facts and summaries before raw episodes in prompts.

### Phase 6: Privacy and UI

- Add private mode.
- Add export and wipe commands.
- Add a memory inspector UI.
- Add retention controls and optional encryption investigation.

## Review Questions

1. Should SQLite be accepted as the first durable backend?
2. Should the first retrieval phase ship FTS-only before vector search?
3. Should `nomic-embed-text` through Ollama be the default embedder?
4. Should Blob Connectors keep reading JSON during migration, or should a local memory endpoint be added first?
5. Which memory settings belong in the first UI pass?

## Suggested First Code PR

After this proposal is accepted, the first code PR should be intentionally small:

- Add `src-tauri/src/modules/memory/events.rs`.
- Add a `MemoryEvent` enum and `PrivacyTier`.
- Add unit tests for event creation and privacy-tier mapping.
- Do not add SQLite yet.
- Do not change current JSON writes yet.

That gives the project a stable interface for future storage changes without forcing the full implementation into one review.
