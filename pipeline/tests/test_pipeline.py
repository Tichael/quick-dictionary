import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

from qd_pipeline.build import KIND_FORM, KIND_FORM_OF, KIND_HEADWORD, KIND_REDIRECT, build, decode_data
from qd_pipeline.languages import LANGUAGES
from qd_pipeline.normalize import normalize

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).parent / "fixtures"


class NormalizeTest(unittest.TestCase):
    def test_shared_vectors(self):
        vectors = json.loads((ROOT / "spec" / "normalization-vectors.json").read_text("utf-8"))
        for raw, expected in vectors:
            with self.subTest(raw=raw):
                self.assertEqual(normalize(raw), expected)


class FrenchKeyTest(unittest.TestCase):
    def test_strips_pronouns(self):
        key = LANGUAGES["fr"]["form_key"]
        cases = {
            "je lance": "lance",
            "j'aime": "aime",
            "il/elle/on lance": "lance",
            "que nous lancions": "lancions",
            "qu'ils/elles lancent": "lancent",
            "je m'abats": "abats",
            "nous nous lavons": "lavons",
            "abats-toi": "abats",
            "lancer": "lancer",
            "porte-monnaie": "porte-monnaie",
        }
        for form, expected in cases.items():
            with self.subTest(form=form):
                self.assertEqual(key(form), expected)


class BuildTest(unittest.TestCase):
    def build_pack(self, lang: str) -> sqlite3.Connection:
        tmp = Path(tempfile.mkdtemp())
        self.stats = build([FIXTURES / f"{lang}-sample.jsonl"], lang, tmp / f"{lang}.db")
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
        self.assertEqual(self.stats["entries"], 6)
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
            [("go", "verb", KIND_FORM)],
        )
        self.assertEqual(self.find(conn, "children"), [("child", "noun", KIND_FORM)])
        self.assertEqual(self.find(conn, "cafe"), [("café", "noun", KIND_HEADWORD)])
        (blob,) = conn.execute("SELECT data FROM entries WHERE word = 'go'").fetchone()
        forms = [f for g in decode_data(blob)["forms"] for f in g["forms"]]
        self.assertEqual(forms, ["goes", "going", "went", "gone"])  # template rows dropped

    def test_form_of_without_forms_table(self):
        # "goose" lists no forms; only the "geese" entry links back to it.
        conn = self.build_pack("en")
        self.assertEqual(self.find(conn, "geese"), [("goose", "noun", KIND_FORM_OF)])
        (note,) = conn.execute("SELECT note FROM lookup WHERE key = 'geese'").fetchone()
        self.assertEqual(note, "plural of goose")


if __name__ == "__main__":
    unittest.main()
