# Changelog

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
