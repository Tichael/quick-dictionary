import { esc, renderResult } from "./entry.js";
import { loadLanguage, t } from "./i18n.js";
import { applyTheme } from "./theme.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const input = document.getElementById("query");
const resultsEl = document.getElementById("results");
const completionsEl = document.getElementById("completions");
const noticeEl = document.getElementById("notice");
const toastEl = document.getElementById("toast");

// Defaults keep the window usable if the settings cannot be read.
let settings = await invoke("get_settings").catch(() => ({ theme: "system", clearOnShow: false }));
applyTheme(settings.theme);
await loadLanguage();

let results = [];
let selected = 0;
let searchId = 0;

async function search() {
  const query = input.value.trim();
  const id = ++searchId;
  if (!query) {
    results = [];
    render(null);
    return;
  }
  let response;
  try {
    response = await invoke("search", { query });
  } catch (error) {
    response = { results: [], completions: [], error: String(error) };
  }
  if (id !== searchId) return; // a newer search is on its way
  results = response.results;
  selected = 0;
  render(response, query);
  resultsEl.scrollTop = 0;
}

function render(response, query) {
  resultsEl.innerHTML = results
    .map((r, i) => `<article class="result${i === selected ? " selected" : ""}" data-index="${i}">${renderResult(r, i === selected)}</article>`)
    .join("");

  let notice = "";
  if (response?.error) {
    notice = `<span class="error">${esc(response.error)}</span>`;
  } else if (response?.noPacks) {
    notice = t("lookup.noPacks");
  } else if (response?.loading) {
    notice = esc(t("lookup.loading"));
  } else if (response && !results.length) {
    notice = t("lookup.noMatch", { query: esc(query) });
  } else if (results[0]?.distance > 0) {
    const spellings = [...new Set(results.map((r) => r.matched_form?.form ?? r.entry.word))];
    const words = spellings
      .slice(0, 5)
      .map((s) => `<button class="link lookup-word" data-word="${esc(s)}">${esc(s)}</button>`)
      .join(", ");
    notice = t("lookup.didYouMean", { query: esc(query), words });
  }
  noticeEl.innerHTML = notice;

  // Completions help when the end of the word is unknown ("conjug…").
  const typed = query ? query.toLowerCase() : "";
  const exact = results[0]?.distance === 0;
  const completions = exact ? [] : (response?.completions ?? []).filter((c) => c.key !== typed).slice(0, 6);
  completionsEl.innerHTML = completions
    .map((c) => `<button class="lookup-word" data-word="${esc(c.key)}" title="${esc(c.lang)}">${esc(c.key)}</button>`)
    .join("");
}

function select(index) {
  if (!results.length) return;
  selected = Math.max(0, Math.min(results.length - 1, index));
  for (const el of resultsEl.querySelectorAll(".result")) {
    const i = Number(el.dataset.index);
    el.classList.toggle("selected", i === selected);
    el.innerHTML = renderResult(results[i], i === selected);
  }
  resultsEl.querySelector(".result.selected")?.scrollIntoView({ block: "nearest" });
}

function lookUp(word) {
  input.value = word;
  input.focus();
  search();
}

let toastTimer;
function toast(text) {
  toastEl.textContent = text;
  toastEl.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toastEl.hidden = true), 1200);
}

let debounce;
input.addEventListener("input", () => {
  clearTimeout(debounce);
  debounce = setTimeout(search, 60);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    event.preventDefault();
    invoke("hide_lookup");
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    select(selected + 1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    select(selected - 1);
  } else if (event.key === "Tab" && completionsEl.firstElementChild && document.activeElement === input) {
    event.preventDefault();
    lookUp(completionsEl.firstElementChild.dataset.word);
  } else if (event.key === "Enter" && results[selected]) {
    // Enter copies the selected word, ready to paste where you were writing.
    copy(results[selected].matched_form?.form?.replace(/^.*\s/, "") ?? results[selected].entry.word);
  } else if (event.key.length === 1 && document.activeElement !== input && !event.ctrlKey && !event.altKey) {
    input.focus(); // typing anywhere goes to the search box
  }
});

async function copy(word) {
  try {
    await navigator.clipboard.writeText(word);
    toast(t("lookup.copied", { word }));
  } catch {
    toast(t("lookup.copyFailed"));
  }
}

document.addEventListener("click", (event) => {
  const target = event.target;
  if (target.closest(".copy")) {
    copy(target.closest(".copy").dataset.word);
  } else if (target.closest(".lookup-word")) {
    lookUp(target.closest(".lookup-word").dataset.word);
  } else if (target.closest(".toggle-senses")) {
    target.closest(".result").querySelector(".senses").classList.add("all");
    target.remove();
  } else if (target.closest(".toggle-compound")) {
    target.closest(".conjugation").classList.add("show-compound");
  } else if (target.closest(".example")) {
    target.closest(".example").classList.toggle("open");
  } else if (target.closest("#open-dictionaries") || target.closest("#settings")) {
    invoke("open_settings");
  } else if (target.closest("#close")) {
    invoke("hide_lookup");
  } else if (target.closest(".result:not(.selected)")) {
    select(Number(target.closest(".result").dataset.index));
  }
});

await listen("lookup-shown", () => {
  if (settings.clearOnShow) {
    input.value = "";
    search();
  }
  input.focus();
  input.select();
});

await listen("settings-changed", async (event) => {
  settings = event.payload;
  applyTheme(settings.theme);
  await loadLanguage();
  search();
});

await listen("library-changed", () => search());

input.focus();
