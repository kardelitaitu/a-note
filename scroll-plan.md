# Smooth Scroll Plan

## Goal
Make ALL scrolling feel like macOS — smooth pixel animation with momentum/deceleration:
- Mouse wheel scrolling
- Arrow key cursor navigation (Up/Down)
- Page Up / Page Down
- Home / End

Must work perfectly for **all 10 fonts** (monospace + proportional).

## Approach: "Revert and Animate" technique

The key insight: instead of manually estimating cursor pixel position (which breaks on variable-width fonts and soft wrapping), we **let the browser handle cursor positioning natively**, then intercept the resulting scroll jump and animate it.

### How it works

```
1. User presses Arrow Down
2. Let browser move cursor + scroll natively → scrollTop changes
3. Next animation frame (before paint): capture new scrollTop
4. Revert scrollTop to old value
5. Animate smoothly: old → new with easing
6. Browser renders → user sees smooth scroll
```

No cursor position math needed. Works with any font, any wrap mode, any text content.

### 1. Wheel scrolling — momentum physics

```
wheel event → accumulate velocity → friction decay per frame → deceleration
```

- `deltaY` normalized to pixels regardless of `deltaMode`
- Velocity capped at ±60px/frame max
- Friction: multiply by 0.88 per frame
- Bounce: -0.3 rebound at scroll boundaries
- Stop threshold: |velocity| < 0.5

### 2. Arrow/Page key scrolling — smooth target animation

```
keydown → let browser scroll → rAF: capture target → revert → ease from old to target
```

- Capture `scrollTop` BEFORE key processing
- After browser processes key (in rAF), capture new `scrollTop`
- If different, revert and start 100ms eased animation
- CSS-ready easing: `ease-out` (cubic-bezier)

### 3. Wheel key interception detail

```js
editor.addEventListener("wheel", (e) => {
  if (e.ctrlKey) {
    // Ctrl+Scroll zoom — unchanged, existing handler
    return;
  }

  e.preventDefault();

  let delta = e.deltaY;
  if (e.deltaMode === 1) delta *= LINE_HEIGHT;      // line mode → pixels
  if (e.deltaMode === 2) delta *= editor.clientHeight; // page mode → pixels

  smoothScroll.velocity += delta * 0.3;

  if (!smoothScroll.animating) {
    smoothScroll.animating = true;
    tickScroll();
  }
});
```

### 4. Arrow key interception detail

```js
editor.addEventListener("keydown", (e) => {
  if (["ArrowUp","ArrowDown","PageUp","PageDown","Home","End"].includes(e.key)) {
    const oldTop = editor.scrollTop;

    requestAnimationFrame(() => {
      const newTop = editor.scrollTop;
      if (newTop !== oldTop) {
        // Revert native jump, animate smoothly
        editor.scrollTop = oldTop;
        animateScrollTo(newTop, 100);
      }
    });
  }
});
```

### 5. Animation helpers

```js
let smoothScroll = {
  velocity: 0,
  animating: false,
  animId: null,
  target: null,
  start: null,
  startTime: null,
  duration: null,
};

// For wheel: physics-based momentum
function tickScroll() {
  smoothScroll.velocity *= 0.88;
  editor.scrollTop += smoothScroll.velocity;

  // Bounce at boundaries
  const max = editor.scrollHeight - editor.clientHeight;
  if (editor.scrollTop < 0) {
    editor.scrollTop = 0;
    smoothScroll.velocity *= -0.3;
  } else if (editor.scrollTop > max) {
    editor.scrollTop = max;
    smoothScroll.velocity *= -0.3;
  }

  if (Math.abs(smoothScroll.velocity) < 0.5) {
    smoothScroll.animating = false;
    smoothScroll.velocity = 0;
    return;
  }
  smoothScroll.animId = requestAnimationFrame(tickScroll);
}

// For keys: tweened animation with ease-out
function animateScrollTo(target, duration) {
  const start = editor.scrollTop;
  const startTime = performance.now();

  function tick() {
    const elapsed = performance.now() - startTime;
    const t = Math.min(elapsed / duration, 1);
    const ease = 1 - Math.pow(1 - t, 3); // ease-out cubic
    editor.scrollTop = start + (target - start) * ease;

    if (t < 1) {
      smoothScroll.animId = requestAnimationFrame(tick);
    } else {
      smoothScroll.animating = false;
    }
  }
  smoothScroll.animating = true;
  tick();
}
```

### 6. Scrollbar styling (CSS)

Already have custom thin scrollbar. Add `scroll-behavior: smooth` as a fallback (won't affect wheel/key scroll but helps programmatic scroll from JS).

### 7. Edge cases

| Case | Handling |
|------|----------|
| **Ctrl+Wheel zoom** | Unchanged — `e.ctrlKey` check returns early |
| **Mouse click to position cursor** | Native scroll — no animation |
| **Text selection drag** | Native scroll — too complex to intercept |
| **Typing characters (not arrows)** | No scroll animation — native |
| **Scrollbar drag** | Native — not intercepted |
| **Very fast continuous wheel** | Velocity capped at ±60/frame |
| **Proportional fonts (Inter, Roboto, etc.)** | ✅ Works — browser knows cursor position |
| **Soft-wrapped text** | ✅ Works — browser handles wrapping |
| **Mixed font sizes** | ✅ Works — browser handles all |
| **Resize window** | No animation — direct |
| **wheel event missing e.preventDefault support** | Passive listener check needed |

### 8. Implementation Order

1. Add `smoothScroll` state + `tickScroll()` momentum loop
2. Add wheel event interceptor (reuse existing handler, extend it)
3. Add keydown interceptor for arrow/page keys
4. Add `animateScrollTo()` tween function for keyboard scroll
5. Test with Cascadia Code (mono), Inter (sans), Roboto, Open Sans
6. Tune parameters: friction (0.88), velocity cap (60), duration (100ms)

### 9. Better Anti-Aliasing

On Windows, ClearType can cause color fringing on dark backgrounds. Add CSS to force grayscale AA for cleaner text — especially noticeable with the 10 available fonts at small sizes in a dark editor.

### CSS additions (in `style.css`)

```css
/* Better font rendering on Windows dark theme */
body, textarea, #editor, #menu-dropdown button {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  text-rendering: optimizeLegibility;
}
```

- `-webkit-font-smoothing: antialiased` — Grayscale AA instead of ClearType subpixel
- `-moz-osx-font-smoothing: grayscale` — macOS equivalent
- `text-rendering: optimizeLegibility` — Better kerning and ligature support

### 10. Files Modified

| File | Change |
|------|--------|
| `src/main.js` | +SmoothScroll engine (~60 lines), wheel handler update, new keydown handler |
| `src/style.css` | Anti-aliasing CSS + maybe `scroll-behavior: smooth` as CSS fallback |

### 11. Tuning Parameters

| Param | Default | Effect |
|-------|---------|--------|
| Friction | 0.88 | Higher = longer glide |
| Velocity cap | 60 | Max px/frame from wheel |
| Wheel dampen | 0.3 | Input sensitivity |
| Bounce rebound | 0.0 | No bounce — clean stop at boundary |
| Stop threshold | 0.5 | px/frame below which scrolling stops |
| Key scroll duration | 100 | ms for arrow/page key animation |
| Key ease | cubic-out | `1-(1-t)^3` |

### 11. Not In Scope

- Scrollbar drag animation — native behavior
- Touchpad gestures — OS handles these via wheel events
- Click-to-position cursor animation — too complex, native is fine
