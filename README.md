# Quick Dictionary

A small, offline dictionary and conjugator that is one keyboard shortcut away.

Press **Ctrl+Alt+D** from any app, type a word, and see its spelling, plural,
definitions and, for verbs, its full conjugation. All your installed languages
are searched at once, so there is nothing to switch. Everything works without
an internet connection once the dictionaries are downloaded.

> **Status:** early, but usable. Windows only for now (Linux and macOS may
> follow).

## Install

1. Go to the [latest release](https://github.com/Tichael/quick-dictionary/releases/latest).
2. Download the `.msi` installer and run it.
3. Quick Dictionary starts in the system tray and opens its settings the first
   time.

Windows 10 and 11 are supported. Nothing else needs to be installed.

## Get your dictionaries

On first launch, the settings window lists the available dictionaries
(currently **French** and **English**). Click **Download** next to the ones
you want. Each is a few hundred MB, and is stored on your computer so lookups
are instant and offline.

The dictionaries are rebuilt every month from Wiktionary. Definitions are in
the word's own language, so French words get French definitions.

## Use it

| Do this                          | To                                         |
|----------------------------------|--------------------------------------------|
| Press **Ctrl+Alt+D**             | Open the lookup window from anywhere       |
| Type a word                      | See its definitions, plural, conjugation   |
| **↑ / ↓**                        | Move between results                       |
| **Enter**, or click a headword   | Copy the selected word                     |
| **Esc**                          | Hide the window                            |

Conjugated forms work too: searching for `allions` finds *aller*.

Right-click the tray icon to open **Settings**, where you can change:

- the shortcut
- whether the app starts with Windows
- the interface language (English or French, or follow the system)
- the window's behavior and theme
- your installed dictionaries (you can also add a dictionary file by hand)

## Updates

The app checks for new versions in the background and asks before installing
one. You can also check by hand in **Settings → About**.

## Something wrong?

Please [open an issue](https://github.com/Tichael/quick-dictionary/issues).
Tell us your Windows version and what you searched for.

## For developers

Building the app, the dictionary pipeline, the command-line tool and the
release process are described in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

The code is MIT-licensed (see [`LICENSE`](LICENSE)). The dictionaries contain
text from [Wiktionary](https://www.wiktionary.org/) and word frequencies from
[wordfreq](https://github.com/rspeer/wordfreq), and are licensed
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/).

## AI usage

Claude Code was used to make this project. If you don't like that, just don't
use it.
