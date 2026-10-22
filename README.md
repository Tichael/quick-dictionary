# Quick Dictionary

A small offline dictionary and conjugator. Press a keyboard shortcut and a
small window opens. Type a word to see how it's spelled, its plural, its
definitions and, for verbs, its full conjugation. Every installed language
is searched at once, so there's nothing to switch.

Status: early. The data pipeline works. The search core and the desktop app
(Tauri, Windows first) come next.

## Repository layout

| Path        | What                                                         |
|-------------|--------------------------------------------------------------|
| `pipeline/` | Python scripts that turn Wiktionary dumps into language packs |
| `spec/`     | Language pack format and shared normalization test vectors   |

## Building language packs

Word data comes from [Wiktionary](https://www.wiktionary.org/), as extracted
by [wiktextract](https://github.com/tatuylonen/wiktextract) and published on
[kaikki.org](https://kaikki.org/). Each language uses its own Wiktionary
edition, so definitions are in the word's own language.

The pipeline only needs Python 3.11+ and its standard library.

```sh
cd pipeline
mkdir -p raw

# French, from the French Wiktionary (~700 MB compressed, all languages; French is filtered out)
curl -o raw/fr-extract.jsonl.gz https://kaikki.org/dictionary/downloads/fr/fr-extract.jsonl.gz
python -m qd_pipeline.build --lang fr raw/fr-extract.jsonl.gz -o ../packs/fr.db

# English, from the English Wiktionary (English words only, ~3.3 GB uncompressed)
curl -o raw/en-English.jsonl https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl
python -m qd_pipeline.build --lang en raw/en-English.jsonl -o ../packs/en.db
```

Check a pack's contents, or look up words the way the app will:

```sh
python -m qd_pipeline.inspect_pack ../packs/fr.db              # stats
python -m qd_pipeline.inspect_pack ../packs/fr.db allions coeur
```

Run the tests:

```sh
cd pipeline
python -m unittest
```

The pack schema is described in [`spec/pack-format.md`](spec/pack-format.md).

## License

The code is MIT-licensed (see `LICENSE`). Language packs contain text from
Wiktionary and are licensed [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
