# Architecture

OpenBlob is split into:

## Frontend

- React
- UI / Avatar / Interaction

## Backend (Rust / Tauri)

- command routing
- system control
- browser automation
- app launching

## Modules

- command_router → parses intent
- browser_automations → controls browser
- steam_games → Steam integration
- system → OS actions
- session_memory → context
- voice → speech input

---

Goal: modular, extensible, hackable.
