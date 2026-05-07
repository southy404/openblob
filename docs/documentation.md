# OpenBlob — Developer Documentation

> **Local-first AI desktop companion for Windows**  
> Built with Tauri v2 · React · Rust · Ollama · Local-first memory · Voice · Vision · Transcript · Connectors

---

## Table of Contents

1. [Overview](#overview)
2. [Current Status](#current-status)
3. [Architecture](#architecture)
4. [Getting Started](#getting-started)
5. [Project Structure](#project-structure)
6. [Core Systems](#core-systems)
   - [Command Router](#command-router)
   - [Capability Execution](#capability-execution)
   - [Active Target Context](#active-target-context)
   - [Browser Automation](#browser-automation)
   - [System and App Launch Runtime](#system-and-app-launch-runtime)
   - [Screen and Vision](#screen-and-vision)
   - [Voice Input](#voice-input)
   - [Wake Word Foundation](#wake-word-foundation)
   - [Transcript System](#transcript-system)
   - [Memory System](#memory-system)
   - [Companion Identity and Profile](#companion-identity-and-profile)
   - [Text-to-Speech](#text-to-speech)
7. [Frontend Windows](#frontend-windows)
8. [Tauri Bridge Layer](#tauri-bridge-layer)
9. [Configuration and Local Data](#configuration-and-local-data)
10. [AI and Model Integration](#ai-and-model-integration)
11. [Global Shortcuts](#global-shortcuts)
12. [Blob Connectors](#blob-connectors)
13. [Command Reference](#command-reference)
14. [Testing and Validation](#testing-and-validation)
15. [Tech Stack](#tech-stack)
16. [Contribution Guide](#contribution-guide)
17. [Known Issues](#known-issues)
18. [Roadmap](#roadmap)
19. [License](#license)

---

## Overview

OpenBlob is an **open-source, local-first desktop companion** for Windows 10/11.

It is not meant to be a simple chatbot window. OpenBlob is designed as a desktop companion that can live on the user's screen, understand context, react visually, execute safe system actions, answer questions, analyze screenshots, transcribe system audio, and be reachable from external channels such as Telegram, Discord, Slack, and Email.

Core product idea:

> Build a desktop copilot that feels alive, useful, extensible, and truly personal — while keeping privacy, user control, and local-first execution at the center.

OpenBlob can currently:

- execute deterministic Windows actions
- open apps, folders, settings, URLs, and media services
- control browser tabs through Chrome / Edge remote debugging
- use local Ollama models for chat, explanation, fallback reasoning, and vision
- analyze screenshots and selected screen regions
- run a transcript system for local system-audio transcription
- persist identity, profile, personality, bonding, episodic memory, and semantic memory
- expose a local command server for external connectors
- receive commands from Telegram, Discord, Slack, and Email through the Python connector layer
- support protected confirmation flows for sensitive power commands
- provide a wake-word settings and microphone-test foundation for future hands-free voice activation

Core design principle:

> **Deterministic first. AI second.**

Whenever a command can be safely and reliably executed through local deterministic code, OpenBlob should do that instead of asking an LLM to guess. AI is used as a capability layer, fallback layer, summarization layer, and intelligence layer — not as the only system controller.

---

## Current Status

OpenBlob is in an **early but active development stage**.

The project already has a functional Tauri/Rust/React foundation and several advanced experimental systems, but some modules are still being stabilized after refactors.

Recent active areas:

- hardening app/media launch routing
- improving Spotify, Steam, Discord, YouTube, and browser fallback behavior
- adding active controlled target context for follow-up commands
- preventing normal chat messages from being swallowed by command routing
- improving local memory foundations
- adding wake-word settings and microphone-test runtime foundation
- expanding dev/settings UI
- improving contributor-friendly structure and documentation

Important current expectation:

- deterministic commands should route through the command/action pipeline
- normal conversation should fall back to the regular LLM chat path
- voice shortcut flow (`ALT + M`) must remain unaffected by wake-word work
- wake-word functionality is currently a **foundation**, not a real hotword detector yet

---

## Architecture

OpenBlob is organized into layered systems.

```text
┌─────────────────────────────────────────────────────────────┐
│                         UI Layer                             │
│ React windows: bubble · dev · quick menu · transcript        │
│ snip overlay · snip panel · timer overlay                    │
└─────────────────────────────┬───────────────────────────────┘
                              │ Tauri invoke / events
┌─────────────────────────────▼───────────────────────────────┐
│                     Tauri Bridge Layer                       │
│ Window management · global shortcuts · event bus · commands  │
└─────────────────────────────┬───────────────────────────────┘
                              │ Rust module calls
┌─────────────────────────────▼───────────────────────────────┐
│                       Runtime Layer                          │
│ Command router · app launch · browser automation · context   │
│ memory · screen capture · transcript · voice · wake-word     │
│ TTS · local HTTP command server                              │
└─────────────────────────────┬───────────────────────────────┘
                              │ Local models / local APIs
┌─────────────────────────────▼───────────────────────────────┐
│                       AI Layer                               │
│ Ollama text models · vision models · Whisper CLI · TTS       │
└─────────────────────────────┬───────────────────────────────┘
                              │ Optional external channels
┌─────────────────────────────▼───────────────────────────────┐
│                  Blob Connectors Layer                       │
│ Python connectors: Telegram · Discord · Slack · Email        │
│ Commands forwarded to localhost:7842 or Ollama fallback      │
└─────────────────────────────────────────────────────────────┘
```

### Main input flow

```text
User input
(text / voice / shortcut / external channel)
        │
        ▼
Normalize input
        │
        ▼
Command router
        │
        ├─ confident deterministic action
        │      ▼
        │   capability executor
        │      ▼
        │   system/app/browser/media action
        │
        └─ no confident action intent
               ▼
            LLM chat / explain / fallback answer
```

### External connector flow

```text
Telegram / Discord / Slack / Email
        │
        ▼
blob_connectors Python process
        │
        ├─ OpenBlob running
        │      ▼
        │   POST localhost:7842/command
        │      ▼
        │   Rust command pipeline
        │
        └─ OpenBlob unavailable
               ▼
            local Ollama fallback response
```

---

## Getting Started

### Requirements

| Dependency | Version | Notes |
| --- | --- | --- |
| Windows | 10 / 11 | Primary target platform |
| Node.js | 18+ | Frontend tooling |
| Rust / Cargo | stable | Tauri backend |
| Tauri CLI | v2 | Recommended through npm script or cargo install |
| Ollama | latest | Local LLM runtime |
| Chrome or Edge | current | Browser automation via remote debugging |
| Python | 3.11+ | Blob Connectors |
| Whisper CLI | local install | Transcript system |

### Install

```bash
git clone https://github.com/southy404/openblob.git
cd openblob
npm install
```

### Ollama setup

```bash
ollama serve
ollama pull llama3.1:8b
ollama pull gemma3

# Optional vision model
ollama pull qwen2.5vl:7b

# Optional embedding model for memory experiments
ollama pull nomic-embed-text
```

### Run desktop app

Recommended:

```bash
npm run tauri dev
```

Alternative from Rust side:

```bash
cd src-tauri
cargo tauri dev
```

If `cargo tauri dev` fails with `no such command: tauri`, install the Tauri CLI or use the npm script:

```bash
cargo install tauri-cli --version "^2"
```

### Build frontend

```bash
npm run build
```

### Rust checks

From the `src-tauri` folder:

```bash
cargo fmt
cargo check -j 1
cargo test --lib -j 1 -- --test-threads=1
```

---

## Project Structure

```text
openblob/
├── public/
│   └── openblob-logo.png
│
├── src/
│   ├── main.tsx
│   └── windows/
│       ├── bubble/
│       │   ├── app.tsx
│       │   └── style.css
│       ├── bubble-dev/
│       │   ├── app.tsx
│       │   └── style.css
│       ├── quick-menu/
│       ├── transcript/
│       ├── snip-overlay/
│       ├── snip-panel/
│       └── timer-overlay/
│
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── core/
│       │   ├── executor/
│       │   └── legacy/
│       ├── modules/
│       │   ├── command_router/
│       │   ├── companion/
│       │   ├── memory/
│       │   ├── profile/
│       │   ├── transcript/
│       │   ├── tts/
│       │   ├── storage/
│       │   ├── browser_automations.rs
│       │   ├── context.rs
│       │   ├── context_resolver.rs
│       │   ├── screen_capture.rs
│       │   ├── session_memory.rs
│       │   ├── steam_games.rs
│       │   ├── system.rs
│       │   ├── voice.rs
│       │   ├── wake_word.rs
│       │   └── windows_discovery.rs
│       └── i18n/
│           └── commands/
│               ├── en.json
│               └── de.json
│
├── blob_connectors/
│   ├── base.py
│   ├── run.py
│   ├── requirements.txt
│   ├── .env.example
│   ├── README.md
│   └── connectors/
│       ├── telegram.py
│       ├── slack.py
│       ├── discord_connector.py
│       └── email.py
│
├── docs/
│   ├── documentation.md
│   └── proposals/
│       └── memory-system.md
│
├── tools/
│   └── piper/
│
├── package.json
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

---

## Core Systems

## Command Router

**Location:** `src-tauri/src/modules/command_router/`

The command router turns natural language into structured actions.

It should be strict enough to avoid accidental execution, but flexible enough to handle German/English mixed inputs and fuzzy user phrasing.

### Route priority

```text
1. Protected confirmations
2. Identity / profile queries
3. Utility commands
4. App/system launch commands
5. Browser and web navigation
6. Media-service commands
7. Screenshot / snip / vision commands
8. Transcript commands
9. Mini-game / companion interaction commands
10. LLM fallback chat
```

### Important rule

Normal chat must not be swallowed by the command router.

Examples that should go to chat/fallback:

- `how are you?`
- `what do you think about this?`
- `explain this to me`
- `can you summarize that?`
- `ich brauche kurz eine Einschätzung`

Examples that should go to action routing:

- `open spotify`
- `open downloads`
- `play Michael Jackson Thriller on YouTube`
- `scroll down`
- `click the first result`
- `shutdown`

If no confident action intent is detected, OpenBlob should return a regular LLM response and reset UI state correctly.

### Deterministic protected actions

Protected commands such as shutdown and restart must use confirmation.

Flow:

```text
User: shutdown
Blob: Are you sure?
User: yes
Blob: executes shutdown if confirmation is still valid
```

Cancellation:

```text
User: cancel
User: no
```

Timeout:

- pending protected actions expire automatically
- stale confirmations should not execute anything

---

## Capability Execution

Execution is split between newer core executor paths and legacy runtime modules.

Relevant areas:

- `src-tauri/src/core/executor/execute.rs`
- `src-tauri/src/core/legacy/app_open_runtime.rs`
- `src-tauri/src/core/legacy/browser_runtime.rs`
- `src-tauri/src/core/legacy/voice_command_executor.rs`
- `src-tauri/src/modules/system.rs`
- `src-tauri/src/modules/browser_automations.rs`

The execution layer is responsible for:

- launching apps
- opening folders
- opening URLs
- executing browser automation
- triggering media services
- controlling volume/media keys
- producing user-facing status messages
- emitting UI events
- writing memory events where appropriate

### Design direction

A future `CapabilityRegistry` / `AppLaunchCapability` layer should centralize:

- known app names
- aliases
- executable discovery
- protocol handlers
- web fallbacks
- service-specific routing
- platform-specific behavior

This avoids growing the legacy runtime with too many hardcoded special cases.

---

## Active Target Context

**Location:** `src-tauri/src/modules/session_memory.rs`

OpenBlob has a lightweight controlled session concept for follow-up commands.

The goal is to distinguish between:

- the app/window the user manually focused
- the app/browser/service OpenBlob intentionally opened or controlled

Manual OS focus is treated as passive context. OpenBlob's own actions create the active controlled context.

### Controlled session model

```ts
type ControlledTargetKind =
  | "browser"
  | "app"
  | "web-service"
  | "media-service";

type ControlledSession = {
  id: string;
  kind: ControlledTargetKind;
  appName?: string;
  service?: "youtube" | "spotify" | "steam" | "discord" | string;
  windowTitle?: string;
  processName?: string;
  url?: string;
  createdBy: "openblob";
  lastCommand?: string;
  lastUpdatedAt: number;
  isActiveControlledTarget: boolean;
};
```

### Expected behavior

```text
User: open YouTube
OpenBlob: opens YouTube and marks it as controlled browser/web-service

User: play Thriller
OpenBlob: routes follow-up to YouTube, not generic search
```

```text
User: open Spotify
OpenBlob: opens Spotify and marks Spotify as controlled media service

User: play Thriller
OpenBlob: routes follow-up to Spotify
```

```text
User: scroll down
OpenBlob: sends scroll to active controlled browser/service context
```

Explicit adoption:

```text
User: use this window
OpenBlob: current focused window becomes controlled target
```

---

## Browser Automation

**Location:** `src-tauri/src/modules/browser_automations.rs`

OpenBlob uses Chrome or Edge with Chrome DevTools Protocol (CDP) via remote debugging.

### Requirements

Chrome/Edge should run with:

```bash
chrome.exe --remote-debugging-port=9222
```

OpenBlob can attempt to launch browsers with the correct flags, but manual setup may still be needed in dev environments.

### Capabilities

- open URLs
- open search pages
- list tabs
- activate tabs
- close tabs
- navigate back/forward
- click visible text / first result
- type into inputs
- submit forms
- inspect page title / URL / visible links
- YouTube search-and-play helper

### YouTube play behavior

Expected behavior:

```text
play Michael Jackson Thriller on YouTube
spiele Michael Jackson Thriller auf YouTube
```

Should:

1. open or reuse YouTube
2. search the query
3. wait for results
4. choose a likely normal video result
5. avoid Shorts/channels/playlists unless explicitly requested
6. click/play the best available result

Matching should be tolerant/fuzzy:

```text
micheal jackson thriller
```

should still resolve to a reasonable top result such as `Michael Jackson - Thriller`.

### Generic browser follow-ups

Commands like these should use the active controlled browser context:

- `scroll down`
- `scroll up`
- `click first result`
- `click play`
- `go back`
- `search for ...`

If no active controlled browser context exists, OpenBlob should either ask or fall back safely.

---

## System and App Launch Runtime

OpenBlob supports app and system launch commands in English and German.

Examples:

- `open spotify`
- `open steam`
- `open discord`
- `open downloads`
- `open settings`
- `open explorer`
- `spiele Elden Ring auf Steam`
- `play Thriller on Spotify`

### Launch resolution layers

```text
1. direct known app aliases
2. Windows app discovery
3. protocol handlers
4. Steam game/library lookup
5. media-service routing
6. web fallback
7. Google search fallback
```

### Service-aware routing

Some commands require service-specific behavior.

Examples:

- `open spotify` should open Spotify normally
- `play Thriller on Spotify` should route through Spotify-specific search/play behavior
- `open steam` should open Steam normally
- `spiele Elden Ring auf Steam` should resolve via Steam game/app/store routing
- `open discord` should prefer installed app/protocol if available

---

## Screen and Vision

**Locations:**

- `src-tauri/src/modules/screen_capture.rs`
- snip overlay / snip panel frontend windows

OpenBlob can capture the screen or a selected region, then pass the image to a local vision model through Ollama.

### Modes

| Mode | Purpose |
| --- | --- |
| OCR | Extract visible text |
| Explain | Describe what is visible |
| Translate | Translate screen text |
| Search | Generate search query from visual content |
| Game assist | Detect game UI, quest logs, errors, or objectives |

### Current commands

- `screenshot`
- `take screenshot`
- `mach screenshot`
- `capture screen`
- `explain this`
- `translate this`
- `search this`

### Privacy rule

Screenshot/vision data should be processed locally by default. Any future external model path must be explicit and visible to the user.

---

## Voice Input

**Location:** `src-tauri/src/modules/voice.rs` and `src/windows/bubble/app.tsx`

The existing voice input flow is shortcut-driven.

Shortcut:

```text
ALT + M
```

This flow should remain stable and independent from the wake-word foundation.

### Current behavior

- user presses voice shortcut
- bubble enters voice/listening state
- speech recognition captures command
- recognized text is routed through the same command/chat pipeline

### Important regression guard

Wake-word work must not break:

- `ALT + M`
- existing voice UI state
- normal text chat fallback
- normal command execution

---

## Wake Word Foundation

**Location:** `src-tauri/src/modules/wake_word.rs`  
**Config location:** `src-tauri/src/modules/profile/companion_config.rs`  
**Dev UI:** `src/windows/bubble-dev/app.tsx`

The wake-word system is currently a safe local foundation. It does **not** implement real hotword detection yet.

Current goal:

> Provide settings, runtime status, and local microphone-test listening infrastructure that can later feed a real wake-word provider such as Porcupine or another local detector.

### Config fields

| Field | Purpose | Default expectation |
| --- | --- | --- |
| `wake_word_enabled` | whether wake-word system is enabled | false |
| `wake_word_phrase` | phrase shown/configured by user | default companion phrase |
| `wake_word_sensitivity` | normalized sensitivity value | 0.0 - 1.0 |
| `wake_word_provider` | selected provider | disabled / none / mic-test / mock / porcupine |

### Tauri commands

| Command | Purpose |
| --- | --- |
| `get_wake_word_settings` | read current config |
| `update_wake_word_settings` | save config |
| `start_wake_word_listener` | explicitly start listener |
| `stop_wake_word_listener` | stop listener |
| `get_wake_word_status` | inspect runtime status |

### Runtime states

| State | Meaning |
| --- | --- |
| `disabled` | wake word disabled |
| `stopped` | enabled but not listening |
| `starting` | listener is starting |
| `listening` | microphone-test listener is active |
| `no_input_device` | no microphone available |
| `permission_error` | microphone permission/access problem |
| `provider_missing` | selected provider is unavailable or not configured |
| `error` | generic runtime error |

### Current microphone-test runtime

The current foundation uses a local desktop audio input crate (`cpal`) to open the default microphone input device in explicit test modes only.

Supported test providers:

- `mic-test`
- `mock`

Provider behavior:

| Provider | Behavior |
| --- | --- |
| `disabled` / `none` | listener does not start |
| `mic-test` | opens local mic, counts chunks, estimates RMS input level |
| `mock` | currently allowed as local test mode |
| `porcupine` | recognized as future provider but not wired yet |

### Status fields

`get_wake_word_status` should return:

- enabled
- phrase
- provider
- sensitivity
- state
- status
- message
- listening
- detected
- provider_configured
- selected_input_device
- available_input_devices
- last_error
- last_started_at
- last_stopped_at
- last_audio_at
- audio_chunks_seen
- input_level

### Privacy and safety rules

- no cloud streaming
- no raw microphone audio storage
- no wake-word detection pretending
- no automatic listening on app startup by default
- no microphone opening when provider is missing
- no panic if no microphone exists
- no blocking Tauri startup
- no Tokio runtime creation inside async contexts
- no runtime audio files in git

### Manual wake-word foundation test

1. Start OpenBlob.
2. Open Dev / Settings window.
3. Enable wake word.
4. Set provider to `mic-test`.
5. Press Start Listener.
6. Confirm state becomes `listening`.
7. Speak or make noise.
8. Confirm `audio_chunks_seen`, `last_audio_at`, or `input_level` changes.
9. Press Stop Listener.
10. Confirm state becomes `stopped`.
11. Disable wake word.
12. Confirm listener does not run.
13. Confirm `ALT + M` still works.

### Remaining wake-word limitations

- no real Porcupine integration yet
- no custom trained phrase model yet
- no automatic wake-to-command flow yet
- no wake confirmation sound/visual yet
- no permission onboarding UX yet
- no device selector persistence yet

---

## Transcript System

**Locations:**

- `src-tauri/src/modules/transcript/`
- `src/windows/transcript/`

OpenBlob includes a local transcript pipeline optimized for system audio.

### Current transcript flow

```text
System audio
   │
   ▼
WASAPI loopback capture
   │
   ▼
Chunk buffering and temporary WAV writing
   │
   ▼
Local Whisper CLI transcription
   │
   ▼
Live transcript window
   │
   ▼
AI post-processing
   ├─ faithful transcript
   ├─ speaker-style grouped blocks
   ├─ summary
   └─ action items
```

### Core files

| File | Purpose |
| --- | --- |
| `audio_capture.rs` | Windows loopback capture and mono conversion |
| `runtime.rs` | worker lifecycle and transcription loop |
| `transcript_engine.rs` | WAV writing and Whisper CLI execution |
| `session.rs` | active transcript sessions and segment state |
| `processor.rs` | AI cleanup, summary, speaker-style grouping |
| `transcript_store.rs` | persistence of transcript output |

### Commands

- `start transcript`
- `stop transcript`
- `open transcript`
- `process transcript`
- `save transcript`

### Current limitations

- optimized for system audio, not microphone input
- true acoustic diarization is not implemented yet
- German quality can be weaker than English depending on model/config
- transcript-to-memory extraction is planned but not complete

---

## Memory System

**Locations:**

- `src-tauri/src/modules/memory/`
- `src-tauri/src/modules/session_memory.rs`
- `docs/proposals/memory-system.md`

OpenBlob uses a local-first layered memory system.

### Memory design principle

> Memory should improve the experience but must never block core execution.

If memory loading, saving, embedding, or retrieval fails, OpenBlob should still execute the requested command where possible.

### Current memory layers

| Layer | File / module | Purpose |
| --- | --- | --- |
| Episodic memory | `episodic_memory.jsonl` | timestamped event log |
| Semantic memory | `semantic_memory.json` | stable facts and preferences |
| Session memory | `session_memory.rs` | runtime context and controlled sessions |
| Personality | `personality_state.json` | energy, affection, curiosity, playfulness |
| Bonding | `bonding_state.json` | relationship level, trust, shared sessions |
| Profile/config | companion/user config files | identity and preferences |

### Episodic memory

Episodic memory records events such as commands, external-channel messages, outcomes, and summaries.

Example:

```json
{
  "version": 1,
  "id": "ep_1234567890",
  "timestamp": "2026-05-06T12:00:00Z",
  "kind": "external_command",
  "app_name": "telegram",
  "context_domain": "external",
  "user_input": "open spotify",
  "summary": "Opened Spotify from Telegram connector",
  "outcome": "success",
  "importance": 0.6
}
```

### Semantic memory

Semantic memory contains facts that should persist beyond a single session:

- preferred language
- known apps
- common user topics
- communication style
- recurring preferences
- owner-related facts
- external channel context

### Session memory

Session memory is runtime-only or short-lived state used during command execution.

Examples:

- last command
- last browser target
- active controlled session
- pending protected action
- last media service
- current context hints

### Memory embedding notes

Embedding-backed memory retrieval is experimental. If the embedding model is missing, OpenBlob should log a warning and continue.

Example expected warning:

```text
Memory embedding skipped: Embedding model not available. Pull with: ollama pull nomic-embed-text
```

This should not break chat or deterministic commands.

### Planned memory evolution

- SQLite-backed event store
- semantic fact extraction
- query-aware retrieval
- embedding-backed retrieval
- memory inspector UI
- transcript-to-memory extraction
- per-channel memory policy
- privacy tiers for memory records

---

## Companion Identity and Profile

**Locations:**

- `src-tauri/src/modules/profile/companion_config.rs`
- `src-tauri/src/modules/profile/user_profile.rs`
- `src-tauri/src/modules/profile/onboarding_state.rs`

OpenBlob stores companion and owner identity locally.

### Key fields

| Field | Purpose |
| --- | --- |
| `blob_name` | display name of the companion |
| `owner_name` | user's name |
| `preferred_language` | language preference |
| `wake_word_enabled` | wake-word setting |
| `wake_word_phrase` | configured phrase |
| `wake_word_sensitivity` | wake-word sensitivity |
| `wake_word_provider` | provider selection |

Identity values should propagate to:

- desktop UI
- companion responses
- dev/settings UI
- Blob Connectors
- memory context

---

## Text-to-Speech

**Location:** `src-tauri/src/modules/tts/`

OpenBlob has local TTS support.

| Provider | Status | Notes |
| --- | --- | --- |
| Piper | active/local | ONNX-based local voice synthesis |
| Kokoro | experimental | alternative local TTS path |

Voice output should be optional and user-controlled.

---

# Frontend Windows

Each frontend surface is a separate React window.

| Window | Path | Purpose |
| --- | --- | --- |
| Bubble | `src/windows/bubble/` | main companion UI, chat, voice, subtitles |
| Bubble Dev | `src/windows/bubble-dev/` | settings, diagnostics, wake-word controls |
| Quick Menu | `src/windows/quick-menu/` | fast command menu |
| Transcript | `src/windows/transcript/` | live transcript and processed transcript view |
| Snip Overlay | `src/windows/snip-overlay/` | screenshot region selection |
| Snip Panel | `src/windows/snip-panel/` | screenshot analysis results |
| Timer Overlay | `src/windows/timer-overlay/` | timer/countdown UI |

## Bubble UI state expectations

Normal lifecycle:

```text
idle → thinking → speaking/responding → idle
```

Error lifecycle:

```text
idle → thinking → error → idle
```

Important regression guard:

- normal chat should not get stuck in `thinking`
- action commands should return a user-facing result
- LLM fallback should reset state after response
- command router failures should surface errors cleanly

---

# Tauri Bridge Layer

**Location:** `src-tauri/src/lib.rs`

The Tauri bridge registers commands and connects frontend windows to Rust modules.

Responsibilities:

- window creation and management
- global shortcut registration
- command registration
- event emission
- startup initialization
- command server startup
- shared app handle access

## External command server

OpenBlob runs a local HTTP command server on:

```text
http://127.0.0.1:7842
```

Example request:

```http
POST /command
Content-Type: application/json

{
  "input": "open spotify",
  "channel": "telegram"
}
```

The server should:

1. accept local connector requests
2. route them through the normal command pipeline
3. write memory events
4. return a channel-safe response

It should only listen on localhost.

---

# Configuration and Local Data

OpenBlob stores persistent local data in the user's app data directory.

Typical categories:

| Category | Example path | Purpose |
| --- | --- | --- |
| Companion config | `config/companion_config.json` | blob name, language, wake-word settings |
| User profile | `config/user_profile.json` | owner name and profile hints |
| Onboarding | `config/onboarding_state.json` | onboarding state |
| Episodic memory | `memory/episodic_memory.jsonl` | event history |
| Semantic memory | `memory/semantic_memory.json` | stable facts |
| Personality | `memory/personality_state.json` | mood/personality values |
| Bonding | `memory/bonding_state.json` | relationship state |
| Transcript output | `openblob-data/transcripts/` | transcript sessions |

Runtime-generated data should not be committed to git.

---

# AI and Model Integration

OpenBlob uses Ollama for local AI inference.

## Common models

| Model | Purpose |
| --- | --- |
| `llama3.1:8b` | default text/chat/fallback |
| `gemma3` | vision/screenshot analysis |
| `qwen2.5vl:7b` | optional stronger vision |
| `nomic-embed-text` | optional memory embeddings |

## AI usage policy

AI should be used when:

- deterministic routing cannot confidently handle the input
- user asks a normal conversational question
- screenshot/vision explanation is needed
- transcript cleanup/summary is requested
- semantic memory extraction is needed

AI should not be used to guess dangerous actions.

Protected actions must stay deterministic and confirmation-based.

---

# Global Shortcuts

| Shortcut | Action |
| --- | --- |
| `CTRL + SPACE` | toggle companion UI |
| `ALT + M` | voice input shortcut |
| `CTRL + ALT + S` | screenshot/snip mode |

Shortcut behavior should stay stable across routing, wake-word, and UI refactors.

---

# Blob Connectors

**Location:** `blob_connectors/`

Blob Connectors is a Python layer that allows OpenBlob to be reached through external channels.

Supported channels:

- Telegram
- Discord
- Slack
- Email

## Architecture

```text
External channel
      │
      ▼
Connector adapter
      │
      ▼
Normalized Message object
      │
      ├─ OpenBlob running
      │      ▼
      │   localhost:7842 command server
      │
      └─ OpenBlob unavailable
             ▼
          Ollama fallback
```

## Quickstart

```bash
cd blob_connectors
pip install -r requirements.txt
cp .env.example .env
python run.py
```

Only connectors with configured tokens start.

## Environment variables

```dotenv
# Telegram
TELEGRAM_BOT_TOKEN=

# Discord
DISCORD_BOT_TOKEN=

# Slack
SLACK_BOT_TOKEN=
SLACK_APP_TOKEN=

# Email
EMAIL_ADDRESS=
EMAIL_PASSWORD=
IMAP_HOST=imap.gmail.com
SMTP_HOST=smtp.gmail.com
IMAP_PORT=993
SMTP_PORT=587
```

## Message model

```python
@dataclass
class Message:
    session_id: str
    user_id: str
    text: str
    channel: str
    username: str
    message_id: str
    timestamp: datetime
    attachments: list[Attachment]
    raw: Any
```

## Memory integration

Connectors read local OpenBlob memory and identity files to build context:

- companion name
- owner name
- semantic memory
- episodic memory
- personality state
- bonding state
- current channel

This lets the blob keep a consistent identity across desktop and external channels.

---

# Command Reference

## Browser and navigation

| Command | Description |
| --- | --- |
| `google <query>` | Google search |
| `google nach <query>` | German Google search |
| `youtube <query>` | YouTube search |
| `open <url>` | open website |
| `open youtube` | open YouTube |
| `go back` / `zurück` | browser back |
| `forward` | browser forward |
| `scroll down` / `scroll up` | scroll active controlled browser |
| `click first result` | click first visible result |
| `type <text>` | type into active input |
| `submit` | press Enter |

## Media and streaming

| Command | Description |
| --- | --- |
| `play <title> on YouTube` | search and play YouTube result |
| `spiele <title> auf YouTube` | German YouTube play |
| `play <title> on Spotify` | route to Spotify |
| `play <game> on Steam` | route to Steam |
| `next video` | next media item |
| `forward <seconds>` | seek forward |
| `rewind` | seek back |

## System control

| Command | Description |
| --- | --- |
| `open <app>` | launch known app |
| `open downloads` | open Downloads folder |
| `open settings` | open Windows Settings |
| `open explorer` | open File Explorer |
| `lock screen` | lock Windows session |
| `shutdown` | protected shutdown flow |
| `restart` | protected restart flow |
| `yes` | confirm pending protected action |
| `no` / `cancel` | cancel pending protected action |
| `volume up/down` | volume control |
| `mute` / `unmute` | audio toggle |
| `next track` | media next |
| `previous track` | media previous |

## Screenshot and vision

| Command | Description |
| --- | --- |
| `screenshot` | start snip mode |
| `take screenshot` | capture screen |
| `mach screenshot` | German screenshot |
| `explain this` | analyze screenshot/context |
| `translate this` | translate screen text |
| `search this` | generate search query |

## Transcript

| Command | Description |
| --- | --- |
| `start transcript` | start system-audio transcript |
| `stop transcript` | stop transcript |
| `open transcript` | open transcript window |
| `process transcript` | generate AI post-processing |
| `save transcript` | save session |

## Voice and wake-word

| Command / control | Description |
| --- | --- |
| `ALT + M` | manual voice input |
| Dev UI wake toggle | enable/disable wake-word foundation |
| Provider `mic-test` | local microphone test mode |
| Start Listener | explicit mic-test listener start |
| Stop Listener | stop mic-test listener |

---

# Testing and Validation

## Recommended pre-PR checks

From repository root:

```bash
npm run build
```

From `src-tauri`:

```bash
cargo fmt
cargo check -j 1
cargo test --lib -j 1 -- --test-threads=1
```

## Manual smoke tests

### Normal chat regression

```text
how are you?
what do you think about this?
explain this to me
```

Expected:

- regular LLM response
- no action routing
- blob state returns to idle

### App launch

```text
open spotify
open steam
open discord
open downloads
open settings
```

Expected:

- app/folder/settings opens
- user-facing response appears
- controlled context updated when relevant

### YouTube

```text
open YouTube
play Michael Jackson Thriller
play Micheal Jackson Thriller on YouTube
```

Expected:

- YouTube opens
- follow-up uses YouTube context
- search-and-play attempts a normal video result

### Wake-word mic-test

```text
Enable wake word in Dev UI
Set provider to mic-test
Start listener
Speak or make noise
Stop listener
```

## Wake Word

Wake-word support is optional, local-first, and disabled unless the user enables it and starts the listener. `ALT + M` remains the manual voice shortcut.

Providers:

- `mic-test` starts local microphone capture only to verify chunks, timestamps, and input level.
- `mock` is development-only and can emit `wake-word-detected` from loud local input.
- `local-openwakeword` / `local-wakeword` are the free local provider path. They discover and validate openWakeWord-style ONNX bundles under `%APPDATA%/OpenBlob/voice/models/wake-word/`, then the repo-local `voice/models/wake-word/` fallback.

Bundle layout:

```text
voice/models/wake-word/
  openwakeword/
    manifest.json
    melspectrogram.onnx
    embedding.onnx
    hey-openblob.onnx
```

The current local provider validates the manifest, checks missing files, normalizes microphone audio to mono 16 kHz fixed windows, and reports `runtime_missing` until an ONNX inference backend is linked. It does not call cloud services, does not require paid keys, does not auto-download models, and does not store raw microphone audio.

Wake-to-voice is controlled separately by `wake_word_auto_listen_enabled`. When enabled, the frontend reacts to `wake-word-detected` and starts the existing voice input flow; the event itself never executes commands directly.

---

## Known Issues

- state becomes `listening`
- `audio_chunks_seen` increases
- `last_audio_at` updates
- `input_level` changes
- stop returns state to `stopped`
- `ALT + M` still works

### Transcript

```text
start transcript
play system audio
stop transcript
process transcript
```

Expected:

- transcript window receives segments
- processing creates summary/action-item output

---

# Tech Stack

OpenBlob is built as a local-first Windows desktop app with a Rust backend, React frontend, local AI models, local audio pipelines, and optional external connector processes.

| Layer | Technology | Notes |
| --- | --- | --- |
| Frontend | React + TypeScript + Vite | Multi-window UI for bubble, dev/settings, quick menu, transcript, snip, and overlays |
| Desktop shell | Tauri v2 | Native Windows desktop shell, window management, commands, events, global shortcuts |
| Backend runtime | Rust | Command routing, app/system control, browser automation, memory, transcript, voice, wake-word runtime |
| Local command server | axum on `127.0.0.1:7842` | Used by Blob Connectors to send commands into the same Rust pipeline |
| AI inference | Ollama | Local model runtime for chat fallback, explanation, context reasoning, and vision prompts |
| Text model | `llama3.1:8b` by default | Can be swapped as the model-routing layer evolves |
| Vision models | `gemma3`, `qwen2.5vl:7b`, LLaVA-style models | Used for screenshot and snip analysis depending on local model availability |
| Embeddings | `nomic-embed-text` optional | Used/planned for semantic retrieval; missing embeddings must not break chat |
| Motion / UI animation | Framer Motion | Blob movement, transitions, and expressive UI states |
| Icons / UI assets | React components + local assets | Used across bubble/dev windows and overlays |
| Text-to-speech | Piper ONNX + Kokoro experimental | Local voice output, provider selection still evolving |
| Manual voice input | Browser/Web Speech API path + existing `ALT + M` flow | Must remain untouched while wake-word work evolves |
| Wake-word foundation | Rust module + `cpal` mic test runtime | Local mic-test/mock provider only; no real detector yet; no cloud streaming; no raw audio storage |
| Audio input | `cpal` | Device availability, default input device, local RMS/input-level metrics for dev UI |
| Transcript audio capture | Windows WASAPI loopback | Captures system audio for local transcript sessions |
| Speech-to-text | Local Whisper CLI | Used by transcript pipeline for audio chunks |
| Browser automation | Chrome DevTools Protocol (CDP) | Chrome/Edge remote debugging for tab/page interaction and YouTube helpers |
| App/system launch | Windows APIs, protocols, shell commands | App discovery, protocol routing, deterministic Windows actions |
| Storage | Local JSON/JSONL files under `%APPDATA%\OpenBlob\` | Companion config, user profile, memory, personality, bonding, transcripts |
| Planned/active memory storage | JSON/JSONL now, SQLite proposal/foundation evolving | Memory must never block command execution |
| External connectors | Python 3.11+ | Telegram, Discord, Slack, Email bridge layer |
| Connector networking | `aiohttp` + platform SDKs | Sends commands to localhost OpenBlob server or falls back to Ollama |
| Testing | Rust unit tests + frontend build checks | `cargo test --lib -j 1 -- --test-threads=1`, `npm run build` when deps are available |
| Primary platform | Windows 10 / 11 | Current target; cross-platform ideas are future work |

## Runtime requirements

| Requirement | Needed for | Notes |
| --- | --- | --- |
| Ollama running on `127.0.0.1:11434` | Local AI responses and vision/model fallback | Pull required models before testing AI flows |
| Chrome or Edge with CDP port `9222` | Browser automation | OpenBlob can attempt launch, but manual startup may be needed |
| Local microphone permission | Wake-word mic-test runtime | Only starts when explicitly triggered; no startup mic capture |
| Whisper CLI configured | Transcript system | Used for local system-audio transcription |
| Python env for `blob_connectors/` | Telegram/Discord/Slack/Email | Optional; desktop app works without connectors |

## Important stack boundaries

- **Wake word is foundation-only right now:** `mic-test`/`mock` can listen locally and report chunks/input level, but no real wake-word detection is implemented yet.
- **Transcript and wake word are different audio paths:** transcript uses system audio loopback; wake-word mic-test uses microphone input through `cpal`.
- **No raw mic audio is stored:** the wake-word foundation only tracks lightweight runtime metrics like chunk count, last audio timestamp, and RMS input level.
- **Deterministic actions should stay model-independent:** app/system/browser commands should not depend on LLM success when a safe deterministic route exists.
- **Normal chat fallback must stay intact:** no confident action intent should mean regular LLM response, not swallowed router output.

---

# Contribution Guide

Contributions are welcome from developers, designers, testers, writers, and AI experimenters.

## Good first contribution areas

- command aliases
- German/English command improvements
- UI polish
- docs cleanup
- bug reproduction notes
- smoke test scripts
- memory inspector UI
- wake-word status UI improvements
- connector improvements

## Before larger changes

Open an issue or discussion for:

- architecture refactors
- new provider integrations
- new persistent data formats
- new connector types
- security-sensitive features
- microphone/voice pipeline changes
- protected system actions

## PR checklist

- explain user-facing behavior
- list changed files/modules
- mention safety/privacy implications
- run Rust checks
- run frontend build when possible
- include manual smoke-test notes
- avoid committing local runtime data

---

# Known Issues

| Area | Status |
| --- | --- |
| Normal chat fallback | Must be guarded after routing changes |
| Snip capture | Region capture may still be unreliable in some cases |
| Browser automation | CDP and UI timing can be fragile |
| YouTube play | Fuzzy search-and-play needs real desktop smoke tests |
| Spotify/Steam routing | Depends on installed apps/protocol handlers |
| Memory embeddings | Optional model may be missing; should not block execution |
| Wake word | Foundation only; no real detector yet |
| Mic permissions | Needs better UX and device selection |
| Transcript diarization | Speaker grouping is AI-estimated, not true diarization |
| Settings UI | Growing, but not final product UX |
| Error handling | Still inconsistent across modules |
| Connectors | Session persistence and permissions need more work |

---

# Roadmap

## Phase 1 — Stabilization

- [ ] keep normal chat fallback stable
- [ ] stabilize command router confidence thresholds
- [ ] reduce legacy launch routing complexity
- [ ] add capability registry for apps/protocols/fallbacks
- [ ] improve app discovery and launch reliability
- [ ] improve YouTube search-and-play reliability
- [ ] improve browser automation timing and consent handling
- [ ] improve protected-action UI feedback
- [ ] improve Dev Window command/debug visibility

## Phase 2 — Voice and Wake Word

- [x] wake-word settings foundation
- [x] local microphone-test runtime foundation
- [ ] microphone permission onboarding
- [ ] input device selector
- [ ] real local wake-word provider integration
- [ ] wake-to-command flow
- [ ] wake feedback sound/animation
- [ ] keep `ALT + M` as fallback manual voice path

## Phase 3 — Memory

- [ ] SQLite-backed memory event store
- [ ] semantic fact extraction
- [ ] embedding-backed retrieval
- [ ] memory inspector UI
- [ ] transcript-to-memory extraction
- [ ] per-channel memory policies
- [ ] privacy tiers for memory

## Phase 4 — Transcript and Meeting Intelligence

- [ ] microphone + system-audio hybrid transcript mode
- [ ] better Whisper model config
- [ ] better German transcript quality
- [ ] real diarization exploration
- [ ] meeting-note templates
- [ ] action-item extraction to tasks
- [ ] transcript search and memory integration

## Phase 5 — Platform and Connectors

- [ ] plugin/capability registry
- [ ] community skill packs
- [ ] WhatsApp connector
- [ ] Matrix / Element connector
- [ ] Telegram voice message support
- [ ] Google Calendar integration
- [ ] per-channel permission system
- [ ] persistent connector sessions

## Phase 6 — UX and Companion Personality

- [ ] onboarding flow
- [ ] richer blob animations
- [ ] more emotional states
- [ ] personality influencing tone safely
- [ ] bonding state visible in Dev UI
- [ ] mini-games beyond Hide & Seek
- [ ] polished glass/motion UI pass

---

# License

OpenBlob is licensed under the [MIT License](../LICENSE).

---

<div align="center">

**OpenBlob is meant to grow.**

Star the repo · Open issues · Suggest features · Contribute code

</div>
