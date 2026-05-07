<div align="center">

<img src="public/openblob-logo.png" width="325" alt="OpenBlob Logo" />

# OpenBlob

**Open-source desktop copilot for Windows**

![License](https://img.shields.io/badge/license-MIT-7F77DD?style=flat-square)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-378ADD?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-v2-1D9E75?style=flat-square)
![Rust](https://img.shields.io/badge/backend-Rust-EF9F27?style=flat-square)
![React](https://img.shields.io/badge/frontend-React-61DAFB?style=flat-square)
![AI](https://img.shields.io/badge/AI-Local--First-D85A30?style=flat-square)
![Status](https://img.shields.io/badge/status-active%20development-639922?style=flat-square)

</div>

---

> **Build a desktop copilot that feels alive, useful, extensible, and truly personal.**  
> Current focus: **stabilizing the core architecture, improving command reliability, refining the multi-window desktop UX, and building safe local-first voice foundations.**

OpenBlob is a local-first AI companion that lives on your Windows desktop. It can respond to chat, understand your active context, run deterministic desktop actions, analyze screenshots, transcribe system audio, and grow through community-driven features, smarter abilities, better design, and new integrations.

<p align="center">
  <img src="public/1.gif" width="100%" alt="OpenBlob preview" />
</p>

---

## What is OpenBlob?

Most desktop assistants are too limited, too closed, too cloud-dependent, or too impersonal.

OpenBlob aims to be different:

- **Open-source** — built in public, for everyone.
- **Local-first** — designed to run on your machine, not someone else's server.
- **Deterministic-first system control** — handles core Windows actions like opening apps, Downloads, Settings, Explorer, screen locking, and protected power commands.
- **Context-aware** — understands the active app/window and can use controlled session context for follow-up actions.
- **Vision-enabled** — can analyze screenshots and selected regions.
- **Voice-ready** — supports voice input and now includes the foundation for configurable wake-word listening.
- **Privacy-conscious** — transparent about what runs locally, what accesses your mic/screen/browser, and what may touch the network.
- **Extensible** — designed for modules, tools, connectors, and future capability packs.
- **Community-built** — welcoming to developers, designers, tinkerers, and curious builders.
- **High-quality UX** — expressive, playful, polished, and useful.
- **Reachable everywhere** — connect your blob through Telegram, Discord, Slack, or Email.

OpenBlob is currently in an **early but ambitious stage**. The foundation exists, the architecture is evolving, and the project is actively being reorganized to become more reliable, contributor-friendly, and easier to extend.

---

## Current Highlights

- Tauri v2 desktop app with Rust backend and React UI.
- Animated desktop blob companion with multi-window UX.
- Natural language command routing in German and English.
- App/media/browser launch routing for common Windows workflows.
- Controlled target/session context for follow-up commands like `play Thriller`, `scroll down`, `click first result`, or `go back`.
- YouTube search-and-play helpers with fuzzy result selection.
- Deterministic Windows actions and protected confirmation flow for sensitive power commands.
- Screenshot/snipping and vision workflows.
- Transcript module for system-audio transcription and AI post-processing.
- Local memory foundations and SQLite-backed memory work in progress.
- Blob Connectors for Telegram, Discord, Slack, and Email.
- Wake-word settings foundation and local microphone test runtime.

---

## Documentation

For a full technical deep-dive — architecture, core systems, Tauri bridge, module reference, transcript system, blob connectors, and contribution guide — see the developer documentation:

📄 **[docs/documentation.md](./docs/documentation.md)**

---

## Installation / Development

> OpenBlob is in active development. Some setup details may change as the architecture is cleaned up.

### Requirements

- Windows 10 or Windows 11
- Node.js / pnpm
- Rust stable toolchain
- Tauri v2 prerequisites
- Ollama for local LLM workflows
- Optional: Whisper CLI for local transcript workflows

### Run locally

```bash
git clone https://github.com/southy404/openblob.git
cd openblob
pnpm install
pnpm tauri dev
```

If you prefer running from the Tauri crate directly:

```bash
cd src-tauri
cargo tauri dev
```

If `cargo tauri dev` fails with `no such command: tauri`, install the Tauri CLI first or use the pnpm script:

```bash
cargo install tauri-cli --version "^2"
```

---

## Command Reference

OpenBlob uses natural language command parsing. Commands are grouped by capability and interpreted contextually. German and English are supported.

---

### 🌐 Browser & Navigation

**Search**

| Command                                  | Description             |
| ---------------------------------------- | ----------------------- |
| `google <query>` / `google nach <query>` | Perform a Google search |
| `search google for <query>`              | Perform a Google search |
| `youtube <query>`                        | Search on YouTube       |
| `search youtube for <query>`             | Search on YouTube       |

**Open & Navigation**

| Command                     | Description           |
| --------------------------- | --------------------- |
| `open <url>`                | Open a website        |
| `open youtube`              | Open YouTube homepage |
| `go back` / `zurück`        | Navigate back         |
| `forward`                   | Navigate forward      |
| `scroll down` / `scroll up` | Scroll page           |

**Tab & Window Control**

| Command                            | Description      |
| ---------------------------------- | ---------------- |
| `open new tab` / `öffne neuen tab` | Open a new tab   |
| `close tab` / `schließe tab`       | Close active tab |
| `open new window`                  | Open new window  |

**Interaction**

| Command              | Description                |
| -------------------- | -------------------------- |
| `click first result` | Click first visible result |
| `type <text>`        | Type into active input     |
| `submit`             | Confirm input with Enter   |

---

### 🎵 App, Media & Controlled Context

OpenBlob can open apps and services, then keep them as the active controlled target for follow-up commands.

| Command                                      | Description                                      |
| -------------------------------------------- | ------------------------------------------------ |
| `open spotify`                               | Open Spotify and set it as controlled context    |
| `open steam`                                 | Open Steam and set it as controlled context      |
| `open discord`                               | Open Discord                                     |
| `open youtube`                               | Open YouTube and set browser context             |
| `play Michael Jackson Thriller on YouTube`   | Search YouTube and play a matching normal video  |
| `spiele Michael Jackson Thriller auf YouTube`| German YouTube play command                      |
| `play Thriller`                              | Use the active controlled media/browser context  |
| `search for <query>`                         | Search inside the active controlled context      |
| `use this window`                            | Explicitly adopt the focused window as context   |

**Context behavior**

- `open YouTube` → YouTube becomes the controlled browser/web-service target.
- `play Thriller` after opening YouTube → runs inside YouTube instead of becoming a generic search.
- `open Spotify` → Spotify becomes the controlled media target.
- `play Thriller` after opening Spotify → routes to Spotify.
- Manual OS focus changes are treated as passive context and do not automatically replace OpenBlob's controlled target.

---

### 🎬 Streaming & Media

| Command                            | Description          |
| ---------------------------------- | -------------------- |
| `play <title> on netflix`          | Open title           |
| `play something <mood> on netflix` | Get recommendation   |
| `more like this`                   | Show similar content |
| `next video`                       | Play next video      |
| `forward <seconds>`                | Seek forward         |
| `rewind`                           | Seek backward        |

---

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

### ✂️ Screenshot & Vision

| Command           | Description           |
| ----------------- | --------------------- |
| `screenshot`      | Start snip mode       |
| `take screenshot` | Capture screen        |
| `mach screenshot` | German variant        |
| `capture screen`  | Capture screen        |
| `explain this`    | Analyze screenshot    |
| `translate this`  | Translate text        |
| `search this`     | Generate search query |

---

### 🎙️ Voice & Wake Word

| Shortcut / Command | Description |
| ------------------ | ----------- |
| `ALT + M`          | Start voice input manually |
| Wake-word settings | Configure phrase, provider, enabled state, and sensitivity in the dev/settings UI |
| `mic-test` provider | Starts local microphone listener for development testing |
| `mock` provider | Local placeholder provider mode |

OpenBlob now includes the foundation for wake-word support. This is **not a real wake-word model yet**. The current system provides:

- configurable wake-word settings
- safe start/stop listener commands
- local microphone input detection through the wake-word module
- listener states such as `disabled`, `stopped`, `starting`, `listening`, `no_input_device`, `permission_error`, `provider_missing`, and `error`
- selected/default input device reporting
- audio chunk counting
- last audio timestamp
- simple input level/RMS for development UI feedback

Privacy behavior:

- no cloud streaming
- no raw audio storage
- no automatic microphone startup unless explicitly enabled and started
- no fake wake-word detection claims
- existing `ALT + M` voice shortcut remains separate and untouched

---

### ✍️ Transcript & Audio Intelligence

| Command              | Description                              |
| -------------------- | ---------------------------------------- |
| `start transcript`   | Start system-audio transcription         |
| `stop transcript`    | Stop the active transcript session       |
| `open transcript`    | Open the transcript window               |
| `process transcript` | Generate structured AI transcript output |
| `save transcript`    | Persist the current transcript session   |

**What this adds**

- real-time transcription of **system audio**
- dedicated **Transcript window**
- cleaned live transcript stream
- AI post-processing into:
  - faithful transcript
  - speaker-style blocks
  - summary
  - action items

> Current transcript flow is optimized for **system audio** such as meetings, YouTube, podcasts, and browser playback. Microphone and hybrid diarization are planned next.

---

### 🧠 Context, Memory & AI

| Command                | Description              |
| ---------------------- | ------------------------ |
| `what is this`         | Explain current context  |
| `explain selection`    | Explain selected content |
| `where am i`           | Detect current app/page  |
| `what is on this page` | Analyze visible UI       |

OpenBlob is moving toward persistent long-term memory and query-aware context retrieval. Current work includes local memory import, SQLite foundations, semantic facts, episodic events, and embedding-backed retrieval where local embedding models are available.

> If embeddings are unavailable, OpenBlob should continue working and skip embedding gracefully instead of breaking chat.

---

### 🌦️ Daily Info & Smart Replies

| Command                  | Description             |
| ------------------------ | ----------------------- |
| `wie viel uhr ist es`    | Get current time        |
| `what time is it`        | Get current time        |
| `welcher tag ist heute`  | Get current date        |
| `what date is it`        | Get current date        |
| `wie ist das wetter`     | Get current weather     |
| `weather today`          | Get weather             |
| `brauche ich eine jacke` | Clothing recommendation |
| `was soll ich anziehen`  | Outfit suggestion       |

---

### 🎮 Interaction & Modes

| Command         | Description          |
| --------------- | -------------------- |
| `hide and seek` | Start mini game      |
| `dance`         | React to music       |
| `sleep`         | Enter idle state     |
| `wake up`       | Reactivate companion |

---

### ⌨️ Shortcuts

| Shortcut         | Description         |
| ---------------- | ------------------- |
| `CTRL + SPACE`   | Toggle companion UI |
| `ALT + M`        | Voice input         |
| `CTRL + ALT + S` | Screenshot / snip   |

---

### Wake Word

Wake-word support is local-first and disabled by default. The manual `ALT + M` voice shortcut remains the fallback.

- `mic-test` checks the local microphone pipeline only.
- `mock` simulates wake detection for development.
- `local-openwakeword` is the real local provider path: users manually install an openWakeWord-style ONNX bundle under `%APPDATA%/OpenBlob/voice/models/wake-word/`.
- Wake-to-voice is opt-in and starts the existing voice input flow only after a `wake-word-detected` event.

No cloud wake provider, paid API key, automatic model download, or raw audio file recording is required.

---

### Notes

- Commands are **fuzzy matched** — exact wording is not required.
- Language can be mixed between German and English.
- Context is used to resolve intent, such as browser vs app vs media service.
- Some commands adapt based on the active controlled target.
- If no confident action intent is detected, normal chat should remain the fallback path.

---

## 📡 Blob Connectors

![Telegram](https://img.shields.io/badge/Telegram-26A5E4?style=flat-square&logo=telegram&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-5865F2?style=flat-square&logo=discord&logoColor=white)
![Slack](https://img.shields.io/badge/Slack-4A154B?style=flat-square&logo=slack&logoColor=white)
![Email](https://img.shields.io/badge/Email-EA4335?style=flat-square&logo=gmail&logoColor=white)

OpenBlob can be reached from the outside world via **Blob Connectors** — a lightweight Python layer that bridges Telegram, Discord, Slack, and Email to the OpenBlob core running on your desktop.

When OpenBlob is running, the connectors forward commands directly to the same pipeline used by voice and text input. When it is not running, they fall back to the local Ollama model automatically.

```text
Telegram / Discord / Slack / Email
              │
        Blob Connectors (Python)
              │
    ┌─────────┴─────────┐
    │                   │
OpenBlob running?    Ollama fallback
(localhost:7842)     (localhost:11434)
    │
Command Router (Rust)
    │
Desktop action executed
```

### What this enables

- Send `open spotify` via Telegram → Spotify opens on your desktop.
- Ask a question in Discord → Blob answers using your local Ollama model.
- Email the blob → it replies with context from your memory.
- All channels can share the same identity, memory, and personality foundations.

### Quickstart

```bash
cd blob_connectors
pip install -r requirements.txt
cp .env.example .env   # fill in your tokens
python run.py
```

Only connectors with tokens set will start. You can begin with just Telegram and add others later.

### Supported channels

| Channel  | Method       | Notes                                      |
| -------- | ------------ | ------------------------------------------ |
| Telegram | Long-polling | Easiest to set up — no server needed       |
| Discord  | Gateway      | DMs + @mentions — no server needed         |
| Slack    | Socket Mode  | No public endpoint needed                  |
| Email    | IMAP polling | Works with Gmail + App Password or Outlook |

### Architecture

All channels normalize to the same `Message` object before reaching the AI handler. Community contributors can add new connectors by subclassing `BlobConnector` and implementing three methods: `receive_message`, `send_response`, and `start`.

```python
class MyConnector(BlobConnector):
    async def receive_message(self, raw) -> Message | None: ...
    async def send_response(self, original: Message, response: str) -> None: ...
    async def start(self) -> None: ...
```

Full setup guides for each channel are in [`blob_connectors/README.md`](./blob_connectors/README.md).

### Memory integration

Blob Connectors can read OpenBlob's local memory files directly:

- `companion_config.json` — blob name, language, wake-word settings, companion preferences
- `user_profile.json` — owner name
- `semantic_memory.json` — known apps, topics, communication style
- `episodic_memory.jsonl` — past interactions across channels
- `personality_state.json` — current mood such as energy, affection, playfulness
- `bonding_state.json` — relationship level and trust score

The richer your OpenBlob usage, the more context the connectors can carry into every conversation.

---

## Browser Automation

OpenBlob uses Chrome or Edge with remote debugging enabled for advanced browser interactions:

- reading current page context
- navigating in the active tab
- clicking visible links and buttons
- typing into inputs
- YouTube search and play helpers
- browser follow-up actions through controlled target context

> Browser automation is powerful. It should remain transparent and user-controlled. Future versions will make permissions and consent handling even clearer.

---

## Screenshot / Vision Intelligence

OpenBlob can capture your screen or a selected region and reason about what it sees:

- OCR and text extraction
- translation and explanation of on-screen text
- game UI, quest log, and error recognition
- automatic search query generation based on in-game content

> Example: screenshot a quest log → detect the game → extract the objective → build a useful search query.

---

## Transcript / Audio Intelligence

OpenBlob can transcribe **system audio in real time** and turn it into more usable material after recording.

Current transcript workflow:

- capture system audio through local loopback
- split audio into local processing chunks
- transcribe chunks with local Whisper CLI
- show live segments in a dedicated Transcript window
- process the session into:
  - a cleaner faithful transcript
  - speaker-style grouped blocks
  - summary
  - action items

This is especially useful for YouTube videos, podcasts, online meetings, spoken walkthroughs, and lectures.

> The current system is intentionally local-first. Temporary audio chunks are created for processing and then cleaned up to avoid unnecessary data buildup.

---

## Wake Word Foundation

OpenBlob now includes the first foundation for wake-word support.

Current status:

- settings exist for enabled state, wake phrase, sensitivity, and provider
- Tauri commands exist for settings, status, start, and stop
- local microphone availability detection is supported
- `mic-test` / `mock` provider modes can start a local listener
- dev UI can show listener state, selected input device, chunks seen, last audio timestamp, and input level
- no actual wake-word detection model is wired yet

Planned next steps:

- real wake-word provider integration, such as Porcupine or another local model
- wake phrase management through the dev/settings UI
- consent-first onboarding for microphone access
- better visual feedback while listening
- wake-word event bridge into the existing voice command flow

---

## Mini Games

OpenBlob has a growing interactive side beyond just being an assistant.

**Hide & Seek** — trigger via voice or text command. The blob hides somewhere on screen. You find it.

More game modes are planned as the project grows.

---

## Design Principles

**1. Local-first**  
Whenever possible, things run locally on the user's machine.

**2. Context > Prompt**  
The assistant should understand your environment — what app you're in, what's on screen, and what it opened before.

**3. Privacy-conscious**  
Users should understand what runs locally, what accesses the browser, what uses the mic, and what may call external services.

**4. Extensible by design**  
New modules, commands, tools, and UI ideas should be straightforward to add.

**5. Community over gatekeeping**  
This project welcomes contributions from developers, designers, tinkerers, AI enthusiasts, and curious builders.

**6. High-quality UX matters**  
A desktop copilot should not just work — it should feel polished, expressive, modern, and enjoyable to use.

**7. Playful, but actually useful**  
Fun interactions and real productivity are not opposites.

---

## Roadmap

### Core

- [ ] Stabilize command routing
- [ ] Preserve normal chat fallback when no confident action intent is detected
- [ ] Improve active app / controlled context detection
- [ ] Fix snip capture reliability
- [ ] Stabilize quick menu window actions / permissions flow
- [ ] Improve browser automation reliability and consent handling
- [ ] Improve voice pipeline
- [ ] Expand settings UI
- [ ] Better error handling across all modules
- [ ] Expand multilingual support beyond current `en` / `de` groundwork

### AI / Intelligence

- [ ] Persistent long-term memory ([proposal](./docs/proposals/memory-system.md))
- [ ] Better multi-model routing
- [ ] Structured reasoning pipeline
- [ ] Tool-based agent system
- [ ] Query-aware semantic memory retrieval
- [ ] Embedding-backed memory retrieval with graceful fallback
- [ ] Higher-accuracy transcript cleanup pipeline
- [ ] Real speaker diarization
- [ ] Transcript-to-memory extraction

### Voice / Wake Word

- [x] Manual voice shortcut foundation
- [x] Wake-word settings foundation
- [x] Local microphone test runtime
- [ ] Real wake-word provider integration
- [ ] Wake-word event bridge into voice command flow
- [ ] Microphone permission/onboarding UX
- [ ] Noise handling and false-positive prevention
- [ ] Local-only wake phrase/model configuration

### Blob Connectors

- [ ] Google Calendar integration through connector/tool layer
- [ ] Semantic memory retrieval per channel
- [ ] Persistent session history across restarts
- [ ] WhatsApp connector
- [ ] Matrix / Element connector
- [ ] Voice message support, such as Telegram voice → Whisper → command
- [ ] Per-channel permission system

### Avatar / UX

- [ ] Richer blob behaviors and reactions
- [ ] Personality system with persistent character state
- [ ] More emotional states and animations
- [ ] UI polish pass: glassmorphism, motion, feel
- [ ] Cleaner onboarding

### Mini Games & Fun

- [ ] More mini game modes beyond Hide & Seek
- [ ] Score tracking and blob reactions to outcomes
- [ ] Interactive blob challenges: tap, race, puzzle

### Platform

- [ ] Plugin architecture
- [ ] Capability registry
- [ ] App launch capability registry
- [ ] Contributor extension guide
- [ ] Community skill packs

### Quality

- [ ] Tests
- [ ] Contributor docs
- [ ] CI improvements
- [ ] Release workflow
- [ ] Smoke-test checklist for Windows app/media/browser workflows

---

## Contributing

Contributions are welcome — all kinds, not just code.

| Area            | Examples                                                         |
| --------------- | ---------------------------------------------------------------- |
| Code            | bug fixes, refactors, new commands, new modules                  |
| Design          | avatar animations, UI/UX improvements, onboarding                |
| Docs            | architecture, guides, contribution ideas                         |
| Ideas           | new integrations, capability proposals, architecture feedback    |
| Quality         | tests, CI, issue templates                                       |
| Mini games      | new game modes, interaction ideas                                |
| AI experiments  | prompting strategies, model routing, agent ideas                 |
| Transcript      | ASR cleanup, diarization, transcript UX, meeting workflows       |
| Voice           | wake-word providers, mic permission UX, local voice pipeline      |
| Blob Connectors | new channel connectors, memory improvements, calendar/tool hooks |

Please open an issue before large changes so we can align on direction.

Smaller cleanup PRs, architecture improvements, UI polish, docs work, and bug fixes are especially welcome while the project structure is being stabilized.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for full details.

---

## Design Goals

OpenBlob should feel:

- **alive** — not static, reacts to context and what's on screen
- **smooth** — fluid motion, no jank
- **modern** — glassmorphism, soft motion, minimal clutter
- **non-intrusive** — stays out of the way when not needed
- **playful, but actually useful** — personality without sacrificing function

Design matters as much as functionality in this project.

---

## Status

**Early-stage, actively evolving, and currently being refactored.**

Recent work focused on:

- making the structure more open-source-friendly
- preparing multilingual support (`en` / `de`)
- separating larger UI elements into dedicated windows
- introducing the transcript module and transcript window
- improving long-term maintainability
- adding deterministic Windows system commands with protected confirmation flow
- adding Blob Connectors for Telegram, Discord, Slack, and Email
- connecting external channels to the Rust command pipeline via local HTTP
- hardening app/media/browser launch routing
- adding controlled target context for follow-up browser and media actions
- making memory embedding failures non-blocking for normal chat
- adding wake-word settings and local microphone test foundation

The project is already functional, but still has rough edges and active regressions in some areas. Expect rapid changes, experimental ideas, and ongoing cleanup.

---

## License

This project is licensed under the [MIT License](./LICENSE).

---

## Acknowledgements

Built with inspiration from:

- desktop companion applications
- local-first AI tools
- agent and automation systems
- modern interface design
- the open-source community

---

## Topics

`desktop-copilot` `tauri` `react` `rust` `ollama` `local-ai` `open-source` `desktop-assistant` `automation` `windows` `voice` `wake-word` `microphone` `vision` `screenshot` `transcript` `speech-to-text` `whisper` `framer-motion` `mini-games` `context-aware` `telegram` `discord` `slack` `connectors`

---

<div align="center">

**OpenBlob is meant to grow.**

If you want to help shape the future of desktop copilots — you're invited.

⭐ Star the repo · 🐛 Open issues · 💡 Suggest features · 🛠 Contribute code

</div>
