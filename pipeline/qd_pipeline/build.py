"""Build a language pack (SQLite) from a wiktextract JSONL dump.

Usage:
    python -m qd_pipeline.build --lang fr INPUT.jsonl[.gz] -o packs/fr.db

INPUT is a kaikki.org wiktextract dump. Only entries whose `lang_code`
matches --lang are kept. See spec/pack-format.md for the output schema.
"""

import argparse
import datetime
import gzip
import json
import sqlite3
import sys
import time
from pathlib import Path

from .languages import LANGUAGES, SKIP_FORM_TAGS
from .normalize import normalize

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
    tags TEXT
);
CREATE TABLE senses (
    entry_id INTEGER NOT NULL REFERENCES entries(id),
    ord INTEGER NOT NULL,
    glosses TEXT NOT NULL,
    tags TEXT,
    examples TEXT,
    form_of TEXT
);
CREATE TABLE forms (
    entry_id INTEGER NOT NULL REFERENCES entries(id),
    ord INTEGER NOT NULL,
    form TEXT NOT NULL,
    tags TEXT,
    raw_tags TEXT
);
CREATE TABLE lookup (
    key TEXT NOT NULL,
    entry_id INTEGER NOT NULL REFERENCES entries(id),
    kind INTEGER NOT NULL,
    form_ord INTEGER
);
"""

INDEXES = """
CREATE INDEX senses_entry ON senses(entry_id, ord);
CREATE INDEX forms_entry ON forms(entry_id, ord);
CREATE INDEX lookup_key ON lookup(key);
"""


def open_input(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8")
    return open(path, "r", encoding="utf-8")


def _join_tags(tags) -> str | None:
    return " ".join(tags) if tags else None


def _first_ipa(entry: dict) -> str | None:
    for sound in entry.get("sounds", []):
        if sound.get("ipa"):
            return sound["ipa"]
    return None


def _senses(entry: dict) -> list[tuple]:
    rows = []
    for sense in entry.get("senses", []):
        glosses = [g for g in sense.get("glosses", []) if g]
        if not glosses:
            continue
        examples = [
            e["text"]
            for e in sense.get("examples", [])
            if e.get("text")
        ][:MAX_EXAMPLES_PER_SENSE]
        form_of = [f["word"] for f in sense.get("form_of", []) if f.get("word")]
        rows.append(
            (
                json.dumps(glosses, ensure_ascii=False),
                _join_tags(sense.get("tags")),
                json.dumps(examples, ensure_ascii=False) if examples else None,
                json.dumps(form_of, ensure_ascii=False) if form_of else None,
            )
        )
    return rows


def _forms(entry: dict) -> list[dict]:
    forms = []
    seen = set()
    for form in entry.get("forms", []):
        text = form.get("form", "").strip()
        tags = form.get("tags", [])
        if not text or text == "-" or SKIP_FORM_TAGS.intersection(tags):
            continue
        dedup_key = (text, tuple(tags), tuple(form.get("raw_tags", [])))
        if dedup_key in seen:
            continue
        seen.add(dedup_key)
        forms.append(form | {"form": text})
    return forms


class PackBuilder:
    def __init__(self, conn: sqlite3.Connection, lang: str):
        self.conn = conn
        self.lang = lang
        self.form_key = LANGUAGES[lang]["form_key"]
        self.next_id = 1
        self.redirects: list[tuple[str, str]] = []
        # form_of targets are resolved after loading, once all headwords exist.
        self.form_of_links: list[tuple[int, str]] = []
        self.stats = {"lines": 0, "entries": 0, "skipped_lang": 0, "redirects": 0}

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
        cur = self.conn.cursor()
        cur.execute(
            "INSERT INTO entries (id, word, pos, ipa, tags) VALUES (?, ?, ?, ?, ?)",
            (
                entry_id,
                word,
                entry.get("pos", ""),
                _first_ipa(entry),
                _join_tags(entry.get("tags")),
            ),
        )
        cur.executemany(
            "INSERT INTO senses (entry_id, ord, glosses, tags, examples, form_of)"
            " VALUES (?, ?, ?, ?, ?, ?)",
            [(entry_id, i, *row) for i, row in enumerate(senses)],
        )
        cur.executemany(
            "INSERT INTO forms (entry_id, ord, form, tags, raw_tags) VALUES (?, ?, ?, ?, ?)",
            [
                (
                    entry_id,
                    i,
                    f["form"],
                    _join_tags(f.get("tags")),
                    json.dumps(f["raw_tags"], ensure_ascii=False) if f.get("raw_tags") else None,
                )
                for i, f in enumerate(forms)
            ],
        )

        headword_key = normalize(word)
        lookups = {headword_key: (KIND_HEADWORD, None)}
        for i, form in enumerate(forms):
            key = self.form_key(normalize(form["form"]))
            # Multi-word forms are compound forms ("j'étais allé", "more
            # absolutely"): their words are already listed on their own.
            if key and " " not in key and key not in lookups:
                lookups[key] = (KIND_FORM, i)
        cur.executemany(
            "INSERT INTO lookup (key, entry_id, kind, form_ord) VALUES (?, ?, ?, ?)",
            [(key, entry_id, kind, ord_) for key, (kind, ord_) in lookups.items()],
        )

        for sense in senses:
            if sense[3]:
                for target in json.loads(sense[3]):
                    self.form_of_links.append((entry_id, target))

    def finish(self) -> None:
        cur = self.conn.cursor()
        cur.execute("CREATE TEMP TABLE headwords AS SELECT id, word FROM entries")
        cur.execute("CREATE INDEX temp.headwords_word ON headwords(word)")

        # "lance" is a form of "lancer": make the key "lance" also reach lancer.
        cur.executemany(
            "INSERT INTO lookup (key, entry_id, kind, form_ord)"
            " SELECT DISTINCT l.key, h.id, ?, NULL"
            " FROM lookup l JOIN headwords h ON h.word = ?"
            " WHERE l.entry_id = ? AND l.kind = ?",
            [
                (KIND_FORM_OF, target, entry_id, KIND_HEADWORD)
                for entry_id, target in self.form_of_links
            ],
        )
        cur.executemany(
            "INSERT INTO lookup (key, entry_id, kind, form_ord)"
            " SELECT ?, id, ?, NULL FROM headwords WHERE word = ?",
            [
                (normalize(title), KIND_REDIRECT, target)
                for title, target in self.redirects
                if title
            ],
        )
        # A key may reach the same entry several ways; keep the most direct.
        cur.execute(
            "DELETE FROM lookup WHERE rowid NOT IN ("
            " SELECT rowid FROM (SELECT rowid, ROW_NUMBER() OVER ("
            "  PARTITION BY key, entry_id ORDER BY kind, form_ord) AS n FROM lookup)"
            " WHERE n = 1)"
        )
        cur.executescript(INDEXES)


def build(inputs: list[Path], lang: str, output: Path) -> dict:
    if lang not in LANGUAGES:
        raise SystemExit(f"unsupported language {lang!r}; known: {', '.join(LANGUAGES)}")
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_suffix(output.suffix + ".tmp")
    tmp.unlink(missing_ok=True)

    conn = sqlite3.connect(tmp)
    conn.execute("PRAGMA journal_mode = OFF")
    conn.execute("PRAGMA synchronous = OFF")
    conn.executescript(SCHEMA)
    builder = PackBuilder(conn, lang)
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
        "license": "CC BY-SA 4.0 (text from Wiktionary contributors)",
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
    args = parser.parse_args(argv)
    stats = build(args.inputs, args.lang, args.output)
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
