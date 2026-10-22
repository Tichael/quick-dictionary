"""Prepare built packs for publishing: compress them and write a manifest.

Usage:
    python -m qd_pipeline.release packs/fr.db packs/en.db -o release/

Writes `<lang>.db.gz` for each pack and `manifest.json`, which the app reads
to list the packs it can download (see spec/pack-format.md).
"""

import argparse
import gzip
import hashlib
import json
import shutil
import sqlite3
from pathlib import Path

MANIFEST_VERSION = 1


def _meta(path: Path) -> dict[str, str]:
    conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    try:
        meta = dict(conn.execute("SELECT key, value FROM meta"))
        (meta["entries"],) = conn.execute("SELECT COUNT(*) FROM entries").fetchone()
    finally:
        conn.close()
    return meta


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def prepare(packs: list[Path], out: Path) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    entries = []
    for pack in packs:
        meta = _meta(pack)
        compressed = out / f"{meta['lang']}.db.gz"
        with open(pack, "rb") as src, gzip.open(compressed, "wb", compresslevel=9) as dst:
            shutil.copyfileobj(src, dst, 1 << 20)
        entries.append(
            {
                "lang": meta["lang"],
                "lang_name": meta["lang_name"],
                "file": compressed.name,
                "size": compressed.stat().st_size,
                "db_size": pack.stat().st_size,
                "sha256": _sha256(pack),
                "schema_version": meta["schema_version"],
                "built_at": meta["built_at"],
                "entries": meta["entries"],
            }
        )
    manifest = {"manifest_version": MANIFEST_VERSION, "packs": sorted(entries, key=lambda e: e["lang"])}
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", "utf-8")
    return manifest


def main(argv=None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("packs", nargs="+", type=Path, help="built pack files")
    parser.add_argument("-o", "--output", type=Path, required=True, help="folder to write to")
    args = parser.parse_args(argv)
    manifest = prepare(args.packs, args.output)
    print(json.dumps(manifest, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
