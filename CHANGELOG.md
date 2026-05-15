# Changelog

## 0.1.5 — 2026-05-16

### Added
- "Auto Start" toggle in hamburger menu: starts notes.exe on Windows boot
- Windows Run registry key (`HKCU\...\Run\Notes`) written via raw Win32 FFI — no new crate deps
- `set_start_with_windows` Tauri command + JS toggle handler with checkmark feedback
- `get_startup_registry()` read-back function (validates key via `RegQueryValueExW`)
- `start_with_windows` config field with `#[serde(default)]` for backward compat
- Font preview on hover: hovering a font in the selection submenu temporarily applies it to the editor
- 4 registry integration tests: set, read, disable, idempotency (serialized via Mutex)
- 3 config tests: default false, JSON roundtrip, missing-field backward compat

### Changed
- CSS checkmark rule now applies to both `#menu-wordwrap.on` and `#menu-startup.on`
- Hamburger menu reordered: Appearance (Theme, Titlebar color, Font) → Behavior toggles (Word wrap, Auto Start) → Security (password items)

### Security
- 175 total tests (153 lib + 14 integration + 8 property-based)

## 0.1.4 — 2026-05-15

### Added
- System tray icon with colored circle matching titlebar color (left-click show/hide, right-click Quit)
- Tray icon tooltip shows exe filename (follows rename)
- Crash reporter: panics captured to `{exe}.crash`, events logged to `{exe}.log`
- `update_tray_color` Tauri command — tray icon updates in real-time on color change
- `TrayState` managed state for dynamic tray icon updates
- 10 tray unit tests: `parse_hex_color` edge cases, icon generation, size/pixel
- 4 diagnostics unit tests: event logging, multi-line append, timestamp, path
- Font selection submenu navigation fix
- `applyFont()` called on config load for correct font on startup

### Changed
- Window title (taskbar label) now matches exe filename dynamically
- `tray::build()` accepts `tooltip` and `initial_color` — no blue flash on startup
- Tray icon color loaded from config at build time, not patched after

### Security
- 124 total tests (109 lib + 11 integration + 4 property-based)

## 0.1.3 — 2026-05-15

### Added
- Font selection in hamburger menu: 10 fonts (Cascadia Code, JetBrains Mono, Fira Code, Source Code Pro, Inter, Roboto, Open Sans, Segoe UI, Arial, Georgia)
- Font submenu with back navigation and checkmark on active font
- Google Fonts CDN integration (7 web fonts loaded automatically)
- Font selection persists across sessions via `font_family` config field
- `font_family` config backward compatibility (defaults to "Cascadia Code" on old configs)
- Crash reporter: panics captured to `{app}.crash`, major events logged to `{app}.log`
- `CONTRIBUTING.md` with cross-platform setup guides (Windows, macOS, Linux, mobile)
- MIT `LICENSE` file
- Encrypted→plaintext migration: `remove_password` now writes consistent `NoteFile` format
- 4 new migration tests: format validation, file I/O, empty note, unicode
- 2 new font config tests: default, backward compat

### Changed
- `remove_password` uses `note::save_file` with `NoteFile { encrypted: false }` instead of old `Note::save`
- Close menu clears font submenu state
- `Config` struct: added `font_family: String` field

### Security
- Crash logs stored locally next to `.exe` — no network calls
- Event logging for password operations (set, unlock, lock, remove, change)

## 0.1.2 — 2026-05-15

### Added
- AES-256-GCM encryption for note content with Argon2id key derivation
- Password protection: set, change, remove password via lock overlay
- Auto-lock timer: configurable timeout (0–60 min, "Never" option), resets on edit
- Lock screen with password prompt — auto-blurs editor 3px, click-to-focus input
- Lock now button in hamburger menu (visible when password is set)
- Set/Change password menu item (toggles label based on state)
- Remove password menu item (visible when protected)
- Close button (✕) on lock overlay to quit without unlocking
- Key derivation salt + nonce stored as hex in config and notes file
- Cached encryption key in app state (cleared on lock/close)
- Config corruption auto-repair: missing salt resets password protection
- `NoteFile` struct: unified on-disk format supporting encrypted and plaintext
- Backward compatible with pre-0.1.2 `.notes` files (no migration needed)
- Lock timeout slider in password setup dialog (not in hamburger menu)
- Property-based tests via `proptest` (4 tests, 10 random cases each)
- Integration test suite: full workflow, file I/O, re-encryption, tamper detection
- 101 total unit + integration tests

### Changed
- `crypto::decrypt` validates nonce length (12 bytes) and returns Err instead of panicking
- `unlock`, `remove_password`, `change_password` use `salt_from_config` helper with auto-repair
- Lock overlay blur reduced 8px → 3px for better readability
- `crypto`, `note`, `config`, `util` modules made public for integration testing

### Security
- AES-256-GCM with random 12-byte nonces (unique per encryption)
- Argon2id key derivation with random 16-byte salt
- GCM authentication tag prevents tampering (bit flips detected)
- Key cached only in-memory, cleared on lock/close
- Empty passwords rejected at command layer
- Nonce uniqueness enforced: same plaintext + same key produces different ciphertext
- Auth tag truncation and corruption detected
- Invalid nonce lengths handled gracefully

## 0.1.1 — 2026-05-15

### Added
- Cursor + scroll position remembered and restored on reopen
- Word wrap toggle in hamburger menu
- Dark/light theme toggle expanded to 15 themes (dark, light, dark-black, dark-blue, dark-choco, light-blue, light-orange, dracula, monokai, nord, solarized-dark, solarized-light, gruvbox-dark, gruvbox-light, catppuccin)
- Theme preview on hover in submenu
- Theme submenu with back navigation
- Titlebar custom color picker with fill slider (0–100% opacity)
- Auto-contrast for title text/buttons based on titlebar color luminance
- Title text opacity increased to 80% across all themes
- Hamburger menu with expandable submenus
- Tooltips on all titlebar buttons
- Fluent-style titlebar buttons with SVG icons
- Proper thumbtack pin SVG icon
- Hidden attribute preserved when rewriting config/notes files
- Unit tests: 17 tests covering config, note, and util modules

### Changed
- Auto-save: 5s debounce after last edit (was every 30s), also saves on minimize/close/pin toggle
- Data files match exe name (notes.exe → notes.config + notes.notes)
- Window centers on first launch (no config file), restores saved position otherwise
- Titlebar background matches app background (no text scroll-through)
- Removed position/size set from frontend (Rust setup handles it)

### Fixed
- Taskbar icon stretched vertically (was 224×256 non-square .ico → proper square sizes)
- White/black flash on window resize (webview background color set to #1e1e1e)
- Cursor position not restoring on reopen

## 0.1.0 — 2026-05-14

### Added
- Full Rust + Tauri v2 rewrite from PowerShell/WPF
- Frameless, always-on-top, resizable window
- Dark theme with monospace editor
- Plain text editing with auto-save every 30s
- Ctrl+Scroll font size zoom (8–72px, factor 1.1)
- Double-click to select line (preserves scroll)
- Window position, size, font size persistence
- Always-on-top pin toggle
- Minimize and close buttons
- Portable: single .exe, data files beside it
- App icon (from reference)

## 0.0.1 — 2026-04-20

### Added
- Initial PowerShell + WPF prototype
- Frameless window with dark semi-transparent background
- Always-on-top, resizable
- Plain text editing with auto-save every 30s
- Config persistence (StickyNote.cfg)
- Note content persistence (temp.json)
- Ctrl+Scroll font size zoom
- Double-click line selection
- Minimize and close buttons
- Slim scrollbar styling
