<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="a-note">
</p>

<h1 align="center">a-note</h1>

<p align="center">
  <strong>~8 MB · Zero install · Portable</strong>
</p>

<p align="center">
  A sticky notes app for Windows that lives in a single `.exe` — no installers, no dependencies, no config files scattered across your system.
</p>

<p align="center">
  <a href="https://github.com/kardelitaitu/a-note/releases/tag/v0.1.0">⬇ Download notes.exe</a>
</p>

---

**Drop it anywhere, run it.** Config and notes save right next to the `.exe` (`notes.config` + `notes.notes`). Rename the file to anything — data files follow. Move the folder to another machine and everything goes with you: notes, window position, font size, pin state.

Built with Rust + Tauri. No Electron. No bloat.

## Features

| | |
|---|---|
| 📌 **Always on top** | Toggle pin to keep the window above everything |
| ⌨️ **Ctrl+Scroll** | Zoom font size 8–72px in real time |
| 💾 **Auto-save** | Every 30 seconds — never lose a thought |
| 🖱️ **Double-click** | Selects the entire line, preserves scroll position |
| 🪟 **Frameless** | Dark theme, clean monospace editor, slim scrollbar |
| 📁 **Portable** | One `.exe`, two data files. Move anywhere. |
| 🏷️ **Smart title** | Title bar shows the `.exe` filename — rename freely |
| 🔒 **Hidden-safe** | Files keep their hidden attribute when rewritten |

## Download

[⬇ Download notes.exe (v0.1.0)](https://github.com/kardelitaitu/a-note/releases/tag/v0.1.0)

No installation. No setup. Run it.

## Build from source

```powershell
cargo install tauri-cli --version "^2"
cargo tauri build
# Output: src-tauri/target/release/notes.exe
```

## Stack

**Backend:** Rust + Tauri v2 · **Frontend:** Vanilla JS + Vite · **Persistence:** JSON files next to the `.exe`

## License

MIT
