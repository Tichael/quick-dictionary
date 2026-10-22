import { esc } from "./entry.js";
import { applyTheme } from "./theme.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const messageEl = document.getElementById("message");
const shortcutButton = document.getElementById("shortcut");

// Defaults keep the window usable if the settings cannot be read.
let settings = await invoke("get_settings").catch(() => ({ theme: "system", clearOnShow: false }));

function showMessage(text, kind = "error") {
  messageEl.textContent = text;
  messageEl.className = kind === "info" ? "info" : "";
  messageEl.hidden = !text;
}

function fill() {
  applyTheme(settings.theme);
  shortcutButton.textContent = settings.shortcut;
  for (const el of document.querySelectorAll("[data-setting]")) {
    const value = settings[el.dataset.setting];
    if (el.type === "checkbox") el.checked = value;
    else el.value = value;
  }
}

async function save(changes) {
  try {
    settings = await invoke("save_settings", { new: { ...settings, ...changes } });
    showMessage("");
  } catch (error) {
    // The backend kept what it could not apply; show what is in effect.
    settings = await invoke("get_settings");
    showMessage(String(error));
  }
  fill();
}

for (const el of document.querySelectorAll("[data-setting]")) {
  el.addEventListener("change", () => {
    const value = el.type === "checkbox" ? el.checked : el.type === "number" ? Number(el.value) : el.value;
    save({ [el.dataset.setting]: value });
  });
}

// --- Shortcut capture ----------------------------------------------------

const MODIFIERS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph"]);

/** "KeyD" -> "D", "Digit1" -> "1"; other codes (F5, Space…) stay as they are. */
function keyName(code) {
  return code.replace(/^Key/, "").replace(/^Digit/, "");
}

let recording = false;

shortcutButton.addEventListener("click", () => {
  recording = !recording;
  shortcutButton.classList.toggle("recording", recording);
  shortcutButton.textContent = recording ? "Press a shortcut…" : settings.shortcut;
});

document.addEventListener("keydown", (event) => {
  if (!recording) return;
  event.preventDefault();
  if (event.key === "Escape") {
    recording = false;
    shortcutButton.classList.remove("recording");
    shortcutButton.textContent = settings.shortcut;
    return;
  }
  if (MODIFIERS.has(event.key)) return; // wait for the actual key
  const parts = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Super");
  const isFunctionKey = /^F\d+$/.test(event.code);
  if (!parts.some((p) => p !== "Shift") && !isFunctionKey) {
    shortcutButton.textContent = "Add Ctrl, Alt or Win…";
    return;
  }
  parts.push(keyName(event.code));
  recording = false;
  shortcutButton.classList.remove("recording");
  save({ shortcut: parts.join("+") });
});

// --- Dictionaries --------------------------------------------------------

function formatSize(bytes) {
  return bytes > 1e9 ? `${(bytes / 1e9).toFixed(1)} GB` : `${Math.round(bytes / 1e6)} MB`;
}

async function showPacks() {
  const info = await invoke("library_info");
  const packsEl = document.getElementById("packs");
  document.getElementById("packs-dir").innerHTML = `Folder: <code>${esc(info.dir)}</code>`;
  if (info.loading) {
    packsEl.innerHTML = `<div class="empty">Opening dictionaries…</div>`;
    return;
  }
  const packs = info.packs.map((p) => `
    <div class="pack">
      <span class="badge">${esc(p.lang.toUpperCase())}</span>
      <div class="details">
        <div class="name">${esc(p.langName)}</div>
        <div class="hint">${p.entries.toLocaleString()} entries · ${formatSize(p.sizeBytes)} ·
          built ${esc(p.builtAt.slice(0, 10))} · <code>${esc(p.file)}</code></div>
      </div>
      <button class="remove" data-file="${esc(p.file)}">Remove</button>
    </div>`);
  const failed = info.failed.map((f) => `
    <div class="pack failed">
      <div class="details">
        <div class="name">${esc(f.file)}</div>
        <div class="hint error">${esc(f.error)}</div>
      </div>
      <button class="remove" data-file="${esc(f.file)}">Remove</button>
    </div>`);
  packsEl.innerHTML = packs.concat(failed).join("") ||
    `<div class="empty">No dictionary installed. Add a language pack file to start looking words up.</div>`;
}

async function run(action, busyText) {
  showMessage(busyText, "info");
  try {
    const result = await action();
    showMessage("");
    return result;
  } catch (error) {
    showMessage(String(error));
  } finally {
    showPacks();
  }
}

document.getElementById("add-pack").addEventListener("click", () =>
  run(() => invoke("add_pack"), "Installing… (this can take a few seconds for a large dictionary)"));
document.getElementById("open-folder").addEventListener("click", () =>
  invoke("open_packs_folder").catch((e) => showMessage(String(e))));
document.getElementById("reload").addEventListener("click", () =>
  run(() => invoke("reload_packs"), "Reloading…"));
document.getElementById("packs").addEventListener("click", (event) => {
  const button = event.target.closest(".remove");
  if (button) run(() => invoke("remove_pack", { file: button.dataset.file }), "");
});

// --- Navigation ----------------------------------------------------------

// External links open in the browser, not in this window.
document.addEventListener("click", (event) => {
  const link = event.target.closest("a[href^='https://']");
  if (link) {
    event.preventDefault();
    invoke("open_url", { url: link.href }).catch((e) => showMessage(String(e)));
  }
});

const main = document.querySelector("main");
const navLinks = document.querySelectorAll("nav a");

function highlight(id) {
  for (const a of navLinks) a.classList.toggle("current", a.getAttribute("href") === `#${id}`);
}

// While scrolling, the current section is the last one starting in the top
// part of the view. A click wins until the scroll it starts has ended.
let clicked = null;
let clickTimer;
main.addEventListener("scroll", () => {
  if (clicked) return;
  let current = null;
  for (const section of document.querySelectorAll("section")) {
    if (section.offsetTop - main.scrollTop < main.clientHeight * 0.4) current = section.id;
  }
  highlight(current);
});
for (const a of navLinks) {
  a.addEventListener("click", () => {
    clicked = a.getAttribute("href").slice(1);
    highlight(clicked);
    clearTimeout(clickTimer);
    clickTimer = setTimeout(() => (clicked = null), 800);
  });
}

await listen("library-changed", showPacks);
await listen("settings-changed", (event) => {
  settings = event.payload;
  fill();
});

fill();
showPacks();
