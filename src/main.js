import { invoke } from "@tauri-apps/api/core";

const editor = document.getElementById("editor");
const titleText = document.getElementById("title-text");
const btnPin = document.getElementById("btn-pin");
const btnMin = document.getElementById("btn-minimize");
const btnClose = document.getElementById("btn-close");
const btnMenu = document.getElementById("btn-menu");
const menuDropdown = document.getElementById("menu-dropdown");
const menuWordwrap = document.getElementById("menu-wordwrap");
const menuStartup = document.getElementById("menu-startup");
let config = { width: 300, height: 400, left: 100, top: 100, font_size: 14, always_on_top: true, word_wrap: false, theme: "dark", titlebar_color: "", titlebar_fill: 100 };

// ── Toast notification ─────────────────────────────────
let lastToastMsg = "";
let lastToastTime = 0;

function showToast(msg, isError) {
  const bar = document.getElementById("toast-bar");
  if (!bar) return;
  // Cooldown: skip if same message shown in last 2s (prevents spam on autosave)
  const now = Date.now();
  if (msg === lastToastMsg && now - lastToastTime < 2000) return;
  lastToastMsg = msg;
  lastToastTime = now;
  bar.textContent = msg;
  bar.className = isError ? "error" : "";
  bar.classList.remove("hidden");
  setTimeout(() => bar.classList.add("hidden"), 3000);
}

// Lock state
let decryptedText = "";
let lockTimer = null;
let isLocked = false;

const lockOverlay = document.getElementById("lock-overlay");
const lockInput = document.getElementById("lock-input");
const lockSubmit = document.getElementById("lock-submit");
const lockError = document.getElementById("lock-error");

const pwdOverlay = document.getElementById("pwd-overlay");
const pwdTitle = document.getElementById("pwd-title");
const pwdInput = document.getElementById("pwd-input");
const pwdError = document.getElementById("pwd-error");
const pwdConfirm = document.getElementById("pwd-confirm");
const pwdCancel = document.getElementById("pwd-cancel");
const pwdConfirmInput = document.getElementById("pwd-confirm-input");

const themes = [
  { id: "dark", label: "Dark" },
  { id: "dark-black", label: "Dark black" },
  { id: "dark-blue", label: "Dark blue" },
  { id: "dark-choco", label: "Dark choco" },
  { id: "dracula", label: "Dracula" },
  { id: "monokai", label: "Monokai" },
  { id: "nord", label: "Nord" },
  { id: "solarized-dark", label: "Solarized dark" },
  { id: "gruvbox-dark", label: "Gruvbox dark" },
  { id: "catppuccin", label: "Catppuccin" },
  { id: "solarized-light", label: "Solarized light" },
  { id: "gruvbox-light", label: "Gruvbox light" },
  { id: "light-blue", label: "Light blue" },
  { id: "light-orange", label: "Light orange" },
  { id: "light", label: "Light" },
];

async function loadConfig() {
  try {
    config = await invoke("load_config");
    editor.style.fontSize = config.font_size + "px";
    editor.style.whiteSpace = config.word_wrap ? "pre-wrap" : "pre";
    applyTheme();
    applyPinState();
    applyWordWrapState();
    applyStartupState();
    applyFont();
  } catch (e) {
    showToast("Failed to load settings", true);
  }
}

function applyPinState() {
  btnPin.className = config.always_on_top ? "active" : "inactive";
}

function applyTheme() {
  document.body.className = "theme-" + config.theme;
  document.querySelectorAll("#menu-themes button").forEach((btn) => {
    btn.className = btn.dataset.theme === config.theme ? "active" : "";
  });
  applyTitlebarColor();
}

function applyTitlebarColor() {
  if (config.titlebar_color) {
    const r = parseInt(config.titlebar_color.slice(1, 3), 16);
    const g = parseInt(config.titlebar_color.slice(3, 5), 16);
    const b = parseInt(config.titlebar_color.slice(5, 7), 16);
    const a = (config.titlebar_fill || 100) / 100;
    document.body.style.setProperty("--titlebar-bg", `rgba(${r},${g},${b},${a})`);
    const lum = 0.299 * r + 0.587 * g + 0.114 * b;
    const isLight = lum > 140;
    document.body.style.setProperty("--title-text", isLight ? "rgba(0,0,0,0.8)" : "rgba(255,255,255,0.8)");
    document.body.style.setProperty("--btn-color", isLight ? "#444" : "#ccc");
    document.body.style.setProperty("--btn-hover-text", isLight ? "#000" : "#fff");
    document.body.style.setProperty("--btn-hover-bg", isLight ? "rgba(0,0,0,0.08)" : "rgba(255,255,255,0.08)");
    document.body.style.setProperty("--btn-active-bg", isLight ? "rgba(0,0,0,0.12)" : "rgba(255,255,255,0.12)");
    document.body.style.setProperty("--title-sep", isLight ? "rgba(0,0,0,0.08)" : "rgba(255,255,255,0.06)");
  } else {
    document.body.style.removeProperty("--titlebar-bg");
    document.body.style.removeProperty("--title-text");
    document.body.style.removeProperty("--btn-color");
    document.body.style.removeProperty("--btn-hover-text");
    document.body.style.removeProperty("--btn-hover-bg");
    document.body.style.removeProperty("--btn-active-bg");
    document.body.style.removeProperty("--title-sep");
  }
  const swatch = document.getElementById("titlebar-swatch");
  if (swatch) {
    swatch.style.background = config.titlebar_color || "var(--titlebar-bg)";
  }
}

function initThemes() {
  const container = document.getElementById("menu-themes");
  themes.forEach((t) => {
    const btn = document.createElement("button");
    btn.dataset.theme = t.id;
    btn.innerHTML = `
      <span>${t.label}</span>
      <span class="check-svg"><svg width="10" height="10" viewBox="0 0 10 10"><path d="M2 5l2 2 4-4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg></span>`;
    btn.addEventListener("mouseenter", () => {
      document.body.className = "theme-" + t.id;
    });
    btn.addEventListener("mouseleave", () => {
      document.body.className = "theme-" + config.theme;
    });
    btn.addEventListener("click", () => {
      config.theme = t.id;
      applyTheme();
      saveConfig();
      closeMenu();
      showToast("Theme saved");
    });
    container.appendChild(btn);
  });
}

const fonts = [
  { id: "Cascadia Code", label: "Cascadia Code", type: "mono" },
  { id: "JetBrains Mono", label: "JetBrains Mono", type: "mono" },
  { id: "Fira Code", label: "Fira Code", type: "mono" },
  { id: "Source Code Pro", label: "Source Code Pro", type: "mono" },
  { id: "Inter", label: "Inter", type: "sans" },
  { id: "Roboto", label: "Roboto", type: "sans" },
  { id: "Open Sans", label: "Open Sans", type: "sans" },
  { id: "Segoe UI", label: "Segoe UI", type: "sans" },
  { id: "Arial", label: "Arial", type: "sans" },
  { id: "Georgia", label: "Georgia", type: "serif" },
];

function applyFont() {
  const family = config.font_family || "Cascadia Code";
  document.body.style.setProperty("--font-family", family + ", monospace");
  document.querySelectorAll("#menu-fonts button").forEach((btn) => {
    btn.className = btn.dataset.font === family ? "active" : "";
  });
}

function initFonts() {
  const container = document.getElementById("menu-fonts");
  fonts.forEach((f) => {
    const btn = document.createElement("button");
    btn.dataset.font = f.id;
    btn.innerHTML = `
      <span>${f.label}</span>
      <span class="check-svg"><svg width="10" height="10" viewBox="0 0 10 10"><path d="M2 5l2 2 4-4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/></svg></span>`;
    btn.addEventListener("mouseenter", () => {
      document.body.style.setProperty("--font-family", f.id + ", monospace");
    });
    btn.addEventListener("mouseleave", () => {
      const family = config.font_family || "Cascadia Code";
      document.body.style.setProperty("--font-family", family + ", monospace");
    });
    btn.addEventListener("click", () => {
      config.font_family = f.id;
      applyFont();
      saveConfig();
      closeMenu();
      showToast("Font Saved");
    });
    container.appendChild(btn);
  });
}

function applyWordWrapState() {
  menuWordwrap.className = config.word_wrap ? "on" : "";
}

function applyStartupState() {
  menuStartup.className = config.start_with_windows ? "on" : "";
}

async function saveConfig() {
  try {
    await invoke("save_config", { cfg: config });
  } catch (e) {
    showToast("Failed to save settings", true);
  }
}

// ── Note load / save ────────────────────────────────────────────────────

async function loadNote() {
  try {
    const data = await invoke("load_note");
    if (data.locked) {
      // Note is encrypted — show lock overlay
      isLocked = true;
      editor.value = "";
      editor.classList.add("locked");
      showLockOverlay();
    } else {
      isLocked = false;
      editor.classList.remove("locked");
      editor.value = data.text || "";
      editor.focus();
      const pos = Math.min(data.cursor_pos || 0, (data.text || "").length);
      setTimeout(() => {
        editor.setSelectionRange(pos, pos);
        editor.scrollTop = data.scroll_top || 0;
      }, 0);
      startLockTimer();
    }
  } catch (e) {
    showToast("Failed to load note", true);
  }
}

async function saveNote() {
  if (isLocked) return; // Don't save while locked
  try {
    await invoke("save_note", {
      note: {
        text: editor.value,
        cursor_pos: editor.selectionStart,
        scroll_top: editor.scrollTop,
      },
    });
    showToast("Note Autosaved");
  } catch (e) {
    showToast("Failed to save note", true);
  }
}

// Auto-save 5s after last edit
let saveTimer;
editor.addEventListener("input", () => {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNote, 2000);
  resetLockTimer(); // Reset lock timer on user activity
});

// Ctrl+Scroll to zoom
editor.addEventListener("wheel", (e) => {
  if (e.ctrlKey) {
    e.preventDefault();
    const step = 1.1;
    let size = parseFloat(editor.style.fontSize) || config.font_size;
    if (e.deltaY < 0) {
      size = Math.min(size * step, 72);
    } else {
      size = Math.max(size / step, 8);
    }
    editor.style.fontSize = size + "px";
    config.font_size = Math.round(size);
    saveConfig();
    return;
  }

  // Smooth scroll with momentum
  e.preventDefault();
  let delta = e.deltaY;
  if (e.deltaMode === 1) delta *= 18;       // lines → pixels
  else if (e.deltaMode === 2) delta *= editor.clientHeight; // pages → pixels
  ss.velocity += delta * 0.3;
  ss.velocity = Math.max(-60, Math.min(60, ss.velocity));
  if (!ss.animating) { ss.animating = true; rAF(tickScroll); }
}, { passive: false });

// ── Smooth Scroll Engine ─────────────────────────────────

let ss = {
  velocity: 0,
  animating: false,
  animId: null,
};

const rAF = requestAnimationFrame.bind(window);

function tickScroll() {
  ss.velocity *= 0.88;
  let top = editor.scrollTop + ss.velocity;
  const maxScroll = editor.scrollHeight - editor.clientHeight;
  if (top < 0) { top = 0; ss.velocity = 0; }
  else if (top > maxScroll) { top = maxScroll; ss.velocity = 0; }
  editor.scrollTop = top;
  if (Math.abs(ss.velocity) < 0.5) {
    ss.animating = false;
    ss.velocity = 0;
    return;
  }
  ss.animId = rAF(tickScroll);
}

function animateScrollTo(target, duration) {
  const start = editor.scrollTop;
  const startTime = performance.now();

  function tick() {
    const elapsed = performance.now() - startTime;
    const t = Math.min(elapsed / duration, 1);
    const ease = 1 - Math.pow(1 - t, 3); // ease-out cubic
    editor.scrollTop = start + (target - start) * ease;
    if (t < 1) ss.animId = rAF(tick);
    else ss.animating = false;
  }

  // Stop any running wheel momentum first
  if (ss.animId) cancelAnimationFrame(ss.animId);
  ss.velocity = 0;
  ss.animating = true;
  tick();
}

// ── Smooth scroll on arrow/page keys ─────────────────────

const SCROLL_KEYS = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End"]);

editor.addEventListener("keydown", (e) => {
  if (!SCROLL_KEYS.has(e.key)) return;
  // Skip repeats (holding key) — let native scroll take over for responsiveness
  if (e.repeat) return;

  // Stop any running wheel momentum
  if (ss.animId) { cancelAnimationFrame(ss.animId); ss.animId = null; }
  ss.velocity = 0;

  // Let browser process the key first (moves cursor + scrolls natively)
  const oldTop = editor.scrollTop;
  rAF(() => {
    const newTop = editor.scrollTop;
    if (newTop !== oldTop) {
      editor.scrollTop = oldTop; // revert the instant jump
      animateScrollTo(newTop, 100);
    }
  });
});

// Double-click to select line
editor.addEventListener("dblclick", () => {
  const scrollTop = editor.scrollTop;
  const text = editor.value;
  const caret = editor.selectionStart;

  let start = text.lastIndexOf("\n", caret - 1);
  if (start === -1) start = 0;
  else start += 1;

  let end = text.indexOf("\n", caret);
  if (end === -1) end = text.length;

  editor.setSelectionRange(start, end);
  editor.scrollTop = scrollTop;
});

// ── Lock overlay ────────────────────────────────────────────────────────

function showLockOverlay() {
  lockOverlay.classList.remove("hidden");
  lockError.classList.add("hidden");
  lockInput.value = "";
  setTimeout(() => lockInput.focus(), 50);
}

function hideLockOverlay() {
  lockOverlay.classList.add("hidden");
  lockError.classList.add("hidden");
}

// Clicking anywhere on the lock overlay focuses the password input
lockOverlay.addEventListener("click", () => {
  lockInput.focus();
});

async function handleUnlock() {
  const pwd = lockInput.value;
  if (!pwd) return;

  lockSubmit.disabled = true;
  try {
    const result = await invoke("unlock", { password: pwd });
    if (result.ok) {
      isLocked = false;
      editor.classList.remove("locked");
      editor.value = result.text || "";
      decryptedText = result.text || "";
      hideLockOverlay();
      editor.focus();
      const pos = Math.min(result.cursor_pos || 0, (result.text || "").length);
      setTimeout(() => {
        editor.setSelectionRange(pos, pos);
        editor.scrollTop = result.scroll_top || 0;
      }, 0);
      startLockTimer();
      updatePasswordMenuItems();
    }
  } catch (e) {
    lockError.textContent = typeof e === "string" ? e : "Wrong password. Try again.";
    lockError.classList.remove("hidden");
    lockInput.value = "";
    lockInput.focus();
  } finally {
    lockSubmit.disabled = false;
  }
}

lockSubmit.addEventListener("click", handleUnlock);
lockInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") handleUnlock();
});

// Close button on lock overlay — save config and close
document.getElementById("lock-close").addEventListener("click", async () => {
  await saveConfig();
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
});

// ── Lock timer (Phase D) ──────────────────────────────────────────────────

function startLockTimer() {
  clearLockTimer();
  if (!config.password_protected || isLocked) return;
  const timeoutMs = (config.lock_timeout_minutes || 5) * 60 * 1000;
  if (timeoutMs <= 0) return; // "Never" setting
  lockTimer = setTimeout(() => {
    lockNow();
  }, timeoutMs);
}

function resetLockTimer() {
  if (!config.password_protected || isLocked) return;
  clearLockTimer();
  startLockTimer();
}

function clearLockTimer() {
  if (lockTimer) {
    clearTimeout(lockTimer);
    lockTimer = null;
  }
}

async function lockNow() {
  clearLockTimer();
  decryptedText = "";
  editor.value = "";
  isLocked = true;
  editor.classList.add("locked");
  showLockOverlay();
  try {
    await invoke("lock");
  } catch (e) {
    showToast("Failed to lock", true);
  }
}

// ── Password setup overlay (set / change / remove) ──────────────────────

let pwdMode = "";
let pwdCallback = null;

function showPwdOverlay(title, placeholder, mode, callback) {
  pwdTitle.textContent = title;
  pwdInput.placeholder = placeholder || "Password";
  pwdMode = mode;
  pwdCallback = callback;
  pwdError.classList.add("hidden");
  pwdError.textContent = "";
  pwdInput.value = "";
  pwdConfirmInput.value = "";
  pwdConfirmInput.style.display = "block";
  pwdOverlay.classList.remove("hidden");

  // Show/hide the auto-lock slider and confirm field based on mode
  const timeoutRow = document.getElementById("pwd-timeout-row");
  if (mode === "remove") {
    timeoutRow.style.display = "none";
    pwdConfirmInput.style.display = "none";
  } else {
    timeoutRow.style.display = "flex";
    // Sync slider with current config value
    const slider = document.getElementById("pwd-lock-timeout");
    slider.value = config.lock_timeout_minutes || 5;
    document.getElementById("pwd-lock-timeout-label").textContent = lockTimeoutLabel(config.lock_timeout_minutes || 5);
  }

  setTimeout(() => pwdInput.focus(), 50);
}

function hidePwdOverlay() {
  pwdOverlay.classList.add("hidden");
  pwdMode = "";
  pwdCallback = null;
}

pwdConfirm.addEventListener("click", async () => {
  const pwd = pwdInput.value;
  if (!pwd) {
    pwdError.textContent = "Password cannot be empty.";
    pwdError.classList.remove("hidden");
    pwdConfirm.disabled = false;
    return;
  }

  // For set/change modes, validate confirm password matches
  if (pwdMode !== "remove" && pwdConfirmInput.value !== pwd) {
    pwdError.textContent = "Passwords do not match.";
    pwdError.classList.remove("hidden");
    pwdConfirm.disabled = false;
    return;
  }

  pwdConfirm.disabled = true;
  try {
    if (pwdMode === "set") {
      await invoke("set_password", { password: pwd });
      await loadConfig();
      updatePasswordMenuItems();
      hidePwdOverlay();
      startLockTimer();
      showToast("Note Encrypted");
    } else if (pwdMode === "change") {
      if (pwdCallback) pwdCallback(pwd);
      hidePwdOverlay();
    } else if (pwdMode === "remove") {
      await invoke("remove_password", { password: pwd });
      await loadConfig();
      updatePasswordMenuItems();
      clearLockTimer();
      hidePwdOverlay();
    }
  } catch (e) {
    pwdError.textContent = typeof e === "string" ? e : "Operation failed.";
    pwdError.classList.remove("hidden");
  } finally {
    pwdConfirm.disabled = false;
  }
});

pwdCancel.addEventListener("click", hidePwdOverlay);
pwdInput.addEventListener("keydown", (e) => {
  if (e.key === "Enter") pwdConfirm.click();
});

// Lock timeout slider in password overlay
document.getElementById("pwd-lock-timeout").addEventListener("input", (e) => {
  const val = parseInt(e.target.value);
  config.lock_timeout_minutes = val;
  document.getElementById("pwd-lock-timeout-label").textContent = lockTimeoutLabel(val);
  saveConfig();
});

// ── Hamburger menu ──────────────────────────────────────────────────────

btnMenu.addEventListener("click", (e) => {
  e.stopPropagation();
  const wasOpen = menuDropdown.classList.contains("open");
  menuDropdown.classList.toggle("open");
  // If closing from a submenu, delay submenu reset until fade-out ends
  if (wasOpen) {
    setTimeout(() => {
      menuDropdown.classList.remove("show-themes", "show-titlebar", "show-fonts");
    }, 150);
  } else {
    menuDropdown.classList.remove("show-themes", "show-titlebar", "show-fonts");
  }
});

function closeMenu() {
  if (!menuDropdown.classList.contains("open")) return;
  menuDropdown.classList.remove("open");
  document.body.className = "theme-" + config.theme;
  // Wait for dropdown fade-out (0.15s) before resetting submenu page,
  // otherwise the main menu flashes during the transition
  setTimeout(() => {
    menuDropdown.classList.remove("show-themes", "show-titlebar", "show-fonts");
  }, 150);
}

// Theme submenu navigation
document.getElementById("menu-theme-btn").addEventListener("click", () => {
  menuDropdown.classList.add("show-themes");
});

document.getElementById("menu-theme-back").addEventListener("click", () => {
  menuDropdown.classList.remove("show-themes");
});

// Font submenu navigation
document.getElementById("menu-font-btn").addEventListener("click", () => {
  menuDropdown.classList.add("show-fonts");
});

document.getElementById("menu-font-back").addEventListener("click", () => {
  menuDropdown.classList.remove("show-fonts");
});

// Titlebar color submenu navigation
document.getElementById("menu-titlebar-btn").addEventListener("click", () => {
  menuDropdown.classList.add("show-titlebar");
});

document.getElementById("menu-titlebar-back").addEventListener("click", () => {
  menuDropdown.classList.remove("show-titlebar");
});

// Titlebar color picker
document.getElementById("menu-color-row").addEventListener("click", () => {
  document.getElementById("titlebar-color-picker").click();
});

document.getElementById("titlebar-color-picker").addEventListener("input", (e) => {
  config.titlebar_color = e.target.value;
  applyTitlebarColor();
  saveConfig();
  invoke("update_tray_color", { color: e.target.value }).catch(() => {});
});

// Titlebar fill slider
document.getElementById("titlebar-fill-slider").addEventListener("input", (e) => {
  config.titlebar_fill = parseInt(e.target.value);
  document.getElementById("titlebar-fill-value").textContent = config.titlebar_fill + "%";
  applyTitlebarColor();
  saveConfig();
});

// Reset titlebar color to default
document.getElementById("menu-titlebar-default").addEventListener("click", () => {
  config.titlebar_color = "";
  config.titlebar_fill = 100;
  document.getElementById("titlebar-fill-slider").value = 100;
  document.getElementById("titlebar-fill-value").textContent = "100%";
  document.getElementById("titlebar-color-picker").value = "#000000";
  applyTitlebarColor();
  saveConfig();
  closeMenu();
  showToast("Titlebar Color Changed");
  // Reset tray icon to default blue
  invoke("update_tray_color", { color: "#5dade2" }).catch(() => {});
});

// Word wrap toggle
menuWordwrap.addEventListener("click", () => {
  config.word_wrap = !config.word_wrap;
  editor.style.whiteSpace = config.word_wrap ? "pre-wrap" : "pre";
  applyWordWrapState();
  saveConfig();
  closeMenu();
});

// Start with Windows toggle
menuStartup.addEventListener("click", () => {
  const enabled = !config.start_with_windows;
  invoke("set_start_with_windows", { enabled }).then(() => {
    config.start_with_windows = enabled;
    applyStartupState();
    showToast(enabled ? "Autostart Enabled" : "Autostart Disabled");
  }).catch((e) => {
    showToast("Failed to update Auto Start", true);
  });
  closeMenu();
});

// Close menu on click outside
document.addEventListener("click", (e) => {
  if (!document.getElementById("menu-area").contains(e.target)) {
    closeMenu();
  }
});

// Pin / always-on-top toggle
btnPin.addEventListener("click", async () => {
  config.always_on_top = !config.always_on_top;
  applyPinState();
  saveConfig();
  showToast(config.always_on_top ? "Always On Top Enabled" : "Always On Top Disabled");
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().setAlwaysOnTop(config.always_on_top);
});

// Minimize
btnMin.addEventListener("click", async () => {
  await saveNote();
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().minimize();
});

// Close — save then close
btnClose.addEventListener("click", async () => {
  await saveNote();
  await saveConfig();
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
});

// Track window resize/move to save config
async function trackWindow() {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const win = getCurrentWindow();

  await win.onResized(async () => {
    const size = await win.outerSize();
    config.width = size.width;
    config.height = size.height;
    saveConfig();
  });

  await win.onMoved(async () => {
    const pos = await win.outerPosition();
    if (pos.x < 0 || pos.y < 0) return;
    config.left = pos.x;
    config.top = pos.y;
    saveConfig();
  });
}

// ── Password menu items (Phase E) ───────────────────────────────────────

function initPasswordMenu() {
  const menuMain = document.getElementById("menu-page-main");

  // Separator before password section
  const sep = document.createElement("div");
  sep.className = "menu-sep";
  sep.id = "menu-pwd-sep";
  menuMain.appendChild(sep);

  // Set/Change password
  const btnSetPwd = document.createElement("button");
  btnSetPwd.id = "menu-set-pwd";
  btnSetPwd.innerHTML = `<span></span><span>Set password...</span>`;
  btnSetPwd.addEventListener("click", () => {
    closeMenu();
    if (config.password_protected) {
      // Change password flow
      showPwdOverlay("Current password", "Current password", "change", async (oldPwd) => {
        showPwdOverlay("New password", "New password", "set", async (newPwd) => {
          try {
            await invoke("change_password", { oldPwd, newPwd });
            await loadConfig();
            updatePasswordMenuItems();
            startLockTimer();
          } catch (e) {
            pwdError.textContent = typeof e === "string" ? e : "Failed to change password.";
            pwdError.classList.remove("hidden");
          }
        });
      });
    } else {
      showPwdOverlay("Set password", "Password", "set", null);
    }
  });
  menuMain.appendChild(btnSetPwd);

  // Lock now
  const btnLockNow = document.createElement("button");
  btnLockNow.id = "menu-lock-now";
  btnLockNow.innerHTML = `<span></span><span>Lock now</span>`;
  btnLockNow.addEventListener("click", () => {
    closeMenu();
    lockNow();
  });
  menuMain.appendChild(btnLockNow);

  // Remove password
  const btnRemovePwd = document.createElement("button");
  btnRemovePwd.id = "menu-remove-pwd";
  btnRemovePwd.innerHTML = `<span></span><span>Remove password</span>`;
  btnRemovePwd.addEventListener("click", () => {
    closeMenu();
    showPwdOverlay("Remove password", "Current password", "remove", null);
  });
  menuMain.appendChild(btnRemovePwd);

  updatePasswordMenuItems();
}

function lockTimeoutLabel(minutes) {
  if (minutes <= 0) return "Never";
  if (minutes < 60) return minutes + "m";
  return Math.floor(minutes / 60) + "h";
}

function updatePasswordMenuItems() {
  const btnSetPwd = document.getElementById("menu-set-pwd");
  const btnLockNow = document.getElementById("menu-lock-now");
  const btnRemovePwd = document.getElementById("menu-remove-pwd");
  const pwdSep = document.getElementById("menu-pwd-sep");

  if (!btnSetPwd) return;

  if (config.password_protected) {
    btnSetPwd.innerHTML = `<span></span><span>Change password...</span>`;
    btnLockNow.style.display = "flex";
    btnRemovePwd.style.display = "flex";
  } else {
    btnSetPwd.innerHTML = `<span></span><span>Set password...</span>`;
    btnLockNow.style.display = "none";
    btnRemovePwd.style.display = "none";
  }
}

// ── Init ─────────────────────────────────────────────────────────────────

(async () => {
  const name = await invoke("get_app_name");
  titleText.textContent = name;

  initThemes();
  initFonts();
  initPasswordMenu();
  await loadConfig();
  document.getElementById("titlebar-fill-slider").value = config.titlebar_fill;
  document.getElementById("titlebar-fill-value").textContent = config.titlebar_fill + "%";
  if (config.titlebar_color) {
    document.getElementById("titlebar-color-picker").value = config.titlebar_color;
  }
  await loadNote();
  await trackWindow();

  // Save config + note before unload
  window.addEventListener("beforeunload", async () => {
    await saveNote();
    await saveConfig();
  });
})();
