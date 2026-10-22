"""Set the app version in crates/qd-app/Cargo.toml and Cargo.lock.

Usage: python scripts/set_version.py 1.2.3

semantic-release runs this when it cuts a release (see .releaserc.json).
Tauri reads the app version from that Cargo.toml.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGE = "quick-dictionary"


def set_version(version: str, root: Path = ROOT) -> None:
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise SystemExit(f"not a plain x.y.z version: {version!r}")

    manifest = root / "crates" / "qd-app" / "Cargo.toml"
    text = manifest.read_text("utf-8")
    # The first `version = ` line is the [package] one.
    text, count = re.subn(r'(?m)^version = "[^"]*"', f'version = "{version}"', text, count=1)
    if count != 1:
        raise SystemExit(f"no version line in {manifest}")
    manifest.write_text(text, "utf-8")

    lock = root / "Cargo.lock"
    text = lock.read_text("utf-8")
    pattern = rf'(\[\[package\]\]\nname = "{PACKAGE}"\nversion = )"[^"]*"'
    text, count = re.subn(pattern, rf'\g<1>"{version}"', text)
    if count != 1:
        raise SystemExit(f"no {PACKAGE} entry in {lock}")
    lock.write_text(text, "utf-8")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    set_version(sys.argv[1])
