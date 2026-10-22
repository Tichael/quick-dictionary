import { esc } from "./entry.js";
import { loadLanguage, t, uiLang } from "./i18n.js";
import { applyTheme } from "./theme.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const messageEl = document.getElementById("message");
const shortcutButton = document.getElementById("shortcut");

// Defaults keep the window usable if the settings cannot be read.
let settings = await invoke("get_settings").catch(() => ({ theme: "system", clearOnShow: false }));
await loadLanguage();
const version = await window.__TAURI__.app.getVersion().catch(() => "");

function showMessage(text, kind = "error") {
  messageEl.textContent = text;
  messageEl.className = kind === "info" ? "info" : "";
  messageEl.hidden = !text;
}

function fill() {
  applyTheme(settings.theme);
  document.title = t("settings.title");
  document.getElementById("version").textContent = t("about.version", { version });
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
  shortcutButton.textContent = recording ? t("general.shortcutRecording") : settings.shortcut;
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
    shortcutButton.textContent = t("general.shortcutNeedsModifier");
    return;
  }
  parts.push(keyName(event.code));
  recording = false;
  shortcutButton.classList.remove("recording");
  save({ shortcut: parts.join("+") });
});

// --- Dictionaries --------------------------------------------------------

function formatSize(bytes) {
  const number = (n, digits) => n.toLocaleString(uiLang(), { maximumFractionDigits: digits });
  return bytes > 1e9 ? t("size.gb", { n: number(bytes / 1e9, 1) }) : t("size.mb", { n: number(bytes / 1e6, 0) });
}

function formatDate(iso) {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? iso.slice(0, 10) : date.toLocaleDateString(uiLang(), { dateStyle: "medium" });
}

async function showPacks() {
  const info = await invoke("library_info");
  const packsEl = document.getElementById("packs");
  document.getElementById("packs-dir").innerHTML = t("dictionaries.folder", { dir: `<code>${esc(info.dir)}</code>` });
  if (info.loading) {
    packsEl.innerHTML = `<div class="empty">${esc(t("dictionaries.loading"))}</div>`;
    return;
  }
  const packs = info.packs.map((p) => `
    <div class="pack">
      <span class="badge">${esc(p.lang.toUpperCase())}</span>
      <div class="details">
        <div class="name">${esc(p.langName)}</div>
        <div class="hint">${t("dictionaries.packInfo", {
          entries: p.entries.toLocaleString(uiLang()),
          size: esc(formatSize(p.sizeBytes)),
          date: esc(formatDate(p.builtAt)),
          file: `<code>${esc(p.file)}</code>`,
        })}</div>
      </div>
      <button class="remove" data-file="${esc(p.file)}">${esc(t("dictionaries.remove"))}</button>
    </div>`);
  const failed = info.failed.map((f) => `
    <div class="pack failed">
      <div class="details">
        <div class="name">${esc(f.file)}</div>
        <div class="hint error">${esc(f.error)}</div>
      </div>
      <button class="remove" data-file="${esc(f.file)}">${esc(t("dictionaries.remove"))}</button>
    </div>`);
  packsEl.innerHTML = packs.concat(failed).join("") ||
    `<div class="empty">${esc(t("dictionaries.none"))}</div>`;
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
  run(() => invoke("add_pack"), t("dictionaries.installing")));
document.getElementById("open-folder").addEventListener("click", () =>
  invoke("open_packs_folder").catch((e) => showMessage(String(e))));
document.getElementById("reload").addEventListener("click", () =>
  run(() => invoke("reload_packs"), t("dictionaries.reloading")));
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
await listen("settings-changed", async (event) => {
  settings = event.payload;
  await loadLanguage();
  fill();
  showPacks();
});

fill();
showPacks();
