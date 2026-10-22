"""Search-key normalization.

Every lookup key stored in a pack is produced by `normalize()`. The app
normalizes the user's query with the same rules, so both implementations
must agree: `spec/normalization-vectors.json` holds shared test vectors.

Rules, applied in order:
  1. Unicode NFKD decomposition, then drop combining marks (é -> e, ç -> c).
  2. Lowercase.
  3. Expand ligatures and special letters (œ -> oe, æ -> ae, ß -> ss).
  4. Unify apostrophes (’ ‘ ʼ ′ -> ') and dashes (‐ ‑ – — -> -).
  5. Collapse whitespace runs to one space and trim.
"""

import re
import unicodedata

_CHAR_MAP = str.maketrans(
    {
        "œ": "oe",
        "æ": "ae",
        "ß": "ss",
        "ø": "o",
        "đ": "d",
        "ł": "l",
        "’": "'",
        "‘": "'",
        "ʼ": "'",
        "′": "'",
        "‐": "-",
        "‑": "-",
        "–": "-",
        "—": "-",
    }
)

_WHITESPACE = re.compile(r"\s+")


def normalize(text: str) -> str:
    decomposed = unicodedata.normalize("NFKD", text)
    stripped = "".join(c for c in decomposed if not unicodedata.combining(c))
    mapped = stripped.lower().translate(_CHAR_MAP)
    return _WHITESPACE.sub(" ", mapped).strip()
