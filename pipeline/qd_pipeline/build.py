"""Build a language pack (SQLite) from a wiktextract JSONL dump.

Usage:
    python -m qd_pipeline.build --lang fr INPUT.jsonl[.gz] -o packs/fr.db \
        [--freq large_fr.msgpack.gz]

INPUT is a kaikki.org wiktextract dump. Only entries whose `lang_code`
matches --lang are kept. --freq is a wordfreq frequency list, used to rank
results. See spec/pack-format.md for the output schema.
"""

import argparse
import datetime
import gzip
import itertools
import json
import sqlite3
import sys
import time
import zlib
from pathlib import Path

from .languages import JUNK_FORMS, LANGUAGES, SKIP_FORM_TAGS
from .normalize import normalize
from .wordfreq import load_zipf

SCHEMA_VERSION = 1
MAX_EXAMPLES_PER_SENSE = 2

# How a lookup key reaches its entry (lookup.kind).
KIND_HEADWORD = 0
KIND_FORM = 1
KIND_FORM_OF = 2
KIND_REDIRECT = 3

SCHEMA = """
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE entries (
    id INTEGER PRIMARY KEY,
    word TEXT NOT NULL,
    pos TEXT NOT NULL,
    ipa TEXT,
    tags TEXT,
    freq INTEGER NOT NULL,
    data BLOB NOT NULL
);
CREATE TABLE lookup (
    key TEXT NOT NULL,
    entry_id INTEGER NOT NULL REFERENCES entries(id),
    kind INTEGER NOT NULL,
    form_ord INTEGER,
    note TEXT,
    PRIMARY KEY (key, entry_id)
) WITHOUT ROWID;
CREATE TABLE keys (key TEXT PRIMARY KEY, freq INTEGER NOT NULL) WITHOUT ROWID;
CREATE TEMP TABLE lookup_stage (
    key TEXT NOT NULL,
    entry_id INTEGER NOT NULL,
    kind INTEGER NOT NULL,
    form_ord INTEGER,
    note TEXT
);
"""


def open_input(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8")
    return open(path, "r", encoding="utf-8")


def encode_data(data: dict) -> bytes:
    text = json.dumps(data, ensure_ascii=False, separators=(",", ":"))
    return zlib.compress(text.encode("utf-8"), 9)


def decode_data(blob: bytes) -> dict:
    return json.loads(zlib.decompress(blob))


def _join_tags(tags) -> str | None:
    return " ".join(tags) if tags else None


def _first_ipa(entry: dict) -> str | None:
    for sound in entry.get("sounds", []):
        if sound.get("ipa"):
            return sound["ipa"]
    return None


def _senses(entry: dict) -> list[dict]:
    senses = []
    for sense in entry.get("senses", []):
        glosses = [g for g in sense.get("glosses", []) if g]
        if not glosses:
            continue
        out = {"glosses": glosses}
        if sense.get("tags"):
            out["tags"] = sense["tags"]
        examples = [e["text"] for e in sense.get("examples", []) if e.get("text")]
        if examples:
            out["examples"] = examples[:MAX_EXAMPLES_PER_SENSE]
        for link in ("form_of", "alt_of"):
            words = [f["word"] for f in sense.get(link, []) if f.get("word")]
            if words:
                out[link] = words
        senses.append(out)
    return senses


def _forms(entry: dict) -> list[dict]:
    forms = []
    seen = set()
    for form in entry.get("forms", []):
        text = form.get("form", "").strip()
        tags = form.get("tags", [])
        if not text or text in JUNK_FORMS or SKIP_FORM_TAGS.intersection(tags):
            continue
        dedup_key = (text, tuple(tags), tuple(form.get("raw_tags", [])))
        if dedup_key in seen:
            continue
        seen.add(dedup_key)
        forms.append({"form": text, "tags": tags, "raw_tags": form.get("raw_tags", [])})
    return forms


def _group_forms(forms: list[dict]) -> list[dict]:
    """Group consecutive forms sharing tags: a conjugation table becomes one
    group per tense instead of repeating the tags on every person."""
    groups = []
    for (tags, raw_tags), run in itertools.groupby(
        forms, key=lambda f: (tuple(f["tags"]), tuple(f["raw_tags"]))
    ):
        group = {"forms": [f["form"] for f in run]}
        if tags:
            group["tags"] = list(tags)
        if raw_tags:
            group["raw_tags"] = list(raw_tags)
        groups.append(group)
    return groups


class PackBuilder:
    def __init__(self, conn: sqlite3.Connection, lang: str, zipf: dict[str, float]):
        self.conn = conn
        self.lang = lang
        self.zipf = zipf
        self.form_key = LANGUAGES[lang]["form_key"]
        self.next_id = 1
        self.pending_entries: list[tuple] = []
        self.pending_lookup: list[tuple] = []
        self.redirects: list[tuple[str, str]] = []
        # Entries that only point to another word: inflections ("lance": form
        # of "lancer"), misspellings and variants ("recieve": misspelling of
        # "receive"). Their targets are resolved once all headwords are known;
        # the entry is then dropped and the link kept as a lookup row.
        self.form_of_links: list[tuple[int, str, str, str, str]] = []
        self.pure_form_ids: set[int] = set()
        self.headwords: dict[str, list[tuple[int, str]]] = {}
        self.stats = {"lines": 0, "entries": 0, "skipped_lang": 0, "redirects": 0}

    def freq(self, key: str) -> int:
        """Zipf frequency of a normalized word, times 100 (0 when unknown)."""
        return round(self.zipf.get(key, 0) * 100)

    def add(self, entry: dict) -> None:
        self.stats["lines"] += 1
        if "redirect" in entry:
            self.redirects.append((entry.get("title", ""), entry["redirect"]))
            self.stats["redirects"] += 1
            return
        if entry.get("lang_code") != self.lang or not entry.get("word"):
            self.stats["skipped_lang"] += 1
            return

        senses = _senses(entry)
        forms = _forms(entry)
        if not senses and not forms:
            return

        entry_id = self.next_id
        self.next_id += 1
        self.stats["entries"] += 1
        word = entry["word"]
        pos = entry.get("pos", "")
        headword_key = normalize(word)
        self.pending_entries.append(
            (
                entry_id,
                word,
                pos,
                _first_ipa(entry),
                _join_tags(entry.get("tags")),
                self.freq(headword_key),
                encode_data({"senses": senses, "forms": _group_forms(forms)}),
            )
        )

        lookups = {headword_key: (KIND_HEADWORD, None)}
        for i, form in enumerate(forms):
            key = self.form_key(normalize(form["form"]))
            # Multi-word forms are compound forms ("j'étais allé", "more
            # absolutely"): their words are already listed on their own.
            if key and " " not in key and key not in lookups:
                lookups[key] = (KIND_FORM, i)
        self.pending_lookup.extend(
            (key, entry_id, kind, ord_, None) for key, (kind, ord_) in lookups.items()
        )

        if senses and all("form_of" in s or "alt_of" in s for s in senses):
            self.pure_form_ids.add(entry_id)
            for sense in senses:
                for target in sense.get("form_of", []) + sense.get("alt_of", []):
                    self.form_of_links.append(
                        (entry_id, headword_key, pos, target, sense["glosses"][-1])
                    )
        else:
            self.headwords.setdefault(word, []).append((entry_id, pos))

        if len(self.pending_entries) >= 10_000:
            self.flush()

    def flush(self) -> None:
        self.conn.executemany(
            "INSERT INTO entries (id, word, pos, ipa, tags, freq, data) VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.pending_entries,
        )
        self.conn.executemany(
            "INSERT INTO lookup_stage (key, entry_id, kind, form_ord, note) VALUES (?, ?, ?, ?, ?)",
            self.pending_lookup,
        )
        self.pending_entries.clear()
        self.pending_lookup.clear()

    def finish(self) -> None:
        # "lance" is a form of "lancer": make the key "lance" reach lancer,
        # preferring targets with the same part of speech.
        unresolved = set()
        for entry_id, key, pos, target, note in self.form_of_links:
            candidates = self.headwords.get(target)
            if not candidates:
                unresolved.add(entry_id)
                continue
            same_pos = [c for c in candidates if c[1] == pos] or candidates
            self.pending_lookup.extend(
                (key, target_id, KIND_FORM_OF, None, note) for target_id, _ in same_pos
            )
        for title, target in self.redirects:
            if title:
                self.pending_lookup.extend(
                    (normalize(title), target_id, KIND_REDIRECT, None, None)
                    for target_id, _ in self.headwords.get(target, [])
                )
        self.flush()

        cur = self.conn.cursor()
        dropped = self.pure_form_ids - unresolved
        self.stats["dropped_form_entries"] = len(dropped)
        self.stats["entries"] -= len(dropped)
        cur.execute("CREATE TEMP TABLE dropped (id INTEGER PRIMARY KEY)")
        cur.executemany("INSERT INTO dropped (id) VALUES (?)", ((i,) for i in dropped))
        cur.execute("DELETE FROM entries WHERE id IN (SELECT id FROM dropped)")
        # A key may reach the same entry several ways; keep the most direct.
        cur.execute(
            "INSERT OR IGNORE INTO lookup (key, entry_id, kind, form_ord, note)"
            " SELECT key, entry_id, kind, form_ord, note FROM lookup_stage"
            " WHERE entry_id NOT IN (SELECT id FROM dropped)"
            " ORDER BY key, entry_id, kind, form_ord"
        )
        keys = [row[0] for row in cur.execute("SELECT DISTINCT key FROM lookup")]
        cur.executemany(
            "INSERT INTO keys (key, freq) VALUES (?, ?)", ((k, self.freq(k)) for k in keys)
        )
        self.stats["keys"] = len(keys)


def build(
    inputs: list[Path],
    lang: str,
    output: Path,
    freq_path: Path | None = None,
    zipf: dict[str, float] | None = None,
) -> dict:
    if lang not in LANGUAGES:
        raise SystemExit(f"unsupported language {lang!r}; known: {', '.join(LANGUAGES)}")
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_suffix(output.suffix + ".tmp")
    tmp.unlink(missing_ok=True)

    conn = sqlite3.connect(tmp)
    conn.execute("PRAGMA journal_mode = OFF")
    conn.execute("PRAGMA synchronous = OFF")
    conn.execute("PRAGMA temp_store = FILE")
    conn.executescript(SCHEMA)
    if zipf is None:
        zipf = load_zipf(freq_path) if freq_path else {}
    builder = PackBuilder(conn, lang, zipf)
    started = time.monotonic()
    for path in inputs:
        with open_input(path) as f:
            for line in f:
                if line.strip():
                    builder.add(json.loads(line))
                if builder.stats["lines"] % 100_000 == 0:
                    print(
                        f"  {builder.stats['lines']:,} lines, "
                        f"{builder.stats['entries']:,} entries kept",
                        file=sys.stderr,
                    )
    builder.finish()

    meta = {
        "schema_version": str(SCHEMA_VERSION),
        "lang": lang,
        "lang_name": LANGUAGES[lang]["name"],
        "source": "Wiktionary, via wiktextract (kaikki.org)",
        "source_files": ", ".join(p.name for p in inputs),
        "freq_source": f"wordfreq ({freq_path.name})" if freq_path else "none",
        "license": "CC BY-SA 4.0 (Wiktionary contributors; wordfreq frequencies)",
        "built_at": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
    }
    conn.executemany("INSERT INTO meta (key, value) VALUES (?, ?)", meta.items())
    conn.commit()
    conn.execute("VACUUM")
    conn.close()
    tmp.replace(output)

    builder.stats["seconds"] = round(time.monotonic() - started, 1)
    return builder.stats


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("inputs", nargs="+", type=Path, help="wiktextract JSONL file(s), optionally .gz")
    parser.add_argument("--lang", required=True, help="language code to keep, e.g. fr or en")
    parser.add_argument("-o", "--output", type=Path, required=True, help="pack file to write")
    parser.add_argument("--freq", type=Path, help="wordfreq list, e.g. large_fr.msgpack.gz")
    args = parser.parse_args(argv)
    stats = build(args.inputs, args.lang, args.output, args.freq)
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
