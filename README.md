<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="a-note">
</p>

<h1 align="center">a-note</h1>

<p align="center">
  <strong>~9 MB · Zero install · Portable · Encrypted</strong>
</p>

<p align="center">
  A sticky notes app that lives in a single `.exe` — no installers, no dependencies, no config files scattered across your system. Every note can be password-protected with AES-256-GCM encryption.
</p>

<p align="center">
  <a href="https://github.com/kardelitaitu/a-note/releases/tag/v0.2.0">⬇ Download notes.exe (v0.2.0)</a>
</p>

---

**Drop it anywhere, run it.** Config and notes save right next to the `.exe` in a single `.notes` file. Rename the file to anything — data files follow. Move the folder to another machine and everything goes with you: notes, window position, font size, pin state, encryption settings.

Built with Rust + Tauri v2. No Electron. No bloat.

## Features

| | |
|---|---|
| 🔒 **AES-256-GCM encryption** | Password-protect notes with Argon2id key derivation |
| ⏰ **Auto-lock timer** | Configurable timeout — note locks after inactivity |
|| 🎨 **15 themes** | Dark, light, dracula, nord, catppuccin, gruvbox, solarized, and more |
|| 🎯 **Custom titlebar color** | Native color picker with fill slider (0–100%) |
|| ✅ **Confirm password** | Two-field password entry ensures no typos on set/change |
| 📌 **Always on top** | Toggle pin to keep the window above everything |
| ⌨️ **Ctrl+Scroll** | Zoom font size 8–72px in real time |
| 💾 **Auto-save** | 5s after you stop typing, also on minimize/close/pin toggle |
| 📝 **Word wrap** | Toggle in the hamburger menu |
| 🖱️ **Double-click** | Selects the entire line, preserves scroll position |
| 🪟 **Frameless** | Dark theme, clean monospace editor, slim scrollbar |
|| 📁 **Portable** | One `.exe`, one `.notes` file. Move anywhere. |
| 🏷️ **Smart title** | Title bar shows the `.exe` filename — rename freely |
| 🔒 **Hidden-safe** | Files keep their hidden attribute when rewritten |
| 📋 **Crash logging** | Panics captured to `{exe}.crash` — no silent failures |

## Security

- **Encryption:** AES-256-GCM with random 12-byte nonces (unique per encryption)
- **Key derivation:** Argon2id with random 16-byte salt
- **Tamper detection:** GCM authentication tag — bit flips, truncation, and corruption are all detected
- **In-memory key:** Decryption key cached only in RAM, cleared on lock or close
- **Config auto-repair:** If config is corrupted, password protection resets gracefully
- **No telemetry:** All logs stay local — zero network calls

## Download

**[⬇ Download notes.exe (v0.2.0)](https://github.com/kardelitaitu/a-note/releases/tag/v0.2.0)**

No installation. No setup. Run it.

Older releases: [v0.1.3](https://github.com/kardelitaitu/a-note/releases/tag/v0.1.3)

## Build from source

### Prerequisites

- Rust 1.70+
- Node.js 18+
- Tauri CLI 2.x (`cargo install tauri-cli --version "^2"`)

### Windows / macOS / Linux

```bash
git clone https://github.com/kardelitaitu/a-note.git
cd a-note
npm install
cargo tauri build
# Output: src-tauri/target/release/notes(.exe)
```

Linux requires additional system libraries — see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Mobile (Android / iOS)

Experimental Tauri v2 mobile support — see [CONTRIBUTING.md](CONTRIBUTING.md).

## Testing

```bash
# 196 tests across unit, integration, and property-based suites
cd src-tauri
cargo test --lib              # 174 unit tests
cargo test --test encryption  # 14 integration tests
cargo test --test property    # 8 property-based tests (proptest)
```

## Stack

| Layer | Technology |
|---|---|
| Backend | Rust + Tauri v2 |
| Crypto | AES-256-GCM (aes-gcm) + Argon2id (argon2) |
| Frontend | Vanilla JS + Vite |
| Styling | CSS custom properties (15 themes) |
| Persistence | JSON files next to `.exe` |
| Testing | cargo test + proptest |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) — includes setup guides for Windows, macOS, Linux, and mobile.

## License

MIT — see [LICENSE](LICENSE).

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.
