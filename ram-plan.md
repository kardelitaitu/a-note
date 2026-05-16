# RAM Usage Optimization Plan

## Current Usage

| Process | RAM | Controllable? |
|---------|-----|---------------|
| notes.exe (Rust backend) | 4.3 MB | ⚠️ Minimal — Rust is already lean |
| WebView2 Manager (shared) | 70 MB | ❌ System process, shared across all WebView2 apps |
| WebView2: Sticky Notes | 35 MB | ✅ **This is what we can optimize** |
| **Total** | **109 MB** | |

The WebView2 Manager (70MB) is a Microsoft component that runs as long as ANY WebView2 app is open. It caches runtime resources, GPU buffers, and network data shared across all WebView2 applications. We cannot control it — it shuts down when the last WebView2 app closes.

The **35MB** for the Sticky Notes webview instance is where we can make changes.

---

## Where the RAM Goes (estimated breakdown for 35MB)

| Component | Est. RAM | Why |
|-----------|----------|-----|
| DOM tree (buttons, menus, overlays) | ~3 MB | 15 theme buttons + 10 font buttons + menu items + overlays. All created at init, never removed. |
| CSSOM (15 themes × ~30 vars) | ~4 MB | All 15 theme blocks parsed and stored in CSSOM. Only 1 is active, but 14 are dead weight. |
| V8 JavaScript heap | ~8 MB | Runtime, closures, `@tauri-apps/api` modules, IPC bridge, event listeners. |
| Font data (Google Fonts CDN) | ~5 MB | 7 web fonts loaded at startup. Each font file is ~300KB–1MB decoded. |
| `editor.value` (note text) | ~2 MB | Full note text in memory (JS string + DOM textarea value = 2x). |
| GPU buffers / compositor layers | ~8 MB | Backdrop-filter GPU allocation, compositing layers for transitions, layers for each animated element. |
| Miscellaneous (IPC buffers, etc.) | ~5 MB | |

---

## Optimization Strategies (ordered by impact)

### 1. Defer Google Fonts — save ~5MB

**Problem:** All 7 Google Fonts are loaded at startup via a single CSS `@import` in `fonts.css`. Each loaded font consumes ~300KB–1MB for the decoded glyph data. Even fonts not currently in use stay in memory.

**Solution:** Load fonts on-demand. Only download the font data when the user selects that font from the submenu. Cache the loaded font in a `Set` so re-selecting doesn't re-download.

- Create a `fonts.css` with just `@font-face` declarations (not `@import`)
- In `initFonts()`, don't preload fonts
- In the font selection `click` handler, dynamically inject the font stylesheet
- CSS: Use generic `monospace`/`sans-serif` fallbacks for unloaded fonts

**Trade-off:** First-time font selection will have a brief flash as the font downloads. Mitigate by showing a brief toast or accepting the one-time delay.

### 2. Lazy-render theme/font submenu content — save ~2MB

**Problem:** `initThemes()` creates 15 `<button>` elements with full DOM structure (label spans, check-svg SVGs, event listeners). `initFonts()` creates 10 more. All these DOM nodes exist in the menu tree forever, even though the submenu is rarely opened.

**Solution:** Don't create theme/font buttons at init time. Create them lazily when the user first opens that submenu.

- Replace `initThemes()` with a function that builds buttons on first `show-themes` class toggle
- Same for fonts
- Cache the built DOM fragment so re-opening doesn't rebuild

**Trade-off:** First time the user opens Theme submenu, there's a slight delay (frame or two) while 15 buttons are created. After that, it's instant.

### 3. Use `requestAnimationFrame` for overlay transitions — save ~3MB GPU

**Problem:** The lock/password overlays have `transition: opacity 0.2s ease, transform 0.2s ease`. The browser creates compositor layers for these animated elements, reserving GPU memory. With 2 overlays + 2 cards, this is ~4 compositing layers.

**Solution:** Move overlay transitions out of CSS and into JS using `requestAnimationFrame`. Remove the CSS `transition` property, and animate opacity/transform manually in JS when showing/hiding overlays.

- Remove `transition` from `#lock-overlay`, `#pwd-overlay`, `#lock-card`, `#pwd-card`
- In the `show` / `hide` functions, use a simple JS animation loop (0.2s ease-out)
- The browser doesn't allocate compositor layers speculatively — only during the actual animation

**Trade-off:** Slightly more JS code (~20 lines). Slightly more CPU during overlay transitions. But the overlays are rarely shown/hidden.

### 4. Remove `backdrop-filter` entirely — save ~3MB GPU

**Problem:** `backdrop-filter: blur(6px)` requires the browser to maintain a separate GPU texture for the background behind the overlay, even when the overlay is `visibility: hidden` (the property is still parsed).

**Solution:** Replace blur with a solid semi-transparent background (no blur). The visual difference is minimal on a compact 300×400 window.

- Remove `backdrop-filter` from both overlay states
- Keep `background: rgba(0, 0, 0, 0.55);` for the dark overlay effect

**Trade-off:** The lock/password overlays will no longer have the blur effect — they'll be a solid dark overlay. This is a visual regression but saves significant GPU memory.

### 5. Strip unused DOM on close — save ~1MB

**Problem:** When the menu closes, the dropdown is hidden but all child DOM nodes (theme buttons, font buttons, SVGs) remain in memory.

**Solution:** When the menu is closed after the user has interacted with submenus, detach the theme/font button containers and recreate them on next open. Or simpler: just remove the `#menu-themes` and `#menu-fonts` children when the menu closes and recreate on demand.

**Trade-off:** Slight recreation cost on next open. But this is a one-time cost that frees memory.

### 6. Trim CSS theme blocks — save ~1MB

**Problem:** Each of the 15 theme blocks defines ~30 CSS custom properties. Parsing 450+ declarations takes CSSOM space.

**Solution:** Reduce the number of themes in the app (e.g., keep 8 popular ones, remove exotic ones). Or generate themes from a base template using JS rather than static CSS.

**Trade-off:** Losing 7 themes that someone might use.

### 7. Remove unused `@tauri-apps/api` imports — save ~500KB

**Problem:** Multiple async imports of `@tauri-apps/api/window` and `@tauri-apps/api/core` create separate module instances.

**Solution:** Import once at the top of main.js and reuse.

```js
// Current (bad — imports every time it's used):
const { getCurrentWindow } = await import("@tauri-apps/api/window");

// Better — import once and cache:
import { invoke } from "@tauri-apps/api/core";
```

**Trade-off:** Need to verify that top-level ESM imports work in the Vite build.

---

## Execution Order

Most impactful first:

1. **Defer Google Fonts** (~5MB saved)
2. **Remove backdrop-filter** (~3MB GPU saved)  
3. **JS animations instead of CSS transitions** (~3MB GPU saved)
4. **Lazy-render theme/font submenus** (~2MB saved)
5. **Strip unused DOM on menu close** (~1MB saved)
6. **Optimize @tauri-apps imports** (~500KB saved)
7. **Trim CSS themes** (~1MB saved, visual trade-off)

## Expected Outcome

| After optimization | RAM |
|--------------------|-----|
| notes.exe | ~4 MB (unchanged) |
| WebView2 Manager | ~70 MB (unchanged — system) |
| WebView2: Sticky Notes | ~20 MB (from 35 MB) |
| **Total** | **~94 MB** |
