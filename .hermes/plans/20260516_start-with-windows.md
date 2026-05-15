# Plan: "Start with Windows" toggle

## Goal

Add a toggle in the hamburger menu to let the user start the app automatically when Windows boots. Must work as a **portable app** — no installer, no system-wide changes, cleans up after itself.

## Approach

Use the **Windows Run registry key** (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`):
- `HKEY_CURRENT_USER` — per-user, no admin required
- Value: `"Notes"` = `"C:\path\to\notes.exe"` (full exe path)
- On toggle OFF: delete the value
- Clean, standard, used by every major desktop app (Chrome, VS Code, Discord, Slack)

For portability:
- Path is set to the exe's current location when enabled
- If the exe moves, the old path breaks — user just toggles off/on
- No leftover registry keys — we delete on toggle OFF
- `#[cfg(windows)]` FFI with `advapi32` — no new crate deps

## Files to change

### 1. `src-tauri/src/config.rs`
- Add `start_with_windows: bool` with `#[serde(default)]` (default: `false`)

### 2. `src-tauri/src/lib.rs`
- Add `#[tauri::command] fn set_start_with_windows(enabled: bool)` that:
  - Calls helper `util::set_startup_registry(enabled)` on Windows
  - Loads config, sets the flag, saves
  - Fires `diagnostics::event("startup", "Start with Windows enabled/disabled")`
- Register in `generate_handler![]`

### 3. `src-tauri/src/util.rs`
- Add `#[cfg(windows)] pub fn set_startup_registry(enabled: bool)`
  - Uses `extern "system"` FFI to `advapi32.dll`: `RegOpenKeyExW`, `RegSetValueExW`, `RegDeleteValueW`
  - Writes/deletes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Notes` = exe path
  - Silently ignores errors (best-effort, like the hidden file logic)

### 4. `index.html`
- Add in `#menu-page-main` after the font button:
```html
<div class="menu-sep"></div>
<button id="menu-startup">
  <span class="menu-check"></span>
  <span>Start with Windows</span>
</button>
```

### 5. `src/main.js`
- Add `const menuStartup = document.getElementById("menu-startup");`
- Add `function applyStartupState()` — toggles `.on` class on `#menu-startup` from `config.start_with_windows`
- Add click handler:
```js
menuStartup.addEventListener("click", async () => {
  const enabled = !config.start_with_windows;
  await invoke("set_start_with_windows", { enabled });
  config.start_with_windows = enabled;
  applyStartupState();
  closeMenu();
});
```
- Call `applyStartupState()` in `loadConfig()`

### 6. `src/style.css`
- No new styles needed — `.menu-check::after` checkmark and `.on` class already work
- The menu-sep and button follow existing patterns

## Testing

- `util.rs`: add test that the FFI doesn't crash when called with bogus exe path (safety net)
- `config.rs`: add test that `start_with_windows` defaults to `false` and roundtrips
- `lib.rs`: can't unit test the Tauri command directly, but the util function is testable
- After build: manual test — toggle on, check registry with `reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Notes`, toggle off, verify it's gone

## Risks & tradeoffs

- **FFI safety**: registry calls are simple but raw `extern "system"` — `unsafe` block needed
- **Exe moved after toggle**: old path in registry won't work — user must toggle off/on
- **Only Windows**: other platforms silently no-op
- **No validation**: `RegSetValueExW` can fail (permissions, etc.) — we silently ignore the error (matching the pattern in `util.rs`)
- **Two `cfg(windows)` blocks**: one in `util.rs` for the registry helper, one in `lib.rs` for the command call — need to keep `set_start_with_windows` always available (returns `()` on non-Windows)

## Verification

```bash
# After build, run:
reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Notes
# Should show nothing initially
# Toggle ON in app, then:
reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Notes
# Should show path to notes.exe
# Toggle OFF:
reg query HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v Notes
# Should show "ERROR: The system was unable to find the specified registry key or value"
```
