import { invoke } from "@tauri-apps/api/core";

const editor = document.getElementById("editor");
const titleText = document.getElementById("title-text");
const btnPin = document.getElementById("btn-pin");
const btnMin = document.getElementById("btn-minimize");
const btnClose = document.getElementById("btn-close");

let config = { width: 300, height: 400, left: 100, top: 100, font_size: 14, always_on_top: true };

async function loadConfig() {
  try {
    config = await invoke("load_config");
    editor.style.fontSize = config.font_size + "px";
    applyPinState();
  } catch (e) {
    console.error("load_config failed:", e);
  }
}

function applyPinState() {
  btnPin.className = config.always_on_top ? "active" : "inactive";
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

  await loadConfig();
  await loadNote();
  await trackWindow();

  // Save config + note before unload
  window.addEventListener("beforeunload", async () => {
    await saveNote();
    await saveConfig();
  });
})();
