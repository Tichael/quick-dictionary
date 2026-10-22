"""Inspect a language pack: print its stats, or look words up.

Usage:
    python -m qd_pipeline.inspect_pack packs/fr.db            # stats
    python -m qd_pipeline.inspect_pack packs/fr.db allions    # lookup
"""

import argparse
import sqlite3
from pathlib import Path

from .build import decode_data
from .normalize import normalize

KIND_LABELS = {0: "headword", 1: "inflected form", 2: "form of", 3: "redirect"}


def stats(conn: sqlite3.Connection) -> None:
    for key, value in conn.execute("SELECT key, value FROM meta ORDER BY key"):
        print(f"{key:>15}: {value}")
    for table in ("entries", "lookup"):
        (count,) = conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()
        print(f"{table:>15}: {count:,} rows")
    print("\nTop parts of speech:")
    for pos, count in conn.execute(
        "SELECT pos, COUNT(*) c FROM entries GROUP BY pos ORDER BY c DESC LIMIT 12"
    ):
        print(f"  {pos or '(none)':<20} {count:,}")


def flat_forms(data: dict) -> list[tuple[str, list[str], list[str]]]:
    return [
        (form, group.get("tags", []), group.get("raw_tags", []))
        for group in data["forms"]
        for form in group["forms"]
    ]


def lookup(conn: sqlite3.Connection, query: str) -> None:
    key = normalize(query)
    rows = conn.execute(
        "SELECT l.kind, l.form_ord, l.note, e.word, e.pos, e.ipa, e.tags, e.data"
        " FROM lookup l JOIN entries e ON e.id = l.entry_id"
        " WHERE l.key = ? ORDER BY l.kind, e.id",
        (key,),
    ).fetchall()
    if not rows:
        print(f"No match for {query!r} (key {key!r})")
        return
    for kind, form_ord, note, word, pos, ipa, tags, blob in rows:
        data = decode_data(blob)
        print(f"\n== {word} ({pos}{', ' + tags if tags else ''}) {ipa or ''}  [{KIND_LABELS[kind]}]")
        if note:
            print(f"   note: {note}")
        if form_ord is not None:
            form, form_tags, raw_tags = flat_forms(data)[form_ord]
            print(f"   matched: {form}  ({' '.join(form_tags + raw_tags) or 'no tags'})")
        for sense in data["senses"]:
            label = f" [{' '.join(sense['tags'])}]" if sense.get("tags") else ""
            print(f"   - {' › '.join(sense['glosses'])}{label}")
            for example in sense.get("examples", []):
                print(f"       « {example} »")
        if data["forms"]:
            print(f"   forms ({len(flat_forms(data))}):")
            for group in data["forms"]:
                label = " ".join(group.get("tags", []) + group.get("raw_tags", []))
                print(f"     [{label}] {' · '.join(group['forms'])}")


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("pack", type=Path)
    parser.add_argument("words", nargs="*")
    args = parser.parse_args(argv)
    conn = sqlite3.connect(f"file:{args.pack}?mode=ro", uri=True)
    if not args.words:
        stats(conn)
    for word in args.words:
        lookup(conn, word)


if __name__ == "__main__":
    main()
