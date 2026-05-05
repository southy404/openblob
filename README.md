<div align="center">

<img src="public/openblob-logo.png" width="325" alt="OpenBlob Logo" />

# OpenBlob

**open-source desktop copilot for Windows**

![License](https://img.shields.io/badge/license-MIT-7F77DD?style=flat-square)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-378ADD?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-v2-1D9E75?style=flat-square)
![Rust](https://img.shields.io/badge/backend-Rust-EF9F27?style=flat-square)
![AI](https://img.shields.io/badge/AI-Multi--Model-D85A30?style=flat-square)
![Status](https://img.shields.io/badge/status-active%20development-639922?style=flat-square)

</div>

---

> **Build a desktop copilot that feels alive, useful, extensible, and truly personal.** <br />
> Current focus: **stabilizing the architecture, improving contributor-friendliness, and refining the multi-window desktop UX.**

OpenBlob is a local-first AI companion that lives on your Windows desktop — sees your screen, understands your context, and grows through community-driven features, smarter abilities, better design, and new integrations.

<p align="center">
  <img src="public/1.gif" width="100%" alt="Screenshot" />
</p>

---

## What is OpenBlob?

Most desktop assistants are too limited, too closed, too cloud-dependent, or too impersonal.

OpenBlob aims to be different:

- **open-source** — built in public, for everyone
- **local-first** — runs on your machine, not someone else's server
- **deterministic-first system control** — handles core Windows actions like opening Downloads, Settings, Explorer, screen locking, and protected power commands
- **context-aware** — understands what app you're in, not just what you type
- **vision-enabled** — analyzes your screen and selected regions
- **privacy-conscious** — transparent about what touches the network
- **extensible** — designed for modules, plugins, and future capability packs
- **community-built** — welcoming to devs, designers, tinkerers, and curious builders
- **high-quality UX** — polished, expressive, playful, and useful
- **reachable everywhere** — talk to your blob via Telegram, Discord, Slack, or Email

OpenBlob is currently in an **early but ambitious stage**:
the foundation is there, the architecture is evolving, and the project is actively being reorganized to become more contributor-friendly and easier to extend.

---

## Documentation

For a full technical deep-dive — architecture, core systems, Tauri bridge, module reference, transcript system, blob connectors, and contribution guide — see the developer documentation:

📄 **[docs/documentation.md](./docs/documentation.md)**

---

## Command Reference

OpenBlob uses natural language command parsing.  
Commands are grouped by capability and interpreted contextually (German + English supported).

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
| `submit`             | Confirm input (Enter)      |

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

### 🧠 Context & AI

| Command                | Description              |
| ---------------------- | ------------------------ |
| `what is this`         | Explain current context  |
| `explain selection`    | Explain selected content |
| `where am i`           | Detect current app/page  |
| `what is on this page` | Analyze visible UI       |

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

### Notes

- Commands are **fuzzy matched** — exact wording is not required
- Language can be mixed (German + English)
- Context is used to resolve intent (e.g. browser vs app vs game)
- Some commands adapt based on the current active application

---

## 📡 Blob Connectors

![Telegram](https://img.shields.io/badge/Telegram-26A5E4?style=flat-square&logo=telegram&logoColor=white)
![Discord](https://img.shields.io/badge/Discord-5865F2?style=flat-square&logo=discord&logoColor=white)
![Slack](https://img.shields.io/badge/Slack-4A154B?style=flat-square&logo=slack&logoColor=white)
![Email](https://img.shields.io/badge/Email-EA4335?style=flat-square&logo=gmail&logoColor=white)

OpenBlob can be reached from the outside world via **Blob Connectors** — a lightweight Python layer that bridges Telegram, Discord, Slack, and Email to the OpenBlob core running on your desktop.

When OpenBlob is running, the connectors forward commands directly to the same pipeline used by voice and text input. When it is not running, they fall back to the local Ollama model automatically.

```
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

- Send `open spotify` via Telegram → Spotify opens on your desktop
- Ask a question in Discord → Blob answers using your local Ollama model
- Email the blob → it replies with context from your memory
- All channels share the same identity, memory, and personality

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

Blob Connectors read OpenBlob's local memory files directly:

- `companion_config.json` — blob name and preferred language
- `user_profile.json` — owner name
- `semantic_memory.json` — known apps, topics, communication style
- `episodic_memory.jsonl` — past interactions across all channels
- `personality_state.json` — current mood (energy, affection, playfulness)
- `bonding_state.json` — relationship level and trust score

The richer your OpenBlob usage, the more context the connectors carry into every conversation.

---

## Design Principles

**1. Local-first**
Whenever possible, things run locally on the user's machine.

**2. Context > Prompt**
The assistant should understand your environment — what app you're in, what's on screen — not just what you type.

**3. Privacy-conscious**
Users should understand what runs locally, what accesses the browser, and what may call external services.

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
- [ ] Improve app / context detection
- [ ] Fix snip capture reliability
- [ ] Stabilize quick menu window actions / permissions flow
- [ ] Improve browser automation reliability + consent handling
- [ ] Improve voice pipeline
- [ ] Add settings UI
- [ ] Better error handling across all modules
- [ ] Expand multilingual support beyond current `en` / `de` groundwork

### AI / Intelligence

- [ ] Persistent long-term memory ([proposal](./docs/proposals/memory-system.md))
- [ ] Better multi-model routing
- [ ] Structured reasoning pipeline
- [ ] Tool-based agent system
- [ ] Higher-accuracy transcript cleanup pipeline
- [ ] Real speaker diarization
- [ ] Transcript-to-memory extraction
- [ ] Semantic memory retrieval (RAG over episodic memory)

### Blob Connectors

- [ ] Google Calendar integration (create, read, delete events)
- [ ] Semantic memory retrieval per channel
- [ ] Persistent session history across restarts
- [ ] WhatsApp connector
- [ ] Matrix / Element connector
- [ ] Voice message support (Telegram voice → Whisper → command)
- [ ] Per-channel permission system

### Avatar / UX

- [ ] Richer blob behaviors and reactions
- [ ] Personality system (persistent character state)
- [ ] More emotional states and animations
- [ ] UI polish pass (glassmorphism, motion, feel)
- [ ] Cleaner onboarding

### Mini Games & Fun

- [ ] More mini game modes beyond Hide & Seek
- [ ] Score tracking / blob reactions to outcomes
- [ ] Interactive blob challenges (tap, race, puzzle)

### Platform

- [ ] Plugin architecture
- [ ] Capability registry
- [ ] Contributor extension guide
- [ ] Community skill packs

### Quality

- [ ] Tests
- [ ] Contributor docs
- [ ] CI improvements
- [ ] Release workflow

---

## Browser Automation

OpenBlob uses Chrome or Edge with remote debugging enabled for advanced browser interactions:

- reading current page context
- navigating in the active tab
- clicking visible links and buttons
- typing into inputs
- YouTube search and play helpers

> Browser automation is powerful — it remains transparent and user-controlled. Future versions will make permissions and consent handling even clearer.

---

## Screenshot / Vision Intelligence

OpenBlob can capture your screen or a selected region and reason about what it sees:

- OCR and text extraction
- Translation and explanation of on-screen text
- Game UI, quest log, and error recognition
- Automatic search query generation based on in-game content

> Example: screenshot a quest log → detect the game → extract the objective → build the perfect search query. All locally.

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

## Mini Games

OpenBlob has a growing interactive side beyond just being an assistant.

**Hide & Seek** — trigger via voice or text command. The blob hides somewhere on screen. You find it.

More game modes are planned as the project grows.

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
- separating larger UI elements into dedicated windows (like the quick menu)
- introducing the first transcript module + transcript window
- improving long-term maintainability
- adding deterministic Windows system commands with protected confirmation flow for sensitive actions
- adding Blob Connectors for Telegram, Discord, Slack, and Email
- connecting external channels to the Rust command pipeline via local HTTP

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

`desktop-copilot` `tauri` `react` `rust` `ollama` `local-ai` `open-source` `desktop-assistant` `automation` `windows` `voice` `vision` `screenshot` `transcript` `speech-to-text` `whisper` `framer-motion` `mini-games` `context-aware` `telegram` `discord` `slack` `connectors`

---

<div align="center">

**OpenBlob is meant to grow.**

If you want to help shape the future of desktop copilots — you're invited.

⭐ Star the repo · 🐛 Open issues · 💡 Suggest features · 🛠 Contribute code

</div>
