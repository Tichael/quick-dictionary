# Quick Dictionary

A small offline dictionary and conjugator. Press a keyboard shortcut and a
small window opens. Type a word to see how it's spelled, its plural, its
definitions and, for verbs, its full conjugation. Every installed language
is searched at once, so there's nothing to switch.

Status: early. The data pipeline and the search core work, with a
command-line tool to try them. The desktop app (Tauri, Windows first) comes
next.

## Repository layout

| Path        | What                                                         |
|-------------|--------------------------------------------------------------|
| `pipeline/` | Python scripts that turn Wiktionary dumps into language packs |
| `crates/qd-core/` | Rust library: opens packs, searches and ranks results |
| `crates/qd-cli/`  | `qd`, a command-line tool on top of `qd-core`          |
| `spec/`     | Language pack format and shared test vectors                 |

## Building language packs

Word data comes from [Wiktionary](https://www.wiktionary.org/), as extracted
by [wiktextract](https://github.com/tatuylonen/wiktextract) and published on
[kaikki.org](https://kaikki.org/). Each language uses its own Wiktionary
edition, so definitions are in the word's own language. Word frequencies,
used to rank results, come from [wordfreq](https://github.com/rspeer/wordfreq).

The pipeline only needs Python 3.11+ and its standard library.

```sh
cd pipeline
mkdir -p raw

# Word frequency lists (~1.5 MB each)
curl -o raw/large_fr.msgpack.gz https://raw.githubusercontent.com/rspeer/wordfreq/master/wordfreq/data/large_fr.msgpack.gz
curl -o raw/large_en.msgpack.gz https://raw.githubusercontent.com/rspeer/wordfreq/master/wordfreq/data/large_en.msgpack.gz

# French, from the French Wiktionary (~700 MB compressed, all languages; French is filtered out)
curl -o raw/fr-extract.jsonl.gz https://kaikki.org/dictionary/downloads/fr/fr-extract.jsonl.gz
python -m qd_pipeline.build --lang fr raw/fr-extract.jsonl.gz --freq raw/large_fr.msgpack.gz -o ../packs/fr.db

# English, from the English Wiktionary (English words only, ~3.3 GB uncompressed)
curl -o raw/en-English.jsonl https://kaikki.org/dictionary/English/kaikki.org-dictionary-English.jsonl
python -m qd_pipeline.build --lang en raw/en-English.jsonl --freq raw/large_en.msgpack.gz -o ../packs/en.db
```

Each build takes a few minutes and produces a pack of about 250–300 MB.

Check a pack's contents, or look up words the way the app will:

```sh
python -m qd_pipeline.inspect_pack ../packs/fr.db              # stats
python -m qd_pipeline.inspect_pack ../packs/fr.db allions coeur
```

## Looking words up from the terminal

```sh
cargo run --release -p qd -- acceuil "que nous allions" went
cargo run --release -p qd -- --full aller      # every definition and tense
cargo run --release -p qd -- --complete conjug # completions
```

`qd` searches every pack in `./packs` (or `--packs DIR`, or `$QD_PACKS`).
The first run builds a search index for each pack, which takes about a
second, and caches it in `packs/.cache`.

## Tests

```sh
cd pipeline && python -m unittest   # pipeline
cargo test                          # search core, from the repository root
```

The Rust tests search small packs in `crates/qd-core/tests/packs`, built by
the pipeline from its fixtures. After changing the fixtures or the pack
format, rebuild them with `python make_test_packs.py` in `pipeline/`.

The pack schema is described in [`spec/pack-format.md`](spec/pack-format.md).

## License

The code is MIT-licensed (see `LICENSE`). Language packs contain text from
Wiktionary and frequencies from wordfreq, and are licensed
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).
