# Quick Dictionary

A small offline dictionary and conjugator. Press a keyboard shortcut and a
small window opens. Type a word to see how it's spelled, its plural, its
definitions and, for verbs, its full conjugation. Every installed language
is searched at once, so there's nothing to switch.

Status: early, but usable. The data pipeline, the search core and a first
version of the desktop app (Tauri, Windows first) work. The app downloads its
dictionaries from this repository's
[`packs` release](https://github.com/Tichael/quick-dictionary/releases/tag/packs),
which a workflow rebuilds every month.

## Repository layout

| Path        | What                                                         |
|-------------|--------------------------------------------------------------|
| `pipeline/` | Python scripts that turn Wiktionary dumps into language packs |
| `crates/qd-core/` | Rust library: opens packs, searches and ranks results |
| `crates/qd-cli/`  | `qd`, a command-line tool on top of `qd-core`          |
| `crates/qd-app/`  | The desktop app (Tauri): tray, shortcut, lookup and settings windows |
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

To publish packs where the app downloads them, `qd_pipeline.release`
compresses them and writes the manifest the app reads:

```sh
python -m qd_pipeline.release ../packs/fr.db ../packs/en.db -o ../release
```

Check a pack's contents, or look up words the way the app will:

```sh
python -m qd_pipeline.inspect_pack ../packs/fr.db              # stats
python -m qd_pipeline.inspect_pack ../packs/fr.db allions coeur
```

## The desktop app

The app lives in the tray. Press **Ctrl+Alt+D** (configurable) anywhere to
open the lookup window and type a word. Esc hides the window, ↑/↓ move between
results, Enter copies the selected word, and clicking a headword copies it too.
The tray menu also opens the settings window: shortcut, start with the system,
interface language (English or French, or the system's), window behavior,
theme, and the installed dictionaries.

To translate the app into another language, add a block of strings to
`crates/qd-app/ui/i18n.js` and `crates/qd-app/src/i18n.rs`, and an option to
the language list in `ui/settings.html`.

### Running it

You need [Rust](https://rustup.rs/) and, on Windows, the MSVC build tools.
WebView2 already ships with Windows 10 and 11.

```sh
cargo run -p quick-dictionary
```

The first launch opens the settings window so you can add dictionaries:
**Download** them from the list, or click **Add a dictionary file…** and pick
a pack you built (`fr.db`, `en.db`). Packs are kept in the app's data folder
(`%APPDATA%\io.github.tichael.quickdictionary\packs` on Windows).

For development, `QD_PACKS=path/to/packs` makes the app use another folder,
and `QD_PACKS_URL=http://…/` another download location: serve a folder made
by `python -m qd_pipeline.release` (see above).

Debug builds open a console window that shows the app's log, including script
errors from the windows. They don't register themselves to start at login
unless you change that setting.

### Building the installer

```sh
cargo install tauri-cli --version "^2" --locked
cd crates/qd-app
cargo tauri build
```

The MSI installer is written to `target/release/bundle/msi/`.

On Linux, building the app needs the WebKitGTK development packages
(`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`,
`libxdo-dev`). The rest of the project doesn't.

## Looking words up from the terminal

```sh
cargo run --release -p qd -- acceuil "que nous allions" went
cargo run --release -p qd -- --full aller      # every definition and tense
cargo run --release -p qd -- --complete conjug # completions
```

`qd` searches every pack in `./packs` (or `--packs DIR`, or `$QD_PACKS`).
The first run builds a search index for each pack, which takes about a
second, and caches it in `packs/.cache`.

## Continuous integration

GitHub Actions runs four workflows (in `.github/workflows/`):

- **CI**, on every pull request and push to `main`: the pipeline tests, the
  interface strings check, `cargo fmt`, Clippy and the Rust tests on Linux.
- **Windows build**, on pull requests and pushes to `main` that change the
  Rust code or the app (Windows minutes cost twice Linux minutes on private
  repositories), or by hand: builds the app and uploads the MSI installer
  and the plain `.exe` as the `quick-dictionary-windows` artifact (GitHub
  zips artifacts; releases are not zipped).
- **App release**, by hand from the Actions tab: builds the MSI installer
  and publishes it as the `v<version>` release, with the version read from
  `crates/qd-app/Cargo.toml`. To release, bump that version, merge, and run
  the workflow; it creates the tag.
- **Language packs**, once a month or by hand from the Actions tab: builds
  `fr.db` and `en.db` from the latest Wiktionary extracts and publishes them,
  compressed, with a `manifest.json` to the `packs` release (about 12 minutes).

The app downloads release files without signing in, which only works while
the repository is public.

## Tests

```sh
cd pipeline && python -m unittest   # pipeline
cargo test                          # search core and app, from the repository root
node --test crates/qd-app/ui-tests/*.test.mjs   # interface strings
```

The Rust tests search small packs in `crates/qd-core/tests/packs`, built by
the pipeline from its fixtures. After changing the fixtures or the pack
format, rebuild them with `python make_test_packs.py` in `pipeline/`.

The pack schema is described in [`spec/pack-format.md`](spec/pack-format.md).

## License

The code is MIT-licensed (see `LICENSE`). Language packs contain text from
Wiktionary and frequencies from wordfreq, and are licensed
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## AI Usage

Claude Code was used to make this project. If you don't like that, just don't use it.
