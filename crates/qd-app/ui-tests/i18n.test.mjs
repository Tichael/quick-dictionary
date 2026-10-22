// Checks the interface strings: run with `node --test crates/qd-app/ui-tests`.
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { test } from "node:test";

import { keysByLanguage, t } from "../ui/i18n.js";

const UI = new URL("../ui/", import.meta.url);

test("every language has the same keys as English", () => {
  const { en, ...others } = keysByLanguage();
  for (const [code, keys] of Object.entries(others)) {
    assert.deepEqual([...keys].sort(), [...en].sort(), code);
  }
});

test("every key used by the pages and scripts exists", () => {
  const known = new Set(keysByLanguage().en);
  for (const file of readdirSync(UI)) {
    if (!/\.(html|js)$/.test(file) || file === "i18n.js") continue;
    const source = readFileSync(new URL(file, UI), "utf8");
    const used = [
      ...source.matchAll(/data-i18n(?:-html|-placeholder|-title)?="([^"]+)"/g),
      ...source.matchAll(/\bt\("([^"]+)"/g),
    ].map((m) => m[1]);
    for (const key of used) assert.ok(known.has(key), `${file}: unknown key ${key}`);
  }
});

test("placeholders are filled", () => {
  assert.equal(t("lookup.noMatch", { query: "xyz" }), "No match for “xyz”.");
  assert.equal(t("missing.key"), "missing.key");
});
