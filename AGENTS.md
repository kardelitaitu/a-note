# Agent Instructions

## Project Overview

Rust + Tauri desktop sticky notes app. Single-window, frameless, always-on-top text editor with auto-save.

## Reference App (PowerShell/WPF)

The reference lives in `this is the reference folder/`. It implements:

- Frameless window with dark semi-transparent background (`#80000000`)
- Always-on-top, resizable
- Plain text editing with word-wrap disabled, monospace-like behavior
- **Auto-save** content every 30s to `temp.json` (we'll use `sticky.notes`)
- **Config persistence** (window position, size, font size) to `sticky.config` (reference uses `StickyNote.cfg`)
- **Ctrl+Scroll** to zoom font size (range 8–72, step factor 1.1)
- **Double-click** a line to select it (preserves scroll offset)
- Minimize (`—`) and Close (`✕`) buttons in top-right corner
- Slim scrollbar styling (4px wide thumb)

## Tech Stack

- **Backend**: Rust + Tauri v2
- **Frontend**: (to be decided — likely vanilla HTML/CSS/JS or Svelte)
- **Persistence**: `sticky.config` (config) + `sticky.notes` (content) in the **executable's directory** (portable — move the `.exe` + both files anywhere)

## Build & Run

```powershell
# Dev
cargo tauri dev

# Build
cargo tauri build
```

## Conventions

- Rust: 4-space indent, `snake_case`, clippy-clean
- Frontend: match framework conventions
- No external DB — file-based JSON persistence only
- Single window, no tray, no system menu
