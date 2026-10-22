"""Write the latest.json the app's updater reads (see crates/qd-app/src/updates.rs).

Usage: python scripts/updater_manifest.py VERSION INSTALLER URL -o latest.json

INSTALLER is the installer built with updater artifacts: its signature,
INSTALLER.sig, sits next to it. URL is where the release serves it. The
release workflow runs this and attaches latest.json to the release; the app
asks for it at releases/latest/download/latest.json.
"""

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def manifest(version: str, signature: str, url: str) -> dict:
    return {
        "version": version,
        "pub_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "platforms": {"windows-x86_64": {"signature": signature, "url": url}},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("version")
    parser.add_argument("installer", type=Path)
    parser.add_argument("url")
    parser.add_argument("-o", "--output", type=Path, required=True)
    args = parser.parse_args()

    signature_file = args.installer.with_name(args.installer.name + ".sig")
    signature = signature_file.read_text("utf-8").strip()
    if not signature:
        raise SystemExit(f"{signature_file} is empty")
    data = manifest(args.version, signature, args.url)
    args.output.write_text(json.dumps(data, indent=2) + "\n", "utf-8")


if __name__ == "__main__":
    main()
