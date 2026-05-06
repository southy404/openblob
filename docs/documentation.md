# OpenBlob — Developer Documentation

> **Local-first AI desktop companion for Windows**  
> Built with Tauri · React · Rust · Ollama

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Getting Started](#getting-started)
4. [Project Structure](#project-structure)
5. [Core Systems](#core-systems)
   - [Command Router](#command-router)
   - [Browser Automation](#browser-automation)
   - [Screen & Vision](#screen--vision)
   - [Transcript System](#transcript-system)
   - [Memory System](#memory-system)
   - [Companion Identity](#companion-identity)
   - [Text-to-Speech](#text-to-speech)
6. [Command Reference](#command-reference)
7. [Frontend Windows](#frontend-windows)
8. [Tauri Bridge Layer](#tauri-bridge-layer)
9. [AI & Model Integration](#ai--model-integration)
10. [Configuration & Profiles](#configuration--profiles)
11. [Global Shortcuts](#global-shortcuts)
12. [Blob Connectors](#blob-connectors)
13. [Contributing](#contributing)
14. [Known Issues](#known-issues)
15. [Roadmap](#roadmap)
16. [License](#license)

---

## Overview

OpenBlob is an **open-source, local-first desktop companion** for Windows 10/11.

It goes beyond a simple chatbot — it acts as an **operating-layer assistant** that can:

- execute desktop commands directly
- execute deterministic system commands such as opening Downloads, Settings, Explorer, locking the screen, and handling protected power actions
- control your browser via remote debugging
- understand your screen through vision models
- remember context across sessions
- speak to you using TTS
- grow with you through a configurable companion identity
- be reached from anywhere via Telegram, Discord, Slack, and Email

**Core design principle:**

> Deterministic first. AI second.

Whenever a command can be executed locally without a model, it is. AI is used as a capability layer — not the whole product.

Protected system actions such as shutdown and restart use an explicit confirmation flow with timeout and cancellation support.

---

## Architecture

OpenBlob is split into three major layers:

```
┌─────────────────────────────────────────────────┐
│                   UI Layer (React)               │
│  bubble · dev-window · quick-menu · snip-panel  │
│  transcript · snip-overlay · timer-overlay      │
└────────────────────┬────────────────────────────┘
                     │ invoke / emit / listen
┌────────────────────▼────────────────────────────┐
│              Bridge Layer (Tauri v2)             │
│  Window management · Shortcuts · Event system   │
└────────────────────┬────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────┐
│             Runtime Layer (Rust)                 │
│  Command routing · Browser automation           │
│  Screen capture · Transcript · Memory · TTS     │
│  External command server (localhost:7842)        │
└─────────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────┐
│          Blob Connectors Layer (Python)          │
│  Telegram · Discord · Slack · Email             │
│  Memory bridge · Ollama fallback                │
└─────────────────────────────────────────────────┘
```

### Data Flow

```
User Input (text / voice / external channel)
       │
       ▼
Command Router
(normalize → explicit command checks → intent scoring)
       │
       ▼
CompanionAction
       │
       ▼
Capability Mapper
       │
       ▼
Capability Executor
       │
  ┌────┴───────────────┐
  │                    │
Deterministic Action   Ollama Fallback
(system/browser/media) (ask / explain / translate / vision)
  │                    │
  └────┬───────────────┘
       │
Subtitle Output + TTS + Channel Response
```

---

## Getting Started

### Requirements

| Dependency     | Version  | Notes                            |
| -------------- | -------- | -------------------------------- |
| Windows        | 10 or 11 | Primary platform                 |
| Node.js        | ≥ 18     | [nodejs.org](https://nodejs.org) |
| Rust + Cargo   | stable   | [rustup.rs](https://rustup.rs)   |
| Tauri CLI      | v2       | via `cargo install tauri-cli`    |
| Ollama         | latest   | [ollama.com](https://ollama.com) |
| Chrome or Edge | any      | Required for browser automation  |
| Python         | ≥ 3.11   | Required for Blob Connectors     |

### Install & Run

```bash
# 1. Clone the repository
git clone https://github.com/your-org/openblob.git
cd openblob

# 2. Install frontend dependencies
npm install

# 3. Start Ollama and pull required models
ollama serve
ollama pull llama3.1:8b        # default text model
ollama pull gemma3             # default vision model

# Optional: better vision quality
ollama pull qwen2.5vl:7b

# 4. Run in development mode
npm run tauri dev

# 5. Build frontend only (no Tauri shell)
npm run build
```

### First Launch Notes

- On first run, Windows Defender or SmartScreen may show warnings — this is expected due to system-level access (keyboard hooks, screen capture, browser debugging).
- Allow the app through your antivirus if you trust it.
- Chrome/Edge must have the remote debugging port `9222` available for browser automation.

---

## Project Structure

```
openblob/
├── src/                        # React frontend
│   ├── windows/
│   │   ├── bubble/             # Main companion interaction surface
│   │   ├── bubble-dev/         # Dev window (settings + debug)
│   │   ├── quick-menu/         # Fast-access command menu
│   │   ├── transcript/         # Transcript window + AI post-processing
│   │   ├── snip-overlay/       # Screenshot capture overlay
│   │   ├── snip-panel/         # Screenshot analysis panel
│   │   └── timer-overlay/      # Timer utility overlay
│   └── main.tsx
│
├── src-tauri/
│   └── src/
│       ├── main.rs             # Tauri entry point + command registration
│       ├── modules/
│       │   ├── command_router/ # Core command parsing + routing
│       │   │   ├── mod.rs
│       │   │   ├── intents.rs  # Intent detection
│       │   │   ├── matchers.rs # Command matching logic
│       │   │   ├── fuzzy.rs    # Fuzzy matching
│       │   │   ├── parser.rs   # Input parsing
│       │   │   ├── types.rs    # Shared types
│       │   │   └── normalize.rs
│       │   ├── companion/
│       │   │   ├── personality.rs
│       │   │   └── bonding.rs
│       │   ├── memory/
│       │   │   ├── episodic_memory.rs
│       │   │   └── semantic_memory.rs
│       │   ├── profile/
│       │   │   ├── companion_config.rs
│       │   │   ├── user_profile.rs
│       │   │   └── onboarding_state.rs
│       │   ├── tts/
│       │   │   ├── manager.rs
│       │   │   ├── piper.rs
│       │   │   └── kokoro.rs
│       │   ├── storage/
│       │   │   ├── json_store.rs
│       │   │   └── paths.rs
│       │   ├── browser_automations.rs
│       │   ├── screen_capture.rs
│       │   ├── transcript/
│       │   ├── session_memory.rs
│       │   ├── context.rs
│       │   ├── context_resolver.rs
│       │   ├── system.rs
│       │   ├── voice.rs
│       │   └── steam_games.rs
│       └── i18n/
│           └── commands/
│               ├── en.json
│               └── de.json
│
├── blob_connectors/            # External channel connectors (Python)
│   ├── base.py                 # Message model + BlobConnector interface
│   ├── run.py                  # Connector runner + AI handler
│   ├── requirements.txt
│   ├── .env.example
│   ├── README.md
│   └── connectors/
│       ├── telegram.py
│       ├── slack.py
│       ├── discord_connector.py
│       └── email.py
│
├── docs/                       # Architecture + design docs
├── tools/piper/                # Bundled TTS binary
└── .github/                    # CI, issue templates, PR template
```

---

## Core Systems

### Command Router

The command router is the brain of OpenBlob. It processes every user input and decides what to do with it.

**Location:** `src-tauri/src/modules/command_router/`

#### Route Priority

```
1. Identity queries        → deterministic (name, owner)
2. Utility commands        → time, date, weather, timer
3. Browser commands        → open, search, navigate, click
4. System / app commands   → launch, volume, media, deterministic OS actions
5. Snip / vision commands  → screenshot, explain, translate
6. Streaming commands      → Netflix, YouTube playback
7. Ollama fallback         → ask, explain, translate (model)
```

#### Deterministic system command path

High-priority operating-system commands are handled deterministically before fuzzy fallback logic when possible.

Examples include:

- opening Downloads
- opening Windows Settings
- opening File Explorer
- locking the screen
- protected power commands such as shutdown and restart

This prevents critical commands from being misclassified as generic app-launch, screenshot, or suggestion-followup actions.

#### Protected actions

Destructive or disruptive actions such as shutdown and restart are never executed immediately from natural language alone.

Instead, OpenBlob uses a guarded pending-action flow:

1. user requests a protected action
2. blob stores a short-lived pending action
3. blob asks for confirmation
4. user must explicitly confirm with a short follow-up such as `yes`
5. the pending action expires automatically after a short timeout
6. the action can also be cancelled explicitly with `no` / `cancel`

This keeps the system deterministic while reducing the risk of accidental execution.

#### Key modules

| File           | Purpose                                              |
| -------------- | ---------------------------------------------------- |
| `intents.rs`   | Maps normalized input to recognized intents          |
| `matchers.rs`  | Pattern matching per command group                   |
| `fuzzy.rs`     | Fuzzy string similarity for tolerant matching        |
| `parser.rs`    | Tokenizes and classifies input                       |
| `normalize.rs` | Lowercasing, trimming, language normalization        |
| `types.rs`     | `CompanionAction`, `IntentKind`, `IntentScore` types |

#### Extending the router

To add a new command:

1. Define the intent in `intents.rs`
2. Add matchers in `matchers.rs`
3. Add i18n keys to `i18n/commands/en.json` and `de.json`
4. Wire the handler in `mod.rs`

#### External command server

OpenBlob exposes a local HTTP server on `localhost:7842` that accepts commands from external sources (Blob Connectors). Commands go through the same pipeline as voice/text input from the UI.

```
POST http://localhost:7842/command
Content-Type: application/json

{ "input": "open spotify", "channel": "telegram" }
```

The server is started automatically when OpenBlob launches and only listens on localhost. External connectors fall back to Ollama automatically if this server is not available.

---

### Browser Automation

**Location:** `src-tauri/src/modules/browser_automations.rs`

OpenBlob controls Chrome or Edge via the **Chrome DevTools Protocol (CDP)** on port `9222`.

#### Capabilities

- List and close tabs
- Open URLs in active or new tab
- Navigate forward/back
- Click elements by visible text or selector
- Type into input fields
- Submit forms
- Inspect current page context (title, URL, visible links)
- YouTube search and play helpers via keyboard simulation

#### Requirements

```bash
# Chrome must be started with remote debugging enabled
chrome.exe --remote-debugging-port=9222
```

> OpenBlob attempts to launch the browser automatically with the correct flags. If it fails, start Chrome/Edge manually with the flag above.

---

### Screen & Vision

**Location:** `src-tauri/src/modules/screen_capture.rs` · `snip_session.rs`

#### Analysis modes

| Mode        | What it does                                             |
| ----------- | -------------------------------------------------------- |
| OCR         | Extract visible text from screen                         |
| Translate   | Detect language + translate on-screen text               |
| Explain     | Describe what is shown                                   |
| Search      | Generate a useful search query from the content          |
| Game assist | Detect game UI / quest text / errors and suggest actions |

---

### Transcript System

**Location:** `src-tauri/src/modules/transcript/` · `src/windows/transcript/`

The transcript system adds a **continuous listening and transcription pipeline** to OpenBlob using Windows WASAPI loopback capture and a local Whisper CLI.

#### End-to-end flow

```
System audio (loopback)
        │ WASAPI capture
        │ Mono chunk buffering
        │ Temporary WAV write
        │ Local Whisper CLI
        ▼
Transcript segments
        ├── Live transcript window
        ├── Session persistence
        └── AI post-processing
             ├── Faithful transcript
             ├── Speaker-style blocks
             ├── Summary
             └── Action items
```

#### Core files

| File                   | Purpose                                                      |
| ---------------------- | ------------------------------------------------------------ |
| `audio_capture.rs`     | Windows loopback capture, mono conversion, chunk buffering   |
| `runtime.rs`           | Worker loop, chunk merge, transcription execution            |
| `transcript_engine.rs` | WAV writing + Whisper CLI execution                          |
| `session.rs`           | Active transcript session lifecycle and segment append       |
| `processor.rs`         | AI-based cleanup, manuscript, speaker grouping, action items |
| `transcript_store.rs`  | Markdown + JSON persistence for finished sessions            |

---

### Memory System

**Location:** `src-tauri/src/modules/memory/`

OpenBlob uses a layered memory architecture that persists locally as JSON files. All memory files are also read by the Blob Connectors layer to provide context to external channel conversations.

Long-term retrieval and SQLite-backed memory are being planned in [docs/proposals/memory-system.md](./proposals/memory-system.md).

#### Memory layers

| Layer       | File                     | Purpose                                                   |
| ----------- | ------------------------ | --------------------------------------------------------- |
| Episodic    | `episodic_memory.jsonl`  | Timestamped log of past interactions and events           |
| Semantic    | `semantic_memory.json`   | Recurring facts: known apps, topics, communication style  |
| Session     | `session_memory.rs`      | Runtime context: last command, last search, browser state |
| Personality | `personality_state.json` | Energy, curiosity, affection, playfulness, focus bias     |
| Bonding     | `bonding_state.json`     | Relationship level, trust score, shared session count     |

#### Design rule

> Memory must never block execution.

Memory is loaded and saved asynchronously. If a memory operation fails, core commands still execute.

#### Episodic memory entry structure

Each entry in `episodic_memory.jsonl` contains:

```json
{
  "version": 1,
  "id": "ep_1234567890",
  "timestamp": "2025-04-19T22:00:00Z",
  "kind": "external_command",
  "app_name": "telegram",
  "context_domain": "external",
  "user_input": "open spotify",
  "summary": "OpenApp { target: spotify }",
  "outcome": "success",
  "importance": 0.6
}
```

The `app_name` field reflects the channel (telegram, discord, slack, email, or desktop) so the blob knows where interactions originated.

---

### Companion Identity

**Location:** `src-tauri/src/modules/profile/companion_config.rs`

#### Identity fields

| Field                | Default | Notes                              |
| -------------------- | ------- | ---------------------------------- |
| `blob_name`          | "Blob"  | The companion's display name       |
| `owner_name`         | ""      | Your name — used in self-reference |
| `preferred_language` | "en"    | Controls i18n command parsing      |

Identity is editable in the **Dev Window**. Both the desktop UI and Blob Connectors read these values — the blob will introduce itself by name and address you by name across all channels.

---

### Text-to-Speech

**Location:** `src-tauri/src/modules/tts/`

| Engine | Notes                                  |
| ------ | -------------------------------------- |
| Piper  | Fast, local ONNX-based voice synthesis |
| Kokoro | Alternative engine (experimental)      |

---

## Command Reference

### 💻 System Control

| Command           | Description                                 |
| ----------------- | ------------------------------------------- |
| `open <app>`      | Launch a known application                  |
| `open downloads`  | Open the Downloads folder                   |
| `open settings`   | Open Windows Settings                       |
| `open explorer`   | Open File Explorer                          |
| `lock screen`     | Lock the current Windows session            |
| `shutdown`        | Ask for confirmation, then shut down the PC |
| `restart`         | Ask for confirmation, then restart the PC   |
| `yes`             | Confirm a pending protected action          |
| `no` / `cancel`   | Cancel a pending protected action           |
| `volume up/down`  | Adjust system volume                        |
| `mute` / `unmute` | Toggle audio                                |
| `play music`      | Media control                               |
| `next track`      | Skip to the next media track                |
| `previous track`  | Go back to the previous media track         |

> Protected power commands use a short confirmation window and expire automatically if they are not confirmed in time.

---

## Frontend Windows

Each UI surface is a separate React app running in its own Tauri window.

| Window            | Path                         | Purpose                                               |
| ----------------- | ---------------------------- | ----------------------------------------------------- |
| **Bubble**        | `src/windows/bubble/`        | Primary interaction: input, voice, subtitles          |
| **Dev Window**    | `src/windows/bubble-dev/`    | Internal settings, command catalog, identity editor   |
| **Quick Menu**    | `src/windows/quick-menu/`    | Fast-access panel for common actions                  |
| **Transcript**    | `src/windows/transcript/`    | Live transcription, processing, and transcript review |
| **Snip Overlay**  | `src/windows/snip-overlay/`  | Region selection for screenshots                      |
| **Snip Panel**    | `src/windows/snip-panel/`    | Analysis results for screenshots                      |
| **Timer Overlay** | `src/windows/timer-overlay/` | Countdown/timer utility                               |

---

## Tauri Bridge Layer

**Location:** `src-tauri/src/lib.rs`

The Tauri layer connects the React frontend to the Rust backend, and also hosts the external command server used by Blob Connectors.

### External command server (Blob Connectors bridge)

The server is started in the `.setup()` block using axum and the AppHandle is passed via state so commands go through the full pipeline:

```rust
// Started automatically in .setup()
tauri::async_runtime::spawn(async move {
    let router = Router::new()
        .route("/command", post(handle_external_command))
        .with_state(ExternalCommandState { app: app_handle });

    let listener = TcpListener::bind("127.0.0.1:7842").await.unwrap();
    axum::serve(listener, router).await.unwrap();
});
```

The handler resolves context, runs `run_command_pipeline`, writes an episodic memory entry, and returns the result — identical to how voice commands are processed.

---

## AI & Model Integration

OpenBlob integrates with **Ollama** for local model inference.

### Default models

| Model          | Use case                 | Pull command               |
| -------------- | ------------------------ | -------------------------- |
| `llama3.1:8b`  | Text, ask, explain       | `ollama pull llama3.1:8b`  |
| `gemma3`       | Vision, screenshots      | `ollama pull gemma3`       |
| `qwen2.5vl:7b` | Better vision (optional) | `ollama pull qwen2.5vl:7b` |

### When Ollama is used

Ollama is invoked as a **fallback only** when no deterministic route matches the input — both in the desktop UI and in Blob Connectors.

---

## Configuration & Profiles

All persistent data lives in local JSON files under `%APPDATA%\OpenBlob\`.

### Data categories

| Category         | Path                            | Contents                                   |
| ---------------- | ------------------------------- | ------------------------------------------ |
| Companion config | `config/companion_config.json`  | Name, language, future wake-word           |
| User profile     | `config/user_profile.json`      | Owner name, app familiarity                |
| Episodic memory  | `memory/episodic_memory.jsonl`  | Timestamped interaction log (JSONL format) |
| Semantic memory  | `memory/semantic_memory.json`   | Learned facts and patterns                 |
| Personality      | `memory/personality_state.json` | Energy, affection, playfulness, focus bias |
| Bonding          | `memory/bonding_state.json`     | Relationship level, trust score            |
| Onboarding state | `config/onboarding_state.json`  | Reserved for future onboarding             |
| Transcript data  | `openblob-data/transcripts/`    | Session JSON, markdown, processing files   |

---

## Global Shortcuts

| Shortcut         | Action                  |
| ---------------- | ----------------------- |
| `CTRL + SPACE`   | Toggle companion bubble |
| `ALT + M`        | Toggle voice input      |
| `CTRL + ALT + S` | Start snip / screenshot |

---

## Blob Connectors

Blob Connectors is a Python layer that bridges external messaging channels to the OpenBlob core. It lives in the `blob_connectors/` directory and runs as a separate process alongside OpenBlob.

### How it works

```
Telegram / Discord / Slack / Email
              │
     blob_connectors/run.py
              │
    ┌─────────┴──────────────┐
    │                        │
OpenBlob running?       Ollama fallback
POST localhost:7842     POST localhost:11434
    │
run_command_pipeline (Rust)
    │
Desktop action + episodic memory entry
```

When OpenBlob is running, commands like `open spotify` sent via Telegram are executed on the desktop exactly as if typed into the bubble. When OpenBlob is not running, the connector falls back to the Ollama model for conversational responses.

### Quickstart

```bash
cd blob_connectors
pip install -r requirements.txt
cp .env.example .env   # fill in tokens
python run.py
```

### Supported channels

| Channel  | Library             | Auth method            | Server required  |
| -------- | ------------------- | ---------------------- | ---------------- |
| Telegram | python-telegram-bot | BotFather token        | No               |
| Discord  | discord.py          | Developer Portal token | No               |
| Slack    | slack-bolt          | Bot token + App token  | No (Socket Mode) |
| Email    | stdlib IMAP/SMTP    | App password           | No               |

### Environment variables

```dotenv
# Telegram
TELEGRAM_BOT_TOKEN=

# Slack
SLACK_BOT_TOKEN=
SLACK_APP_TOKEN=

# Discord
DISCORD_BOT_TOKEN=

# Email
EMAIL_ADDRESS=
EMAIL_PASSWORD=
IMAP_HOST=imap.gmail.com
SMTP_HOST=smtp.gmail.com
IMAP_PORT=993
SMTP_PORT=587
```

Only channels with tokens set are started. You can run just one connector to start.

### Message normalization

All channels normalize to the same `Message` object before reaching the AI handler:

```python
@dataclass
class Message:
    session_id: str     # unique per user/conversation
    user_id: str        # sender identifier in that system
    text: str           # cleaned message content
    channel: str        # "telegram" | "discord" | "slack" | "email"
    username: str       # display name
    message_id: str
    timestamp: datetime
    attachments: list[Attachment]
    raw: Any            # original payload for channel-specific features
```

### Memory context

Before each Ollama call, the connector reads OpenBlob's local memory files and builds a rich system prompt:

- blob name and owner name from `companion_config.json` and `user_profile.json`
- communication style, favorite apps, recurring topics from `semantic_memory.json`
- recent interaction history from `episodic_memory.jsonl`
- current mood derived from `personality_state.json`
- relationship level and trust from `bonding_state.json`
- current channel

This means the blob knows its own name, addresses you by name, and has context from desktop usage — across all channels.

### Adding a new connector

Subclass `BlobConnector` and implement three methods:

```python
from blob_connectors.base import BlobConnector, Message

class MyConnector(BlobConnector):
    def __init__(self):
        super().__init__("myplatform")

    async def receive_message(self, raw) -> Message | None:
        # normalize raw payload to Message
        # return None to ignore this message

    async def send_response(self, original: Message, response: str) -> None:
        # send response back to the user

    async def start(self) -> None:
        # start polling / webhook / socket

    async def stop(self) -> None:
        # clean shutdown
```

Then register it in `build_connectors()` in `run.py`.

### Episodic memory from external channels

When a command is executed via an external connector, the Rust backend writes an episodic memory entry with `app_name` set to the channel name. This means the blob's memory reflects activity across all surfaces — desktop, Telegram, Discord, Slack, and Email — in a single unified log.

---

## Contributing

All contributions are welcome — code, design, documentation, ideas, and testing.

### Before you start

- Open an **issue** before large changes to align on direction
- Small fixes, refactors, and doc improvements can be submitted directly as PRs
- Use the provided [PR template](.github/PULL_REQUEST_TEMPLATE.md)

### Areas open for contribution

| Area            | Examples                                                         |
| --------------- | ---------------------------------------------------------------- |
| Core            | New commands, better routing, bug fixes                          |
| Frontend        | UI polish, new windows, animations                               |
| AI              | Better prompting, model routing, agent ideas                     |
| TTS / Voice     | Voice pipeline improvements, new models                          |
| Browser         | More reliable automation, consent handling                       |
| Memory          | Memory inspector UI, smarter retrieval, RAG over episodic memory |
| Mini games      | New game modes, blob interactions                                |
| i18n            | New language support beyond `en` / `de`                          |
| Tests           | Unit + integration tests across Rust modules                     |
| Docs            | Architecture docs, guides, examples                              |
| Blob Connectors | New channel connectors, calendar/tool integrations, WhatsApp     |

### Development tips

- Run `npm run tauri dev` for hot-reload during development
- Rust changes require a rebuild — use `cargo check` for faster feedback
- Each window is independent — you can work on one UI surface without affecting others
- The external command server on `localhost:7842` starts automatically with OpenBlob

---

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

| Area                           | Status                                                                   |
| ------------------------------ | ------------------------------------------------------------------------ |
| Snip capture region            | ⚠️ May only trigger reliably on the second attempt                       |
| Quick menu window              | ⚠️ Event/capability flow is still being refined after the refactor       |
| Browser automation reliability | ⚠️ Some commands remain less reliable after recent refactors             |
| Deterministic system commands  | ⚠️ Initial Windows system command set is stable, but coverage is limited |
| Protected action UX            | ⚠️ Confirm/cancel/timeout flow works, but richer UI feedback is planned  |
| Multi-model routing fallback   | ⚠️ Fallback logic is still rough in some cases                           |
| Voice recognition pipeline     | ⚠️ Occasional recognition failures still occur                           |
| Context detection edge cases   | ⚠️ Fallback to the last known app is not always correct                  |
| Error handling consistency     | ⚠️ Error handling is still inconsistent across modules                   |
| Settings UI                    | ❌ Not yet implemented                                                   |
| Identity propagation           | ⚠️ Not all response paths are identity-aware yet                         |
| Transcript language quality    | ⚠️ English currently performs better than German                         |
| Speaker separation             | ⚠️ AI-grouped, not true acoustic diarization yet                         |
| Connector session persistence  | ⚠️ Session history is lost when a connector restarts                     |

---

## Roadmap

### Phase 1 — Stabilization

- [ ] Stable command routing pipeline
- [ ] Reliable snip capture
- [ ] Browser automation consent flow
- [ ] Settings UI
- [ ] Improved error handling
- [ ] Identity propagated to all answer paths
- [ ] Expand deterministic Windows system command coverage
- [ ] Improve protected-action UX (confirm / cancel / timeout feedback)
- [ ] Add command debug visibility in the Dev Window

### Phase 2 — Product Polish

- [ ] Onboarding flow
- [ ] Wake-word configuration
- [ ] Memory inspector UI
- [ ] Cleaner multi-model routing
- [ ] Transcript window polish and session UX improvements

### Phase 3 — Intelligence

- [ ] Persistent long-term memory
- [ ] Structured reasoning pipeline
- [ ] Tool-based agent system
- [ ] Better multi-app context awareness
- [ ] Better transcript cleanup and faithful manuscript generation
- [ ] Stronger AI-assisted speaker grouping and transcript understanding
- [ ] Semantic memory retrieval over episodic history (RAG)

### Phase 4 — Platform & Connectors

- [ ] Plugin / capability registry
- [ ] Community skill packs
- [ ] Personality and bonding influence
- [ ] Cross-platform exploration
- [ ] Future microphone + mixed audio transcript modes
- [ ] Transcript-to-memory and meeting intelligence workflows
- [ ] Google Calendar integration via Blob Connectors
- [ ] Voice message support in Telegram connector
- [ ] WhatsApp and Matrix connectors
- [ ] Per-channel permission system

---

## Tech Stack

| Layer                   | Technology                        |
| ----------------------- | --------------------------------- |
| Frontend                | React + TypeScript + Vite         |
| Desktop shell           | Tauri v2                          |
| Backend runtime         | Rust                              |
| External command server | axum (localhost:7842)             |
| AI inference            | Ollama (multi-model)              |
| Vision models           | gemma3 / qwen2.5vl / llama vision |
| Motion / animations     | Framer Motion                     |
| TTS                     | Piper (ONNX) + Kokoro             |
| Speech-to-text          | Whisper CLI (local)               |
| Audio capture           | Windows WASAPI loopback           |
| Blob Connectors         | Python 3.11+ / aiohttp            |
| Platform                | Windows 10 / 11                   |

---

## License

OpenBlob is licensed under the [MIT License](./LICENSE).

---

<div align="center">

**OpenBlob is meant to grow.**

Star the repo · Open issues · Suggest features · Contribute code

</div>
