"""Build the small packs that qd-core's tests search, from tests/fixtures.

Usage (from pipeline/):
    python make_test_packs.py

Rerun it after changing the fixtures or the pack format, and commit the
resulting crates/qd-core/tests/packs/*.db files.
"""

from pathlib import Path

from qd_pipeline.build import build

HERE = Path(__file__).parent
OUT = HERE.parent / "crates" / "qd-core" / "tests" / "packs"

# Made-up frequencies (Zipf values), chosen to exercise ranking.
ZIPF = {
    "en": {"go": 6.0, "went": 5.5, "color": 5.2, "colour": 4.6, "receive": 5.0,
           "recieve": 2.9, "accommodate": 4.0, "child": 5.3, "children": 5.5},
    "fr": {"lancer": 4.5, "lance": 4.2, "ecole": 5.3, "accueil": 4.7,
           "acceuil": 2.7, "coeur": 5.1},
}

for lang, zipf in ZIPF.items():
    stats = build([HERE / "tests" / "fixtures" / f"{lang}-sample.jsonl"], lang, OUT / f"{lang}.db", zipf=zipf)
    print(lang, stats)
