<div align="center">

<img src="public/openblob-logo.png" width="325" alt="OpenBlob Logo" />

# OpenBlob

**open-source desktop copilot for Windows**

![License](https://img.shields.io/badge/license-MIT-7F77DD?style=flat-square)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-378ADD?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-v2-1D9E75?style=flat-square)
![Rust](https://img.shields.io/badge/backend-Rust-EF9F27?style=flat-square)
![Ollama](https://img.shields.io/badge/AI-Ollama-D85A30?style=flat-square)
![Status](https://img.shields.io/badge/status-early%20stage-639922?style=flat-square)

</div>

---

> **Build a desktop copilot that feels alive, useful, extensible, and truly personal.**

OpenBlob is a local-first AI companion that lives on your Windows desktop — and grows through community-driven features, smarter abilities, better design, and new integrations.

---

## What is OpenBlob?

Most desktop assistants are too limited, too closed, too cloud-dependent, or too impersonal.

OpenBlob aims to be different:

- **open-source** — built in public, for everyone
- **local-first** — runs on your machine, not someone else's server
- **privacy-conscious** — transparent about what touches the network
- **extensible** — designed for modules, plugins, and new capabilities
- **community-built** — welcoming to devs, designers, tinkerers, and curious builders
- **high-quality UX** — polished, expressive, and enjoyable to use

---

## Features

### Current / in progress

| Feature                                             | Status |
| --------------------------------------------------- | ------ |
| Desktop companion UI (transparent Tauri window)     | ✅     |
| Organic blob avatar with Framer Motion              | ✅     |
| Voice + typed command routing                       | ✅     |
| Ollama local AI integration                         | ✅     |
| Browser automation via Chrome/Edge remote debugging | ✅     |
| Local app launching                                 | ✅     |
| Steam game launching                                | ✅     |
| Session memory for recent interactions              | ✅     |
| Basic command parsing (German + English)            | ✅     |
| Speech bubble / companion bubble windows            | ✅     |

### Planned

- Settings UI
- Plugin / capability system
- Richer blob behaviors and emotional states
- Better onboarding experience
- Music-reactive avatar motion
- Better browser awareness and consent handling
- More local model support
- Community skill packs
- Cross-platform exploration

---

## Tech Stack

| Layer    | Technology                |
| -------- | ------------------------- |
| Frontend | React + TypeScript + Vite |
| Desktop  | Tauri v2                  |
| Backend  | Rust                      |
| AI       | Ollama                    |
| Motion   | Framer Motion             |
| Platform | Windows 10 / 11           |

---

## Getting Started

### Requirements

- Windows 10 or 11
- [Node.js](https://nodejs.org/)
- [Rust](https://rustup.rs/) + Cargo
- [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
- [Ollama](https://ollama.com/) installed locally
- Chrome or Edge (for browser automation features)

### Install dependencies

```bash
npm install
```

### Run in development

```bash
npm run tauri dev
```

### Build frontend only

```bash
npm run build
```

### Ollama setup

```bash
ollama serve
ollama pull llama3.1:8b
```

> If your local model setup differs, adapt the model name in the app configuration or Rust backend.

---

## Project Structure

```
openblob/
├─ src/                          # React frontend
├─ src-tauri/
│  └─ src/
│     ├─ lib.rs
│     └─ modules/
│        ├─ app_profiles.rs
│        ├─ browser_automations.rs
│        ├─ command_router.rs    # intent parsing
│        ├─ session_memory.rs
│        ├─ steam_games.rs
│        ├─ system.rs
│        ├─ voice.rs
│        └─ windows_discovery.rs
├─ docs/
│  ├─ architecture.md
│  ├─ roadmap.md
│  └─ design.md
├─ .github/
│  ├─ ISSUE_TEMPLATE/
│  │  ├─ bug_report.md
│  │  └─ feature_request.md
│  ├─ workflows/
│  │  └─ ci.yml
│  └─ PULL_REQUEST_TEMPLATE.md
├─ .gitignore
├─ CHANGELOG.md
├─ CODE_OF_CONDUCT.md
├─ CONTRIBUTING.md
├─ LICENSE
├─ README.md
└─ SECURITY.md
```

---

## Philosophy

**1. Local-first**
Whenever possible, things run locally on the user's machine.

**2. Privacy-conscious**
Users should understand what runs locally, what accesses the browser, and what may call external services.

**3. Extensible by design**
New modules, commands, tools, and UI ideas should be straightforward to add.

**4. Community over gatekeeping**
This project welcomes contributions from developers, designers, tinkerers, AI enthusiasts, and curious builders.

**5. High-quality UX matters**
A desktop copilot should not just work — it should feel polished, expressive, modern, and enjoyable to use.

---

## Roadmap

### Core

- [ ] Stabilize command routing
- [ ] Improve app discovery
- [ ] Improve browser automation reliability
- [ ] Improve voice pipeline
- [ ] Add settings UI
- [ ] Better error handling

### Avatar / UX

- [ ] Richer blob behaviors
- [ ] Petting / emotional reactions
- [ ] Sleep / wake / hide presence states
- [ ] Music-reactive motion
- [ ] Cleaner onboarding

### Platform

- [ ] Plugin architecture
- [ ] Capability registry
- [ ] Contributor extension guide
- [ ] Community skill packs

### Quality

- [ ] Tests
- [ ] Contributor docs
- [ ] Issue templates
- [ ] CI
- [ ] Release workflow

---

## Browser Automation

OpenBlob uses Chrome or Edge with remote debugging enabled for advanced browser interactions:

- reading current page context
- navigating in the active tab
- clicking visible links and buttons
- typing into inputs
- YouTube search and play helpers

> Browser automation is powerful — it remains transparent and user-controlled. Future versions will make permissions and status even clearer.

---

## Contributing

Contributions are welcome — all kinds, not just code.

| Area    | Examples                                                      |
| ------- | ------------------------------------------------------------- |
| Code    | bug fixes, refactors, new commands, new modules               |
| Design  | avatar animations, UI/UX improvements, onboarding             |
| Docs    | architecture, guides, contribution ideas                      |
| Ideas   | new integrations, capability proposals, architecture feedback |
| Quality | tests, CI, issue templates                                    |

Please open an issue before large changes so we can align on direction.

See [CONTRIBUTING.md](./CONTRIBUTING.md) for full details.

---

## Design Goals

OpenBlob should feel:

- **alive** — not static, reacts to context
- **smooth** — fluid motion, no jank
- **modern** — glassmorphism, soft motion, minimal clutter
- **non-intrusive** — stays out of the way when not needed
- **playful, but actually useful** — personality without sacrificing function

Design matters as much as functionality in this project.

---

## Status

**Early-stage, actively evolving.**

Expect rapid changes, rough edges, experimental ideas, and ongoing refactors.

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

`desktop-copilot` `tauri` `react` `rust` `ollama` `local-ai` `open-source` `desktop-assistant` `automation` `windows` `voice` `framer-motion`

---

<div align="center">

**OpenBlob is meant to grow.**

If you want to help shape the future of desktop copilots — you're invited.

⭐ Star the repo · 🐛 Open issues · 💡 Suggest features · 🛠 Contribute code

</div>
