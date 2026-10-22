# Language pack format (schema version 1)

A language pack is one SQLite file per language (`en.db`, `fr.db`), built by
`pipeline/` from a wiktextract dump. The app opens every installed pack
read-only and searches them all at once.

Pack content comes from Wiktionary and is licensed CC BY-SA 4.0. That is why
packs are distributed separately from the MIT-licensed code.

## Tables

### `meta(key, value)`

| key              | example                                     |
|------------------|---------------------------------------------|
| `schema_version` | `1`                                         |
| `lang`           | `fr`                                        |
| `lang_name`      | `Français`                                  |
| `source`         | `Wiktionary, via wiktextract (kaikki.org)`  |
| `source_files`   | `fr-extract.jsonl.gz`                       |
| `license`        | `CC BY-SA 4.0 (…)`                          |
| `built_at`       | `2026-10-22T14:00:00+00:00`                 |

### `entries(id, word, pos, ipa, tags, data)`

One row per word *and* part of speech: "lance" (noun) and "lancer" (verb)
are separate entries. `pos` uses wiktextract's English codes (`noun`,
`verb`, `adj`, …). `ipa` is the first pronunciation listed. `tags` is a
space-separated list (`feminine`, …).

`data` is zlib-compressed UTF-8 JSON. The app always reads an entry as a
whole, and compression shrinks conjugation tables about 7×:

```json
{
  "senses": [
    {
      "glosses": ["Se déplacer jusqu'à un endroit."],
      "tags": ["figuratively"],
      "examples": ["…"],
      "form_of": ["…"]
    }
  ],
  "forms": [
    {"tags": ["indicative", "present"],
     "forms": ["je vais", "tu vas", "il/elle/on va", "nous allons", "vous allez", "ils/elles vont"]},
    {"raw_tags": ["Passé simple"], "forms": ["…"]}
  ]
}
```

Only `glosses` and `forms` (inside a group) are always present. The other
keys are left out when empty.

- **Senses** are the definitions, in Wiktionary order. `glosses` goes from
  general to specific: a sub-sense lists its parent first. Display the last
  item, with the earlier ones as context. `examples` holds at most 2
  sentences. `form_of` lists the headwords this sense is an inflection of.
- **Forms** are plurals, feminines and full conjugation tables, in source
  order. Consecutive forms with the same tags are grouped. Conjugation
  forms carry only mood and tense tags (`indicative present`), and the
  person is written into the form itself (`je vais`). When the extractor
  could not map a header to a tag, the original header is in `raw_tags`.

### `lookup(key, entry_id, kind, form_ord, note)`

The search index (primary key `(key, entry_id)`). `key` is a normalized
search string (see below). `note` is only set on form-of rows, and holds
the gloss that made the link ("plural of goose").

| kind | meaning                                                         |
|------|-----------------------------------------------------------------|
| 0    | headword: the key is the entry's own word                       |
| 1    | inflected form: `form_ord` is the matched form's index, counted across groups |
| 2    | form-of: the key is a word whose sense says it's a form of this one |
| 3    | redirect: a Wiktionary redirect page title points to this entry |

There is at most one row per (key, entry). When several kinds apply, the
lowest one is kept.

Wiktionary also has a page for most inflected forms ("lance": *first
person singular of lancer*). When every sense of such an entry is a form-of
link, and its base word is in the pack, the entry is not stored. It becomes
a kind-2 lookup row instead, since the base word's forms already describe
it.

## Key normalization

The keys are built by `pipeline/qd_pipeline/normalize.py`, and the app must
normalize queries the same way:

1. NFKD, then drop combining marks
2. lowercase
3. œ→oe, æ→ae, ß→ss, ø→o, đ→d, ł→l
4. apostrophes ’ ‘ ʼ ′ → `'`; dashes ‐ ‑ – — → `-`
5. collapse whitespace and trim

`spec/normalization-vectors.json` lists `[input, expected]` pairs that both
implementations test against.

For form keys, each language also removes decorations that users don't
type. French removes subject, reflexive and `que` pronouns (`que nous
lancions` → `lancions`) and imperative suffixes (`abats-toi` → `abats`).
Forms that still contain a space after that get no key. They are compound
forms (`j'étais allé`, `more absolutely`), and their individual words are
already listed.
