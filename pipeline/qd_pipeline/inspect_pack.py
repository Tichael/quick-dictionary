"""Inspect a language pack: print its stats, or look words up.

Usage:
    python -m qd_pipeline.inspect_pack packs/fr.db            # stats
    python -m qd_pipeline.inspect_pack packs/fr.db allions    # lookup
"""

import argparse
import json
import sqlite3
from pathlib import Path

from .normalize import normalize

KIND_LABELS = {0: "headword", 1: "inflected form", 2: "form of", 3: "redirect"}


def stats(conn: sqlite3.Connection) -> None:
    for key, value in conn.execute("SELECT key, value FROM meta ORDER BY key"):
        print(f"{key:>15}: {value}")
    for table in ("entries", "senses", "forms", "lookup"):
        (count,) = conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()
        print(f"{table:>15}: {count:,} rows")
    print("\nTop parts of speech:")
    for pos, count in conn.execute(
        "SELECT pos, COUNT(*) c FROM entries GROUP BY pos ORDER BY c DESC LIMIT 12"
    ):
        print(f"  {pos or '(none)':<20} {count:,}")


def lookup(conn: sqlite3.Connection, query: str) -> None:
    key = normalize(query)
    rows = conn.execute(
        "SELECT l.entry_id, l.kind, l.form_ord, e.word, e.pos, e.ipa"
        " FROM lookup l JOIN entries e ON e.id = l.entry_id"
        " WHERE l.key = ? ORDER BY l.kind, e.id",
        (key,),
    ).fetchall()
    if not rows:
        print(f"No match for {query!r} (key {key!r})")
        return
    for entry_id, kind, form_ord, word, pos, ipa in rows:
        print(f"\n== {word} ({pos}) {ipa or ''}  [{KIND_LABELS[kind]}]")
        if form_ord is not None:
            form, tags = conn.execute(
                "SELECT form, tags FROM forms WHERE entry_id = ? AND ord = ?",
                (entry_id, form_ord),
            ).fetchone()
            print(f"   matched: {form}  ({tags or 'no tags'})")
        for glosses, tags, examples in conn.execute(
            "SELECT glosses, tags, examples FROM senses WHERE entry_id = ? ORDER BY ord",
            (entry_id,),
        ):
            label = f" [{tags}]" if tags else ""
            print(f"   - {' › '.join(json.loads(glosses))}{label}")
            for example in json.loads(examples or "[]"):
                print(f"       « {example} »")
        forms = conn.execute(
            "SELECT form, tags, raw_tags FROM forms WHERE entry_id = ? ORDER BY ord",
            (entry_id,),
        ).fetchall()
        if forms:
            print(f"   forms ({len(forms)}):")
            for form, tags, raw_tags in forms[:60]:
                raw = f"  raw={json.loads(raw_tags)}" if raw_tags else ""
                print(f"     {form:<30} {tags or ''}{raw}")
            if len(forms) > 60:
                print(f"     … {len(forms) - 60} more")


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
