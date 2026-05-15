# Contributing

Thanks for your interest in contributing to Sticky Notes!

## Project Overview

A portable, encrypted sticky notes app built with Rust + Tauri v2.  
Single-window, frameless, always-on-top text editor with AES-256-GCM encryption.

**Tech stack:** Rust backend, vanilla HTML/CSS/JS frontend, Tauri v2 desktop framework.

## Development Setup

### Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.70+ | `rustup install stable` |
| Node.js | 18+ | For Vite frontend build |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2"` |

### Windows

```powershell
# 1. Install Rust
winget install Rustlang.Rustup
rustup default stable

# 2. Install Tauri prerequisites (MSVC Build Tools + WebView2)
#    WebView2 is pre-installed on Windows 10 (version 1803+) and Windows 11
#    MSVC build tools: https://visualstudio.microsoft.com/visual-cpp-build-tools/

# 3. Clone and build
git clone https://github.com/kardelitaitu/a-note.git
cd a-note
npm install
cargo tauri dev    # Dev mode with hot-reload
cargo tauri build  # Production build → src-tauri/target/release/notes.exe
```

### macOS

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install Xcode Command Line Tools
xcode-select --install

# 3. Clone and build
git clone https://github.com/kardelitaitu/a-note.git
cd a-note
npm install
cargo tauri dev
cargo tauri build   # → src-tauri/target/release/notes (dmg/app)
```

### Linux

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Install Tauri system dependencies
# Debian/Ubuntu:
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Fedora:
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libxdo-devel libappindicator-gtk3-devel librsvg2-devel

# Arch:
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file \
  openssl appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg

# 3. Clone and build
git clone https://github.com/kardelitaitu/a-note.git
cd a-note
npm install
cargo tauri dev
cargo tauri build   # → src-tauri/target/release/notes (AppImage/deb)
```

### Mobile (Android / iOS)

Tauri v2 supports mobile targets. To build for mobile:

```bash
# Install mobile prerequisites
cargo tauri init --mobile

# Android
cargo tauri android init
cargo tauri android dev      # Run on connected device/emulator
cargo tauri android build    # → src-tauri/gen/android/app/build/outputs/apk/

# iOS (macOS only)
cargo tauri ios init
cargo tauri ios dev
cargo tauri ios build        # → src-tauri/gen/ios/build/
```

**Note:** Mobile support is experimental in this repo. The UI is designed for desktop and will need responsive adjustments for mobile screens.

## Project Structure

```
a-note/
├── index.html              # Entry point (Vite builds from here)
├── package.json            # Frontend dependencies (Vite, Tauri API)
├── src/
│   ├── main.js             # Frontend logic (Tauri invoke calls)
│   └── style.css           # All styling (15 themes, lock overlay, etc.)
├── src-tauri/
│   ├── Cargo.toml          # Rust dependencies
│   ├── tauri.conf.json     # Tauri window/build configuration
│   └── src/
│       ├── main.rs         # Tauri entry point
│       ├── lib.rs          # Tauri commands + app state
│       ├── config.rs       # Config persistence (sticky.config)
│       ├── crypto.rs       # AES-256-GCM + Argon2id
│       ├── diagnostics.rs  # Crash reporting + event logging
│       ├── note.rs         # Note/NoteFile model + persistence
│       └── util.rs         # File I/O helpers
└── tests/
    ├── encryption.rs       # Integration tests
    └── property.rs         # Property-based tests (proptest)
```

## Code Style

### Rust
- 4-space indent
- `snake_case` for functions/variables, `CamelCase` for types
- `Result<_, String>` for error handling (no unwrap in production code)
- Run `cargo clippy` before submitting
- All public functions should have doc comments

### Frontend (JS/CSS)
- 2-space indent
- `camelCase` for JS variables/functions
- CSS custom properties for theming
- No external JS frameworks — vanilla only

## Testing

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --lib                    # Unit tests
cargo test --test encryption        # Integration tests
cargo test --test property          # Property-based tests
cargo test <test_name>              # Single test
```

All new features must include tests. For encryption/crypto changes, add:
- Unit tests in the relevant module
- Integration tests in `tests/encryption.rs`
- Property-based tests in `tests/property.rs` if applicable

## Making Changes

1. **Fork and branch**
   ```bash
   git checkout -b feat/your-feature
   ```

2. **Make changes** — use conventional commit messages:
   ```
   feat: add new feature
   fix: correct bug in encryption
   test: add test for edge case
   docs: update contributing guide
   refactor: simplify key derivation
   ```

3. **Ensure quality gates pass**
   ```bash
   cargo test                    # All tests pass
   cargo clippy -- -D warnings   # No clippy warnings
   cargo build                   # Compiles clean
   ```

4. **Push and open a PR**
   ```bash
   git push -u origin feat/your-feature
   ```
   Open a PR against the `master` branch with a clear description.

## Commit Guidelines

- One logical change per commit
- Keep commits small and focused
- Reference issues/PRs in commit messages where relevant
- Use present tense ("Add feature" not "Added feature")

## Release Process

Releases follow [semantic versioning](https://semver.org/):

1. Update version in `Cargo.toml` and `tauri.conf.json`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Build with `cargo tauri build`
5. Create GitHub release with `.exe` asset and changelog

See the `publish_release_github` skill for the automated workflow.

## License

MIT — see `LICENSE` for details.
