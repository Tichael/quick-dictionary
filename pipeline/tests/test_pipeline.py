import gzip
import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from qd_pipeline.build import KIND_FORM, KIND_FORM_OF, KIND_HEADWORD, KIND_REDIRECT, build, decode_data
from qd_pipeline.languages import LANGUAGES
from qd_pipeline.normalize import normalize
from qd_pipeline.wordfreq import load_zipf

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).parent / "fixtures"


class NormalizeTest(unittest.TestCase):
    def test_shared_vectors(self):
        vectors = json.loads((ROOT / "spec" / "normalization-vectors.json").read_text("utf-8"))
        for raw, expected in vectors:
            with self.subTest(raw=raw):
                self.assertEqual(normalize(raw), expected)


def _msgpack(value) -> bytes:
    """Encode the small msgpack subset wordfreq files use."""
    if isinstance(value, str):
        raw = value.encode("utf-8")
        return (bytes([0xA0 | len(raw)]) if len(raw) < 32 else bytes([0xD9, len(raw)])) + raw
    if isinstance(value, int):
        return bytes([value])
    if isinstance(value, list):
        head = bytes([0x90 | len(value)]) if len(value) < 16 else bytes([0xDC]) + len(value).to_bytes(2, "big")
        return head + b"".join(_msgpack(v) for v in value)
    if isinstance(value, dict):
        return bytes([0x80 | len(value)]) + b"".join(_msgpack(k) + _msgpack(v) for k, v in value.items())
    raise TypeError(value)


class WordfreqTest(unittest.TestCase):
    def test_load_zipf(self):
        buckets = [[] for _ in range(300)]
        buckets[0] = ["de"]
        buckets[200] = ["été"]
        buckets[250] = ["ete", "cheval"]
        path = Path(tempfile.mkdtemp()) / "large_fr.msgpack.gz"
        path.write_bytes(gzip.compress(_msgpack([{"format": "cB", "version": 1}] + buckets)))
        # "ete" keeps the value of the commoner "été".
        self.assertEqual(load_zipf(path), {"de": 9.0, "ete": 7.0, "cheval": 6.5})


class FormKeyTest(unittest.TestCase):
    def test_shared_vectors(self):
        vectors = json.loads((ROOT / "spec" / "form-key-vectors.json").read_text("utf-8"))
        for lang, cases in vectors.items():
            key = LANGUAGES[lang]["form_key"]
            for form, expected in cases:
                with self.subTest(lang=lang, form=form):
                    self.assertEqual(key(form), expected)


class BuildTest(unittest.TestCase):
    ZIPF = {"go": 6.03, "went": 5.5, "lance": 4.2, "lancer": 4.5}

    def build_pack(self, lang: str) -> sqlite3.Connection:
        tmp = Path(tempfile.mkdtemp())
        self.stats = build(
            [FIXTURES / f"{lang}-sample.jsonl"], lang, tmp / f"{lang}.db", zipf=self.ZIPF
        )
        conn = sqlite3.connect(tmp / f"{lang}.db")
        self.addCleanup(conn.close)
        return conn

    def find(self, conn, query):
        return conn.execute(
            "SELECT e.word, e.pos, l.kind FROM lookup l JOIN entries e ON e.id = l.entry_id"
            " WHERE l.key = ? ORDER BY l.kind, e.id",
            (normalize(query),),
        ).fetchall()

    def test_french(self):
        conn = self.build_pack("fr")
        # Dropped: "school" (English), the empty "vide", and the verb form
        # "lance", which lancer's own forms already cover.
        self.assertEqual(self.stats["entries"], 7)
        self.assertEqual(self.stats["dropped_form_entries"], 1)
        self.assertEqual(self.find(conn, "ecole"), [("école", "noun", KIND_HEADWORD)])
        self.assertEqual(self.find(conn, "ECOLES"), [("école", "noun", KIND_FORM)])
        self.assertEqual(self.find(conn, "coeur"), [("cœur", "noun", KIND_HEADWORD)])
        self.assertEqual(self.find(conn, "lancons"), [("lancer", "verb", KIND_FORM)])
        self.assertEqual(self.find(conn, "lancâmes"), [("lancer", "verb", KIND_FORM)])
        self.assertEqual(self.find(conn, "abats"), [("s’abattre", "verb", KIND_FORM)])
        self.assertEqual(
            self.find(conn, "lance"),
            [("lance", "noun", KIND_HEADWORD), ("lancer", "verb", KIND_FORM)],
        )
        # A form whose base word is missing from the dump is kept as is.
        self.assertEqual(self.find(conn, "chevals"), [("chevals", "noun", KIND_HEADWORD)])
        # Compound tenses are displayed but are not lookup keys.
        self.assertEqual(self.find(conn, "ai lance"), [])
        self.assertEqual(self.find(conn, "school"), [])

        (blob,) = conn.execute("SELECT data FROM entries WHERE word = 'lancer'").fetchone()
        data = decode_data(blob)
        self.assertEqual(data["senses"][0]["examples"], ["Lancer une pierre.", "Lancer un javelot."])
        self.assertEqual(
            data["forms"][:3],
            [
                {"forms": ["lancer"], "tags": ["infinitive", "present"]},
                {"forms": ["avoir lancé"], "tags": ["infinitive", "past"]},
                {"forms": ["je lance", "nous lançons"], "tags": ["indicative", "present"]},
            ],
        )
        # The duplicated participle is stored once.
        self.assertEqual(sum(len(g["forms"]) for g in data["forms"]), 8)
        self.assertEqual(data["forms"][-1], {"forms": ["nous lançâmes"], "raw_tags": ["Passé simple"]})

    def test_french_redirect(self):
        # "Lancer" normalizes to the same key as "lancer", so it adds nothing new.
        conn = self.build_pack("fr")
        kinds = {kind for _, _, kind in self.find(conn, "Lancer")}
        self.assertNotIn(KIND_REDIRECT, kinds)
        self.assertEqual(self.find(conn, "école normale"), [("école", "noun", KIND_REDIRECT)])

    def test_english(self):
        conn = self.build_pack("en")
        self.assertEqual(
            self.find(conn, "went"),
            [("went", "noun", KIND_HEADWORD), ("Went", "name", KIND_HEADWORD), ("go", "verb", KIND_FORM)],
        )
        self.assertEqual(self.find(conn, "children"), [("child", "noun", KIND_FORM)])
        self.assertEqual(self.find(conn, "cafe"), [("café", "noun", KIND_HEADWORD)])
        (blob,) = conn.execute("SELECT data FROM entries WHERE word = 'go'").fetchone()
        forms = [f for g in decode_data(blob)["forms"] for f in g["forms"]]
        self.assertEqual(forms, ["goes", "going", "went", "gone"])  # template rows dropped

    def test_misspelling_links_to_correct_spelling(self):
        conn = self.build_pack("en")
        self.assertEqual(self.find(conn, "recieve"), [("receive", "verb", KIND_FORM_OF)])
        (note,) = conn.execute("SELECT note FROM lookup WHERE key = 'recieve'").fetchone()
        self.assertEqual(note, "Misspelling of receive.")
        (blob,) = conn.execute("SELECT data FROM entries WHERE word = 'receive'").fetchone()
        self.assertEqual(decode_data(blob)["forms"], [{"forms": ["receives"], "tags": ["present", "singular", "third-person"]}])

    def test_frequencies(self):
        conn = self.build_pack("en")
        self.assertEqual(conn.execute("SELECT freq FROM entries WHERE word = 'go'").fetchone(), (603,))
        self.assertEqual(conn.execute("SELECT freq FROM entries WHERE word = 'child'").fetchone(), (0,))
        keys = dict(conn.execute("SELECT key, freq FROM keys"))
        self.assertEqual(keys["went"], 550)
        self.assertEqual(keys["children"], 0)
        (distinct,) = conn.execute("SELECT COUNT(DISTINCT key) FROM lookup").fetchone()
        self.assertEqual(len(keys), distinct)

    def test_form_of_without_forms_table(self):
        # "goose" lists no forms; only the "geese" entry links back to it.
        conn = self.build_pack("en")
        self.assertEqual(self.find(conn, "geese"), [("goose", "noun", KIND_FORM_OF)])
        (note,) = conn.execute("SELECT note FROM lookup WHERE key = 'geese'").fetchone()
        self.assertEqual(note, "plural of goose")


if __name__ == "__main__":
    unittest.main()
