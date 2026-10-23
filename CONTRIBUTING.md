# Contributing

Notes for building, testing and releasing Quick Dictionary. For what the app
does and how to install it, see the [README](README.md).

## Repository layout

| Path              | What                                                                 |
|-------------------|----------------------------------------------------------------------|
| `pipeline/`       | Python scripts that turn Wiktionary dumps into language packs        |
| `crates/qd-core/` | Rust library: opens packs, searches and ranks results                |
| `crates/qd-cli/`  | `qd`, a command-line tool on top of `qd-core`                        |
| `crates/qd-app/`  | The desktop app (Tauri): tray, shortcut, lookup and settings windows |
| `spec/`           | Language pack format and shared test vectors                         |

The pack schema is described in [`spec/pack-format.md`](spec/pack-format.md).

## Pull requests

Pull requests are squash-merged, so their title becomes the commit that
semantic-release reads: titles follow
[Conventional Commits](https://www.conventionalcommits.org/) (`feat: …`,
`fix(app): …`), and the **PR title** workflow checks it.

## Running the desktop app

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
by `python -m qd_pipeline.release` (see below).

Debug builds open a console window that shows the app's log, including script
errors from the windows. They don't register themselves to start at login
unless you change that setting.

### Translating the interface

Add a block of strings to `crates/qd-app/ui/i18n.js` and
`crates/qd-app/src/i18n.rs`, and an option to the language list in
`ui/settings.html`.

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

The app downloads its dictionaries from this repository's
[`packs` release](https://github.com/Tichael/quick-dictionary/releases/tag/packs).

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
cargo test                          # search core and app, from the repository root
node --test crates/qd-app/ui-tests/*.test.mjs   # interface strings
```

The Rust tests search small packs in `crates/qd-core/tests/packs`, built by
the pipeline from its fixtures. After changing the fixtures or the pack
format, rebuild them with `python make_test_packs.py` in `pipeline/`.

## Continuous integration and releases

GitHub Actions runs five workflows (in `.github/workflows/`):

- **CI**, on every pull request and push to `main`: the pipeline tests, the
  interface strings check, `cargo fmt`, and Clippy and the tests of the
  search core and CLI on Linux. (The desktop app would need WebKitGTK there,
  so it is checked in the Windows build.)
- **Windows build**, on pull requests and pushes to `main` that change the
  Rust code or the app (Windows minutes cost twice Linux minutes on private
  repositories), or by hand: runs Clippy and the tests of the app, builds
  it and uploads the MSI installer and the plain `.exe` as the
  `quick-dictionary-windows` artifact (GitHub zips artifacts; releases are
  not zipped).
- **Release**, on every push to `main`:
  [semantic-release](https://semantic-release.gitbook.io/) reads the
  [Conventional Commits](https://www.conventionalcommits.org/) since the last
  release (`fix:` → patch, `feat:` → minor, `feat!:` or `BREAKING CHANGE:` →
  major; `docs:`, `ci:`, `chore:`… release nothing). When there is something
  to release, it sets the version in `crates/qd-app/Cargo.toml`, tags
  `v<version>`, creates the GitHub release with generated notes, and a
  Windows job attaches the MSI installer, signed for the app's updater,
  with the `latest.json` that tells installed copies about it. Configured
  in `.releaserc.json`.
- **PR title**, on pull requests: checks the title follows Conventional
  Commits.
- **Language packs**, once a month or by hand from the Actions tab: builds
  `fr.db` and `en.db` from the latest Wiktionary extracts and publishes them,
  compressed, with a `manifest.json` to the `packs` release (about 12
  minutes).

The app downloads release files without signing in, which only works while
the repository is public.

### App updates

The app checks for a newer version 10 seconds after it starts, then once a
day, and asks before installing it (Settings → About also has a button).
It reads `latest.json` from the latest GitHub release, and only installs an
installer signed with the updater key. To set that key up, once:

```sh
npx @tauri-apps/cli@2 signer generate -w ~/.tauri/quick-dictionary.key
```

Put the public key (the `.key.pub` file's content) in
`plugins.updater.pubkey` in `crates/qd-app/tauri.conf.json`, and add the
private key and its password as the repository secrets
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Keep
a copy of the private key: installed copies only accept updates signed with
it. The release workflow refuses to build while the public key is missing.

`QD_UPDATES_URL` points the app at another `latest.json` (testing).
