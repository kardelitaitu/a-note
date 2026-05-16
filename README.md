<p align="center">
  <img src="src-tauri/icons/128x128.png" width="96" height="96" alt="a-note">
</p>

<h1 align="center">a-note</h1>

<p align="center">
  <strong>Your notes app that travels with you.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/tests-205%20passing-brightgreen" alt="tests">
  <img src="https://img.shields.io/badge/Rust-1.70%2B-DEA584?logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white" alt="Tauri v2">
  <img src="https://img.shields.io/badge/platform-Windows-0078D6?logo=windows11&logoColor=white" alt="Windows">
  <img src="https://img.shields.io/github/v/release/kardelitaitu/a-note?sort=semver&label=version" alt="Version">
  <img src="https://img.shields.io/github/downloads/kardelitaitu/a-note/total?label=downloads" alt="Downloads">
  <img src="https://img.shields.io/github/license/kardelitaitu/a-note" alt="License">
</p>

<p align="center">
  Keep notes, settings, and security in one local file beside the app,
  with no installer and no cloud dependency.
</p>

<p align="center">
  <strong>Auto Start for instant availability · Always on Top toggle for active workflows.</strong>
</p>

<p align="center">
  <a href="https://github.com/kardelitaitu/a-note/releases/tag/v0.2.0">Download latest release</a>
  ·
  <a href="src/preview.mp4">Watch full preview (MP4)</a>
  ·
  <a href="#build-from-source">Build from source</a>
</p>

<p align="center">
  <a href="src/preview.mp4">
    <img src="src/preview.gif" alt="a-note preview" width="880">
  </a>
</p>

---

## Overview

a-note is designed for reliable personal note-taking with a minimal operational footprint.
Application state is kept next to the executable so it remains portable across folders and machines.
Password-protected notes use authenticated encryption and are never sent over the network.

## Feature Highlights

| Capability | Details |
|---|---|
| Portable runtime | Single executable with local `.notes` data file |
| Encryption at rest | AES-256-GCM for note ciphertext |
| Key derivation | Argon2id with per-password random salt |
| Password lifecycle | Set, change, remove, lock, and unlock flows |
| Auto-lock | Inactivity timeout with configurable duration |
| Editing experience | Word wrap, zoom, line select, and autosave |
| UI customization | Multiple themes, font options, title bar color |
| Desktop behavior | Always-on-top mode and startup option |
| Diagnostics | Local crash/event logging |

## Security Model

- Encryption: AES-256-GCM with random nonce generation per encryption.
- Password derivation: Argon2id with random 16-byte salt.
- Tamper detection: authentication tag validation detects corruption and modification.
- Key handling: decryption key stays in memory and is cleared on lock/close.
- Data locality: logs and notes remain local to the machine.

## Download

Latest release: [notes.exe v0.2.0](https://github.com/kardelitaitu/a-note/releases/tag/v0.2.0)

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

Linux may require additional system libraries. See [CONTRIBUTING.md](CONTRIBUTING.md).

### Mobile (Android / iOS)

Experimental Tauri v2 mobile support is documented in [CONTRIBUTING.md](CONTRIBUTING.md).

## Testing

```bash
cd src-tauri
cargo test --lib
cargo test --test encryption
cargo test --test property
```

## Technology Stack

| Layer | Technology |
|---|---|
| Backend | Rust + Tauri v2 |
| Crypto | AES-256-GCM + Argon2id |
| Frontend | Vanilla JavaScript + Vite |
| Styling | CSS custom properties |
| Persistence | Local JSON beside executable |
| Testing | cargo test + proptest |

## Contributing

Development setup and platform notes are in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).

## Changelog

Release history is tracked in [CHANGELOG.md](CHANGELOG.md).
