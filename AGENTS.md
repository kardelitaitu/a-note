# Agent Instructions

## Project Overview

Rust + Tauri v2 desktop sticky notes app. Single-window, frameless, always-on-top text editor with auto-save, password encryption, and system tray icon.

## Tech Stack

- **Backend**: Rust + Tauri v2 (tray-icon feature)
- **Frontend**: Vanilla HTML/CSS/JS built with Vite
- **Persistence**: Single `{exe}.notes` file (NoteData JSON: version, config, note, log)
- **Encryption**: AES-256-GCM + Argon2id
- **Testing**: cargo test (lib + integration + property/proptest)

## Build & Run

```bash
cd src-tauri
cargo tauri dev     # development
cargo tauri build   # release build → target/release/notes.exe
```

## Project Structure

```
sticky-notes-rust/
├── index.html                 # App shell + hamburger menu HTML
├── src/
│   ├── main.js                # Core UI logic, menu, password, themes, fonts
│   └── style.css              # All styles, themes, menu, lock overlay
├── src-tauri/
│   ├── Cargo.toml             # Rust dependencies
│   ├── tauri.conf.json        # Tauri config (copyright, version, window)
│   └── src/
│       ├── main.rs            # Entry point (calls sticky_notes_lib::run)
│       ├── lib.rs             # Tauri commands, AppState, salt_from_config, run()
│       ├── config.rs          # Config struct (window, font, theme, password, etc.)
│       ├── crypto.rs          # AES-256-GCM encrypt/decrypt, Argon2id key derivation
│       ├── diagnostics.rs     # In-memory event log, crash reporter ({exe}.crash)
│       ├── note.rs            # Note/NoteFile structs, load/save, migration helpers
│       ├── storage.rs         # NoteData struct (combined config+note+log), save/load/migrate
│       ├── tray.rs            # System tray icon (colored circle, left-click toggle)
│       └── util.rs            # file write (preserve hidden), Windows registry FFI
└── tests/
    ├── encryption.rs          # Full workflow integration tests
    └── property.rs            # Proptest: roundtrip, nonce uniqueness, wrong key
```

## Key Architecture

- **storage.rs** is the single source of truth. All I/O goes through `storage::load()` / `storage::save()`.
- **NoteData** wraps Config + NoteFile + log string in one JSON file.
- **Migration** from v0.1.x separate files (.config + .notes + .log) happens on first v0.2.0 launch.
- **Auto-repair**: `storage::load()` fixes deadlock states (password_protected=true with empty salt + unencrypted note).
- **diagnostics.rs** uses in-memory buffer, flushed to NoteData.log on save. Crash reporter writes standalone `{exe}.crash`.
- **All commands** (load_config, save_config, load_note, save_note, password ops) go through storage.

## Conventions

- Rust: 4-space indent, snake_case, `cargo fix` before commit
- Frontend: lowercase-with-hyphens IDs, className toggle for states
- Test count tracked in README and memory after each expansion
- Branch naming: `vX.Y.Z` (both branches and tags — use `refs/tags/` when pushing tags)
- Release: `gh release create` with `notes.exe` binary
- No external DB, no tray menu on macOS/Linux (Windows tray only), no network telemetry

## Menu Layout (hamburger)

```
[Theme           ▶]  Appearance
[Titlebar color  ▶]
[Font            ▶]
─────
[Word wrap]           Behavior
[Auto Start]
─────                   (dynamic)
[Set/Change password]   Security
[Lock now]
[Remove password]
```
