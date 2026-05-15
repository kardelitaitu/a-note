# TODO

## Phase 1 — Project Scaffolding

- [ ] Create Tauri v2 project (`cargo tauri init`)
- [ ] Choose and set up frontend (vanilla HTML/CSS/JS or Svelte)
- [ ] Configure `tauri.conf.json` for single window:
  - Frameless (`decorations: false`)
  - Always-on-top (`alwaysOnTop: true`)
  - Resizable
  - Default size and position
- [ ] Set up `tauri.conf.json` security capabilities for filesystem access

## Phase 2 — Rust Backend (Tauri Commands)

### Config persistence (`sticky.config`)
- [ ] Define `Config` struct (`width`, `height`, `left`, `top`, `font_size`)
- [ ] Read config from `sticky.config` **next to the .exe** (`std::env::current_exe()`)
- [ ] Write config to `sticky.config` (on resize, move, font zoom, close)
- [ ] Return defaults if file missing or corrupt

### Note content persistence (`sticky.notes`)
- [ ] Read note content from `sticky.notes` file **next to the .exe** (JSON inside, `.notes` extension)
- [ ] Write note content to `sticky.notes` (`{ "text": "..." }`)
- [ ] **Auto-save** via timer (every 30s)

### Window management
- [ ] Expose `get_config` / `save_config` commands to frontend
- [ ] Expose `load_note` / `save_note` commands to frontend
- [ ] Handle window close — save config + note before exit

## Phase 3 — Frontend

### Layout
- [ ] Frameless window with dark semi-transparent background (`#80000000`)
- [ ] `<textarea>` styled with no border, no outline, transparent bg
- [ ] Custom title bar buttons (Minimize `—`, Close `✕`) in top-right corner
- [ ] Slim scrollbar styling (4px wide thumb)

### Features
- [ ] Load note text on startup
- [ ] **Auto-save** every 30s via `setInterval` → `save_note`
- [ ] **Ctrl+Scroll** zoom: font size range 8–72, step factor 1.1
- [ ] **Double-click** a line to select it (preserve scroll offset)
- [ ] Word-wrap disabled (horizontal scroll for long lines)

### Window integration
- [ ] Save config on window resize/move events
- [ ] Save config on font zoom
- [ ] Save config + note on close

## Phase 4 — Polish

- [ ] Test all features match reference behavior
- [ ] Handle monitor DPI / multi-monitor edge cases
- [ ] App icon (use reference icons as base)
- [ ] `cargo tauri build` — verify production build works
- [ ] Lint Rust code (`clippy`)
- [ ] Remove `this is the reference folder/` from release builds (keep in dev)

## Future Ideas (Stretch)

- [ ] Multiple sticky notes (tabs or separate windows)
- [ ] Rich text / markdown support
- [ ] Syntax highlighting
- [ ] Sync across devices
