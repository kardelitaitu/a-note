# TODO — v0.1.2

## Completed in v0.1.1

### 1. Remember cursor + scroll position ✅
- `cursor_pos` + `scroll_top` in Note struct with backward compat
- Saves in each auto-save, restores via `setTimeout(0)` after text render

### 2. Word wrap toggle ✅
- Implemented in hamburger menu dropdown (not standalone button)
- `word_wrap: bool` in Config, toggles `white-space: pre` / `pre-wrap`
- Tests: config roundtrip, backward compat

### 3. Theme system (expanded) ✅
- 15 themes: dark, dark-black, dark-blue, dark-choco, light-blue, light-orange, dracula, monokai, nord, solarized-dark, solarized-light, gruvbox-dark, gruvbox-light, catppuccin
- Submenu with back navigation
- Theme preview on hover
- Custom titlebar color picker (native OS picker) with fill slider (0–100%)
- Auto-contrast text/buttons based on luminance
- All colors via CSS variables

### 4. Hamburger menu ✅
- Three pages: main, themes submenu, titlebar color submenu
- Click-outside to close
- SVG checkmark on active theme

### 5. General polish ✅
- Fluent-style buttons with SVGs (46×32, transparent, hover glow)
- Tooltips on all buttons
- Data files match exe name (`notes.exe` → `notes.config` + `notes.notes`)
- Window centers on first launch
- Hidden attribute preserved on rewrite
- Title text at 80% opacity
- 17 unit tests
- Changelog (0.0.1 → 0.1.0 → 0.1.1)

---

## Next up

### 6. Encrypted notes (password + auto-lock)

**Difficulty:** Hard · **Estimate:** 8h · **Risk:** High

Password-protect the note. Content is AES-encrypted in `sticky.notes`. After idle timeout, the app blurs and locks. Password prompted once per session.

---

#### Phase A: Crypto backend (Rust) ~2h ✅

- [x] Add `aes-gcm`, `argon2`, `rand` crates to `Cargo.toml`
- [x] Add `password_protected: bool`, `password_salt: String`, `lock_timeout_minutes: u32` to Config (defaults: false, "", 5)
- [x] Create `crypto.rs`:
  - [x] `fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]>` — Argon2id
  - [x] `fn encrypt(plaintext: &str, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>)>` — AES-256-GCM, returns (nonce, ciphertext)
  - [x] `fn decrypt(ciphertext: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<String>`
  - [x] `fn generate_salt() -> [u8; 16]`
- [x] Design note file format: `{ "encrypted": true, "nonce_hex": "...", "ciphertext_hex": "..." }`
- [x] Implement `NoteFile` struct for the encrypted format

#### Phase B: Tauri commands ~2h ✅

- [x] `set_password(password: String)` — generate salt, store in config, re-encrypt current note content
- [x] `unlock(password: String)` — derive key, decrypt note, return plaintext. Error if wrong password.
- [x] `remove_password(password: String)` — verify password, decrypt note, disable protection in config
- [x] `change_password(old_pwd: String, new_pwd: String)` — verify old, re-encrypt with new
- [x] Modify `save_note` — if `password_protected`, encrypt before writing
- [x] Modify `load_note` — if `password_protected`, return `{ locked: true }` instead of plaintext
- [x] Add all commands to `generate_handler!`
- [x] Add `lock` command to clear cached encryption key
- [x] Add `AppState` with `Mutex<Option<[u8; 32]>>` for cached key

#### Phase C: Password UI ~1.5h ✅

- [x] **HTML:** Password prompt overlay (`#lock-overlay`) with input field + submit button
- [x] **CSS:** Full-screen overlay, centered card, blur effect on editor when active
- [x] **CSS:** `#editor.locked { filter: blur(8px); pointer-events: none; }`
- [x] **JS:** On `load_note` returns `{ locked: true }` → show overlay
- [x] **JS:** On password submit → call `unlock`, on success → hide overlay, remove blur
- [x] **JS:** On wrong password → show error message, clear input
- [x] **JS:** "Set password" flow: first-time prompt → call `set_password`, re-save note
- [x] **JS:** "Remove password" flow: prompt for current password → call `remove_password`

#### Phase D: Lock timer ~1.5h ✅

- [x] **JS:** `startLockTimer()` — read `config.lock_timeout_minutes`, set timeout
- [x] **JS:** Reset timer on each `editor input` event (when unlocked)
- [x] **JS:** When timer fires → clear decrypted text from memory, call `lockNow()`
- [x] **JS:** `lockNow()` — add `locked` class to editor, show overlay
- [x] **JS:** On unlock → restart timer

#### Phase E: Hamburger menu items ~1h ✅

- [x] **HTML/CSS:** Add menu items in main page:
  - `Set password...` / `Change password...` (shows correct label based on state)
  - `Lock now` (only when unlocked + protected)
  - `Remove password...` (only when protected)
  - Lock timeout: slider (0–60 min) with "Never" label at 0
- [x] **JS:** Toggle visibility of password menu items based on `config.password_protected`
- [x] **JS:** Lock timeout slider → updates `config.lock_timeout_minutes`, `saveConfig()`, restarts timer
- [x] **JS:** "Lock now" → calls `lockNow()` + `closeMenu()`
- [x] **JS:** "Remove password" → prompt modal → `remove_password()` → update menu

#### Phase F: Tests ~1h ✅

- [x] Unit: `encrypt(plaintext)` → `decrypt(ciphertext)` returns original
- [x] Unit: wrong password fails decryption
- [x] Unit: empty password derived key works (Argon2 accepts empty strings)
- [x] Unit: config roundtrip with `password_protected`, `password_salt`, `lock_timeout_minutes`
- [x] Unit: salt length is 16 bytes
- [x] Manual: set password → type → close → reopen → unlock → text matches
- [x] Manual: type → wait idle → blur + lock → enter password → restored, timer reset
- [x] Manual: "Lock now" → immediate blur
- [x] Manual: "Remove password" → note is no longer encrypted on disk
- [x] Manual: lock timeout slider → verify file saves with new value

---

#### add font selection on the hamburger menu, it should at the bottom with submenu of selections
we should keep the font on our codebase(maybe?). because this codebase is for cross platfrom
Inter
Roboto
Open Sans
JetBrains Mono
Fira Code
Cascadia Code
Georgia
Segoe UI
Arial
Source Code Pro

---
### 8. Cloud sync (Google Drive)

**Difficulty:** Hard · **Estimate:** 12h · **Risk:** High

Optional Google Drive sync for `sticky.notes`.

#### Subtasks
- [ ] Add `reqwest` + `oauth2` Rust crates
- [ ] Create Google Cloud project + OAuth client ID
- [ ] Create `sync.rs`: auth flow, token storage, upload/download
- [ ] OAuth via local HTTP listener for redirect
- [ ] Sync on auto-save (if authenticated)
- [ ] Sync on startup (if authenticated, download latest)
- [ ] Conflict resolution: compare timestamps, keep newest
- [ ] UI: sync status indicator, auth button in hamburger menu
- [ ] Token storage in config (encrypted refresh token)

#### Tests
- [ ] Unit: sync config roundtrip
- [ ] Manual: auth → edit → sync → reopen on another machine → synced
- [ ] Manual: offline → edit → online → syncs on next save
