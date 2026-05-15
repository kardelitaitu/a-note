import { invoke } from "@tauri-apps/api/core";

const editor = document.getElementById("editor");
const titleText = document.getElementById("title-text");
const btnPin = document.getElementById("btn-pin");
const btnMin = document.getElementById("btn-minimize");
const btnClose = document.getElementById("btn-close");
const btnMenu = document.getElementById("btn-menu");
const menuDropdown = document.getElementById("menu-dropdown");
const menuWordwrap = document.getElementById("menu-wordwrap");
let config = { width: 300, height: 400, left: 100, top: 100, font_size: 14, always_on_top: true, word_wrap: false, theme: "dark" };

const themes = [
  { id: "dark", label: "Dark" },
  { id: "light", label: "Light" },
  { id: "dark-black", label: "Dark black" },
  { id: "dark-blue", label: "Dark blue" },
  { id: "dark-choco", label: "Dark choco" },
  { id: "light-blue", label: "Light blue" },
  { id: "light-orange", label: "Light orange" },
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
}

function initThemes() {
  const container = document.getElementById("menu-themes");
  themes.forEach((t) => {
    const btn = document.createElement("button");
    btn.dataset.theme = t.id;
    btn.innerHTML = `<span class="menu-check"></span><span>${t.label}</span>`;
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
});

function closeMenu() {
  menuDropdown.classList.remove("open");
}

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
  await loadNote();
  await trackWindow();

  // Save config + note before unload
  window.addEventListener("beforeunload", async () => {
    await saveNote();
    await saveConfig();
  });
})();
