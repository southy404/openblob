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
   - [Memory System](#memory-system)
   - [Companion Identity](#companion-identity)
   - [Text-to-Speech](#text-to-speech)
6. [Frontend Windows](#frontend-windows)
7. [Command Reference](#command-reference)
8. [Tauri Bridge Layer](#tauri-bridge-layer)
9. [AI & Model Integration](#ai--model-integration)
10. [Configuration & Profiles](#configuration--profiles)
11. [Global Shortcuts](#global-shortcuts)
12. [Contributing](#contributing)
13. [Known Issues](#known-issues)
14. [Roadmap](#roadmap)
15. [License](#license)

---

## Overview

OpenBlob is an **open-source, local-first desktop companion** for Windows 10/11.

It goes beyond a simple chatbot — it acts as an **operating-layer assistant** that can:

- execute desktop commands directly
- control your browser via remote debugging
- understand your screen through vision models
- remember context across sessions
- speak to you using TTS
- grow with you through a configurable companion identity

**Core design principle:**

> Deterministic first. AI second.

Whenever a command can be executed locally without a model, it is. AI is used as a capability layer — not the whole product.

---

## Architecture

OpenBlob is split into three major layers:

```
┌─────────────────────────────────────────────────┐
│                   UI Layer (React)               │
│  bubble · dev-window · quick-menu · snip-panel  │
│        snip-overlay · timer-overlay             │
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
│  Screen capture · Memory · Identity · TTS       │
└─────────────────────────────────────────────────┘
```

### Data Flow

```
User Input (text / voice)
       │
       ▼
Command Router
       │
  ┌────┴──────────┐
  │               │
Direct Action   Ollama Fallback
(local/browser/ (ask / explain /
 system/media)   translate / vision)
  │               │
  └────┬──────────┘
       │
Subtitle Output + TTS
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
- Allow the app through your antivirus if you trust it. See [Security Notice](#security-notice).
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
4. System / app commands   → launch, volume, media
5. Snip / vision commands  → screenshot, explain, translate
6. Streaming commands      → Netflix, YouTube playback
7. Ollama fallback          → ask, explain, translate (model)
```

#### Key modules

| File           | Purpose                                       |
| -------------- | --------------------------------------------- |
| `intents.rs`   | Maps normalized input to recognized intents   |
| `matchers.rs`  | Pattern matching per command group            |
| `fuzzy.rs`     | Fuzzy string similarity for tolerant matching |
| `parser.rs`    | Tokenizes and classifies input                |
| `normalize.rs` | Lowercasing, trimming, language normalization |
| `types.rs`     | `CommandRoute`, `RouteResult`, `Intent` types |

#### Extending the router

To add a new command:

1. Define the intent in `intents.rs`
2. Add matchers in `matchers.rs`
3. Add i18n keys to `i18n/commands/en.json` and `de.json`
4. Wire the handler in `mod.rs`

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

#### Current limitations

- Requires the debug browser to be running before browser commands work
- Some commands may be less reliable after page navigations
- Browser binary paths are currently hardcoded — this is being improved

---

### Screen & Vision

**Location:** `src-tauri/src/modules/screen_capture.rs` · `snip_session.rs`

The snip system enables screenshot-based interactions.

#### Workflow

```
User triggers snip
       │
Snip overlay opens
       │
User selects screen region
       │
Image captured with context metadata (active app, window title)
       │
Passed to Ollama vision model (gemma3 / qwen2.5vl)
       │
Returns structured result
```

#### Analysis modes

| Mode        | What it does                                             |
| ----------- | -------------------------------------------------------- |
| OCR         | Extract visible text from screen                         |
| Translate   | Detect language + translate on-screen text               |
| Explain     | Describe what is shown                                   |
| Search      | Generate a useful search query from the content          |
| Game assist | Detect game UI / quest text / errors and suggest actions |

#### Vision model setup

```bash
ollama pull gemma3        # default vision model
ollama pull qwen2.5vl:7b  # higher quality alternative
```

Set the active model in app configuration or the Rust backend config.

---

### Memory System

**Location:** `src-tauri/src/modules/memory/`

OpenBlob uses a layered memory architecture.

#### Memory layers

| Layer    | File                 | Purpose                                                   |
| -------- | -------------------- | --------------------------------------------------------- |
| Episodic | `episodic_memory.rs` | Logs of past interactions and events                      |
| Semantic | `semantic_memory.rs` | Recurring facts: known apps, topics, patterns             |
| Session  | `session_memory.rs`  | Runtime context: last command, last search, browser state |

#### Design rule

> Memory must never block execution.

Memory is loaded and saved asynchronously. If a memory operation fails, core commands still execute.

#### Storage

Memory is persisted to local JSON files via `storage/json_store.rs`. File paths are resolved through `storage/paths.rs`.

---

### Companion Identity

**Location:** `src-tauri/src/modules/profile/companion_config.rs`

OpenBlob maintains a configurable identity layer for its companion persona.

#### Identity fields

| Field                | Default | Notes                              |
| -------------------- | ------- | ---------------------------------- |
| `blob_name`          | "Blob"  | The companion's display name       |
| `owner_name`         | ""      | Your name — used in self-reference |
| `preferred_language` | "en"    | Controls i18n command parsing      |

#### Editing identity

Identity is editable in the **Dev Window** under the Identity section. Full onboarding UI is planned for a future release.

#### Current status

Identity values are stored and editable. Not every answer path is fully identity-aware yet — this is an ongoing alignment task across the codebase.

---

### Text-to-Speech

**Location:** `src-tauri/src/modules/tts/`

OpenBlob includes native TTS using bundled voice models.

#### Engines

| Engine | File        | Notes                                  |
| ------ | ----------- | -------------------------------------- |
| Piper  | `piper.rs`  | Fast, local ONNX-based voice synthesis |
| Kokoro | `kokoro.rs` | Alternative engine (experimental)      |

#### Bundled models

```
src-tauri/models/
├── de_DE-thorsten-medium.onnx.json   # German voice
└── en_US-lessac-high.onnx.json       # English voice
```

#### TTS manager

`manager.rs` handles:

- selecting the correct engine and model based on language
- queuing and interrupting speech
- toggling TTS on/off from the bubble UI

---

## Frontend Windows

Each UI surface in OpenBlob is a separate React app running in its own Tauri window.

| Window            | Path                         | Purpose                                             |
| ----------------- | ---------------------------- | --------------------------------------------------- |
| **Bubble**        | `src/windows/bubble/`        | Primary interaction: input, voice, subtitles        |
| **Dev Window**    | `src/windows/bubble-dev/`    | Internal settings, command catalog, identity editor |
| **Quick Menu**    | `src/windows/quick-menu/`    | Fast-access panel for common actions                |
| **Snip Overlay**  | `src/windows/snip-overlay/`  | Region selection for screenshots                    |
| **Snip Panel**    | `src/windows/snip-panel/`    | Analysis results for screenshots                    |
| **Timer Overlay** | `src/windows/timer-overlay/` | Countdown/timer utility                             |

### Window communication

Windows communicate via Tauri's event system:

```typescript
// Emit from any window
import { emit } from "@tauri-apps/api/event";
await emit("snip-complete", { imagePath: "..." });

// Listen in any window
import { listen } from "@tauri-apps/api/event";
await listen("snip-complete", (event) => {
  console.log(event.payload);
});
```

### Tauri invoke (calling Rust from frontend)

```typescript
import { invoke } from "@tauri-apps/api/core";

// Execute a user command
const result = await invoke("handle_command", { input: "open youtube" });

// Get current companion config
const config = await invoke("get_companion_config");
```

---

## Command Reference

OpenBlob uses natural language parsing. Commands are fuzzy-matched — exact wording is not required. German and English are both supported.

### Browser & Navigation

| Command              | Description              |
| -------------------- | ------------------------ |
| `google <query>`     | Google search            |
| `youtube <query>`    | YouTube search           |
| `open <url>`         | Open a website           |
| `go back` / `zurück` | Navigate back            |
| `open new tab`       | New browser tab          |
| `close tab`          | Close current tab        |
| `click first result` | Click first visible link |
| `type <text>`        | Type into active field   |
| `scroll down / up`   | Scroll current page      |

### System & Apps

| Command                         | Description        |
| ------------------------------- | ------------------ |
| `open <app>`                    | Launch application |
| `volume up / down`              | System volume      |
| `mute / unmute`                 | Toggle audio       |
| `next track` / `previous track` | Media navigation   |
| `play music`                    | Media play         |

### Screenshot & Vision

| Command          | Description                           |
| ---------------- | ------------------------------------- |
| `screenshot`     | Start snip mode                       |
| `explain this`   | Analyze current screenshot            |
| `translate this` | Translate on-screen text              |
| `search this`    | Generate search query from screenshot |

### Utility

| Command                 | Description     |
| ----------------------- | --------------- |
| `what time is it`       | Current time    |
| `what date is it`       | Current date    |
| `weather today`         | Current weather |
| `start timer 5 minutes` | Start a timer   |
| `coin flip`             | Flip a coin     |

### Streaming

| Command                            | Description            |
| ---------------------------------- | ---------------------- |
| `play <title> on netflix`          | Open Netflix title     |
| `play something <mood> on netflix` | Netflix recommendation |
| `next video`                       | Skip to next           |
| `forward <seconds>`                | Seek forward           |

> **Note:** Commands like `youtube lofi beats`, `play lofi beats on youtube`, and `search youtube for lofi beats` all resolve to the same action.

---

## Tauri Bridge Layer

**Location:** `src-tauri/src/main.rs`

The Tauri layer connects the React frontend to the Rust backend.

### Registering Tauri commands

```rust
// In main.rs
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        handle_command,
        get_companion_config,
        update_companion_config,
        start_snip,
        // ...
    ])
    .run(tauri::generate_context!())
    .expect("error running OpenBlob");
```

### Registering global shortcuts

```rust
use tauri_plugin_global_shortcut::GlobalShortcutExt;

app.global_shortcut().register("CTRL+SPACE", || {
    // toggle bubble window
})?;
```

### Window management

Windows are opened via `open.ts` files in each window directory:

```typescript
// src/windows/bubble/open.ts
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export function openBubble() {
  new WebviewWindow("bubble", {
    url: "bubble.html",
    transparent: true,
    alwaysOnTop: true,
    decorations: false,
  });
}
```

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

Ollama is invoked as a **fallback only** when no deterministic route matches the input.

Use cases:

- open-ended questions
- general explanations
- translation (when no local route is active)
- vision analysis (screenshot flows)

### Model configuration

Adjust default models in the Rust backend configuration. A settings UI for model selection is planned.

---

## Configuration & Profiles

All persistent data lives in local JSON files managed by `storage/json_store.rs`.

### Data categories

| Category         | File                    | Contents                         |
| ---------------- | ----------------------- | -------------------------------- |
| Companion config | `companion_config.json` | Name, language, future wake-word |
| User profile     | `user_profile.json`     | Owner name, app familiarity      |
| Episodic memory  | `episodic_memory.json`  | Interaction history              |
| Semantic memory  | `semantic_memory.json`  | Learned facts and patterns       |
| Onboarding state | `onboarding_state.json` | Reserved for future onboarding   |

File paths are resolved via `storage/paths.rs`, typically inside the app's local data directory.

---

## Global Shortcuts

| Shortcut         | Action                  |
| ---------------- | ----------------------- |
| `CTRL + SPACE`   | Toggle companion bubble |
| `ALT + M`        | Toggle voice input      |
| `CTRL + ALT + S` | Start snip / screenshot |

> `CTRL + SPACE` is currently slightly unstable and being improved.

---

## Security Notice

OpenBlob uses deep system integration. Some security software may flag it.

**Capabilities that may trigger warnings:**

- Global keyboard shortcuts (keyboard hooks)
- Screen capture and region snipping
- Input simulation (keyboard and mouse)
- Active window and process inspection
- Browser automation via remote debugging
- Local AI model execution

**What OpenBlob does NOT do:**

- Send your data to external servers (unless you configure an external API or model)
- Store any data outside your local machine
- Run hidden background processes beyond what Tauri requires

**OpenBlob is fully open-source.** You can read every line of code before running it.

If you encounter antivirus blocks:

1. Add the OpenBlob directory to your antivirus exclusion list
2. Allow the app through Windows Defender
3. Ensure port `9222` is not blocked by a firewall (needed for browser automation)

---

## Contributing

All contributions are welcome — code, design, documentation, ideas, and testing.

### Before you start

- Open an **issue** before large changes to align on direction
- Small fixes, refactors, and doc improvements can be submitted directly as PRs
- Use the provided [PR template](.github/PULL_REQUEST_TEMPLATE.md)

### Areas open for contribution

| Area        | Examples                                     |
| ----------- | -------------------------------------------- |
| Core        | New commands, better routing, bug fixes      |
| Frontend    | UI polish, new windows, animations           |
| AI          | Better prompting, model routing, agent ideas |
| TTS / Voice | Voice pipeline improvements, new models      |
| Browser     | More reliable automation, consent handling   |
| Memory      | Memory inspector UI, smarter retrieval       |
| Mini games  | New game modes, blob interactions            |
| i18n        | New language support beyond `en` / `de`      |
| Tests       | Unit + integration tests across Rust modules |
| Docs        | Architecture docs, guides, examples          |

### Development tips

- Run `npm run tauri dev` for hot-reload during development
- Rust changes require a rebuild — use `cargo check` for faster feedback
- Each window is independent — you can work on one UI surface without affecting others
- Check `docs/architecture.md` for system-level design notes

---

## Known Issues

| Area                           | Status                                                |
| ------------------------------ | ----------------------------------------------------- |
| `CTRL + SPACE` global shortcut | ⚠️ Slightly unstable, WIP                             |
| Snip capture region            | ⚠️ May only trigger reliably on second attempt        |
| Quick menu window              | ⚠️ Event/capability flow being refined after refactor |
| Browser automation reliability | ⚠️ Some commands less reliable after recent refactors |
| Multi-model routing fallback   | ⚠️ Logic still rough                                  |
| Voice recognition pipeline     | ⚠️ Occasional recognition failures                    |
| Context detection edge cases   | ⚠️ Fallback to last known app not always correct      |
| Error handling consistency     | ⚠️ Inconsistent across modules                        |
| Settings UI                    | ❌ Not yet implemented                                |
| Identity propagation           | ⚠️ Not all answer paths are identity-aware yet        |

---

## Roadmap

### Phase 1 — Stabilization

- [ ] Stable command routing pipeline
- [ ] Reliable snip capture
- [ ] Browser automation consent flow
- [ ] Settings UI
- [ ] Improved error handling
- [ ] Identity propagated to all answer paths

### Phase 2 — Product Polish

- [ ] Onboarding flow
- [ ] Wake-word configuration
- [ ] Memory inspector UI
- [ ] Cleaner multi-model routing

### Phase 3 — Intelligence

- [ ] Persistent long-term memory
- [ ] Structured reasoning pipeline
- [ ] Tool-based agent system
- [ ] Better multi-app context awareness

### Phase 4 — Platform

- [ ] Plugin / capability registry
- [ ] Community skill packs
- [ ] Personality and bonding influence
- [ ] Cross-platform exploration

---

## Tech Stack

| Layer               | Technology                        |
| ------------------- | --------------------------------- |
| Frontend            | React + TypeScript + Vite         |
| Desktop shell       | Tauri v2                          |
| Backend runtime     | Rust                              |
| AI inference        | Ollama (multi-model)              |
| Vision models       | gemma3 / qwen2.5vl / llama vision |
| Motion / animations | Framer Motion                     |
| TTS                 | Piper (ONNX) + Kokoro             |
| Platform            | Windows 10 / 11                   |

---

## License

OpenBlob is licensed under the [MIT License](./LICENSE).

---

<div align="center">

**OpenBlob is meant to grow.**

Star the repo · Open issues · Suggest features · Contribute code

</div>
