# a-note

A fully portable, always-on-top sticky notes app for Windows — rewritten from PowerShell/WPF into Rust + Tauri.

**Zero install. Zero dependencies.** Drop `notes.exe` anywhere, run it. Config (`sticky.config`) and notes (`sticky.notes`) live right next to it. Move the whole folder to another machine — your notes and window position travel with you.

## Features

- Frameless, resizable, always-on-top window
- Dark theme with monospace text
- Plain text editing with auto-save (every 30s)
- Window position, size, and font size persist across sessions
- **Ctrl+Scroll** to zoom font size (8–72px)
- **Double-click** a line to select it
- Minimize and Close buttons in top-right corner

## Build

```powershell
# Install Tauri CLI (one-time)
cargo install tauri-cli --version "^2"

# Dev mode (hot-reload)
cargo tauri dev

# Production build — outputs to src-tauri/target/release/notes.exe
cargo tauri build
```

## Project Structure

```
├── src/                    # Frontend (vanilla JS)
│   ├── main.js
│   └── style.css
├── src-tauri/              # Rust + Tauri backend
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── lib.rs          # Tauri commands + setup
│   │   ├── config.rs       # sticky.config read/write
│   │   └── note.rs         # sticky.notes read/write
│   ├── capabilities/       # Tauri v2 permissions
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── index.html
├── package.json
└── vite.config.js
```

## Reference

The original PowerShell/WPF implementation is in `this is the reference folder/`.

## License

MIT
