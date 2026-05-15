import { invoke } from "@tauri-apps/api/core";

const editor = document.getElementById("editor");
const titleText = document.getElementById("title-text");
const btnPin = document.getElementById("btn-pin");
const btnMin = document.getElementById("btn-minimize");
const btnClose = document.getElementById("btn-close");
const btnMenu = document.getElementById("btn-menu");
const menuDropdown = document.getElementById("menu-dropdown");
const menuWordwrap = document.getElementById("menu-wordwrap");
let config = { width: 300, height: 400, left: 100, top: 100, font_size: 14, always_on_top: true, word_wrap: false, theme: "dark", titlebar_color: "", titlebar_fill: 100 };

const themes = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "dark-black", label: "Dark black" },
  { id: "dark-blue", label: "Dark blue" },
  { id: "dark-choco", label: "Dark choco" },
  { id: "light-blue", label: "Light blue" },
  { id: "light-orange", label: "Light orange" },
  { id: "dracula", label: "Dracula" },
  { id: "monokai", label: "Monokai" },
  { id: "nord", label: "Nord" },
  { id: "solarized-dark", label: "Solarized dark" },
  { id: "solarized-light", label: "Solarized light" },
  { id: "gruvbox-dark", label: "Gruvbox dark" },
  { id: "gruvbox-light", label: "Gruvbox light" },
  { id: "catppuccin", label: "Catppuccin" },
];

async function loadConfig() {
  try {
    config = await invoke("load_config");
    editor.style.fontSize = config.font_size + "px";
    editor.style.whiteSpace = config.word_wrap ? "pre-wrap" : "pre";
    applyTheme();
    applyPinState();
    applyWordWrapState();
  } catch (e) {
    console.error("load_config failed:", e);
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
    document.body.style.setProperty("--title-text", isLight ? "rgba(0,0,0,0.4)" : "rgba(255,255,255,0.4)");
    document.body.style.setProperty("--btn-color", isLight ? "#555" : "#b3b3b3");
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
    btn.addEventListener("click", () => {
      config.theme = t.id;
      applyTheme();
      saveConfig();
      closeMenu();
    });
    container.appendChild(btn);
  });
}

function applyWordWrapState() {
  menuWordwrap.className = config.word_wrap ? "on" : "";
}

async function saveConfig() {
  try {
    await invoke("save_config", { cfg: config });
  } catch (e) {
    console.error("save_config failed:", e);
  }
}

async function loadNote() {
  try {
    const data = await invoke("load_note");
    editor.value = data.text;
    editor.focus();
    const pos = Math.min(data.cursor_pos || 0, data.text.length);
    setTimeout(() => {
      editor.setSelectionRange(pos, pos);
      editor.scrollTop = data.scroll_top || 0;
    }, 0);
  } catch (e) {
    console.error("load_note failed:", e);
  }
}

async function saveNote() {
  try {
    await invoke("save_note", {
      note: {
        text: editor.value,
        cursor_pos: editor.selectionStart,
        scroll_top: editor.scrollTop,
      },
    });
  } catch (e) {
    console.error("save_note failed:", e);
  }
}

// Auto-save 5s after last edit
let saveTimer;
editor.addEventListener("input", () => {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNote, 5000);
});

// Ctrl+Scroll to zoom
editor.addEventListener("wheel", (e) => {
  if (!e.ctrlKey) return;
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

// Menu toggle
btnMenu.addEventListener("click", (e) => {
  e.stopPropagation();
  menuDropdown.classList.toggle("open");
  menuDropdown.classList.remove("show-themes", "show-titlebar");
});

function closeMenu() {
  menuDropdown.classList.remove("open");
  menuDropdown.classList.remove("show-themes", "show-titlebar");
}

// Theme submenu navigation
document.getElementById("menu-theme-btn").addEventListener("click", () => {
  menuDropdown.classList.add("show-themes");
});

document.getElementById("menu-theme-back").addEventListener("click", () => {
  menuDropdown.classList.remove("show-themes");
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
});

// Word wrap toggle
menuWordwrap.addEventListener("click", () => {
  config.word_wrap = !config.word_wrap;
  editor.style.whiteSpace = config.word_wrap ? "pre-wrap" : "pre";
  applyWordWrapState();
  saveConfig();
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

// Init
(async () => {
  const name = await invoke("get_app_name");
  titleText.textContent = name;

  initThemes();
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
