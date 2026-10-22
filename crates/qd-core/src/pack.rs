//! One language pack: a read-only SQLite file plus an in-memory index of
//! its search keys for typo-tolerant and prefix search.

use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::ZlibDecoder;
use fst::automaton::{Levenshtein, Str};
use fst::{Automaton, IntoStreamer, Map, MapBuilder, Streamer};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::Error;
use crate::entry::{Entry, EntryData};
use crate::spelling::{edit_distance, spelling_cost};

pub const SCHEMA_VERSION: &str = "1";

/// How a search key reaches an entry (`lookup.kind` in the pack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    /// The key is the entry's own word.
    Headword,
    /// The key is one of the entry's inflected forms.
    Form,
    /// The key is a word Wiktionary describes as a form of this entry.
    FormOf,
    /// A Wiktionary redirect points from the key to this entry.
    Redirect,
}

impl MatchKind {
    fn from_db(kind: i64) -> MatchKind {
        match kind {
            0 => MatchKind::Headword,
            1 => MatchKind::Form,
            2 => MatchKind::FormOf,
            _ => MatchKind::Redirect,
        }
    }
}

/// A `lookup` row: one way a key reaches an entry.
#[derive(Debug, Clone)]
pub struct Hit {
    pub entry_id: i64,
    pub kind: MatchKind,
    pub form_ord: Option<usize>,
    pub note: Option<String>,
}

/// A search key close to the query, from the fuzzy index.
#[derive(Debug, Clone)]
pub struct KeyMatch {
    pub key: String,
    /// Plain edits between the query and `key`.
    pub distance: u32,
    /// Weighted cost of those edits, in half edits (see [`spelling_cost`]).
    pub cost: u32,
    pub freq: u16,
}

pub struct Pack {
    pub path: PathBuf,
    pub lang: String,
    pub lang_name: String,
    /// When the pack was built (ISO 8601), from its `meta` table.
    pub built_at: String,
    conn: Connection,
    /// Every search key, mapped to its frequency.
    keys: Map<Vec<u8>>,
}

impl Pack {
    /// Opens a pack. The typo-tolerant index is built from the pack's keys,
    /// which takes about a second; with a `cache_dir`, it is saved there
    /// and reused while the pack is unchanged.
    pub fn open(path: &Path, cache_dir: Option<&Path>) -> Result<Pack, Error> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let meta = |key: &str| -> Result<String, Error> {
            conn.query_row("SELECT value FROM meta WHERE key = ?", [key], |r| r.get(0))
                .optional()?
                .ok_or_else(|| Error::InvalidPack(path.to_owned(), format!("missing meta {key}")))
        };
        let version = meta("schema_version")?;
        if version != SCHEMA_VERSION {
            return Err(Error::InvalidPack(
                path.to_owned(),
                format!("schema version {version}, expected {SCHEMA_VERSION}"),
            ));
        }
        let lang = meta("lang")?;
        let lang_name = meta("lang_name")?;
        let built_at = meta("built_at").unwrap_or_default();
        let cache_file = cache_dir.map(|dir| {
            let version: String = built_at
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .collect();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            dir.join(format!("{stem}.{version}.keys.fst"))
        });
        let keys = match cache_file.as_deref().and_then(load_key_index) {
            Some(keys) => keys,
            None => {
                let keys = build_key_index(&conn)?;
                if let Some(file) = &cache_file {
                    // The cache is an optimization: failing to write it is fine.
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
                    let _ = save_key_index(file, &stem, keys.as_fst().as_bytes());
                }
                keys
            }
        };
        Ok(Pack {
            path: path.to_owned(),
            lang,
            lang_name,
            built_at,
            conn,
            keys,
        })
    }

    /// Number of entries (one per word and part of speech).
    pub fn entry_count(&self) -> Result<u64, Error> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))?;
        Ok(count as u64)
    }

    /// Number of distinct search keys.
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Frequency of a key, if the pack has it.
    pub fn key_freq(&self, key: &str) -> Option<u16> {
        self.keys.get(key).map(|f| f as u16)
    }

    /// All the ways `key` reaches an entry.
    pub fn hits(&self, key: &str) -> Result<Vec<Hit>, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT entry_id, kind, form_ord, note FROM lookup WHERE key = ?")?;
        let rows = stmt.query_map([key], |r| {
            Ok(Hit {
                entry_id: r.get(0)?,
                kind: MatchKind::from_db(r.get(1)?),
                form_ord: r.get::<_, Option<i64>>(2)?.map(|o| o as usize),
                note: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn entry(&self, id: i64) -> Result<Entry, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT word, pos, ipa, tags, freq, data FROM entries WHERE id = ?")?;
        let (word, pos, ipa, tags, freq, blob): (
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            Vec<u8>,
        ) = stmt.query_row(params![id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })?;
        let mut json = Vec::new();
        ZlibDecoder::new(blob.as_slice()).read_to_end(&mut json)?;
        let data: EntryData = serde_json::from_slice(&json)?;
        Ok(Entry {
            id,
            word,
            pos,
            ipa,
            tags: tags
                .map(|t| t.split(' ').map(String::from).collect())
                .unwrap_or_default(),
            freq: freq as u16,
            senses: data.senses,
            forms: data.forms,
        })
    }

    /// Keys within `max_distance` edits of `key` (excluding `key` itself).
    pub fn fuzzy(&self, key: &str, max_distance: u32) -> Result<Vec<KeyMatch>, Error> {
        let automaton = Levenshtein::new(key, max_distance)?;
        let mut stream = self.keys.search(automaton).into_stream();
        let mut matches = Vec::new();
        while let Some((candidate, freq)) = stream.next() {
            let candidate = String::from_utf8_lossy(candidate).into_owned();
            if candidate != key {
                let distance = edit_distance(key, &candidate);
                let cost = spelling_cost(key, &candidate);
                matches.push(KeyMatch {
                    key: candidate,
                    distance,
                    cost,
                    freq: freq as u16,
                });
            }
        }
        Ok(matches)
    }

    /// The `limit` most frequent keys starting with `prefix`.
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<KeyMatch> {
        let mut stream = self
            .keys
            .search(Str::new(prefix).starts_with())
            .into_stream();
        let mut matches = Vec::new();
        while let Some((candidate, freq)) = stream.next() {
            matches.push(KeyMatch {
                key: String::from_utf8_lossy(candidate).into_owned(),
                distance: 0,
                cost: 0,
                freq: freq as u16,
            });
        }
        matches.sort_by(|a, b| b.freq.cmp(&a.freq).then_with(|| a.key.cmp(&b.key)));
        matches.truncate(limit);
        matches
    }
}

fn build_key_index(conn: &Connection) -> Result<Map<Vec<u8>>, Error> {
    // `keys` is a WITHOUT ROWID table keyed by `key`, so this scan is
    // already in the byte order the FST builder needs.
    let mut stmt = conn.prepare("SELECT key, freq FROM keys ORDER BY key")?;
    let mut rows = stmt.query([])?;
    let mut builder = MapBuilder::memory();
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let freq: i64 = row.get(1)?;
        builder.insert(key, freq as u64)?;
    }
    Ok(builder.into_map())
}

fn load_key_index(file: &Path) -> Option<Map<Vec<u8>>> {
    Map::new(std::fs::read(file).ok()?).ok()
}

/// Writes the index to `file`, and deletes indexes of older versions of
/// the same pack.
fn save_key_index(file: &Path, stem: &str, bytes: &[u8]) -> std::io::Result<()> {
    let dir = file.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, file)?;
    let prefix = format!("{stem}.");
    for old in std::fs::read_dir(dir)?.filter_map(Result::ok) {
        let name = old.file_name().to_string_lossy().into_owned();
        let version = name
            .strip_prefix(&prefix)
            .and_then(|n| n.strip_suffix(".keys.fst"));
        let same_pack = version.is_some_and(|v| v.chars().all(|c| c.is_ascii_alphanumeric()));
        if same_pack && old.path() != file {
            let _ = std::fs::remove_file(old.path());
        }
    }
    Ok(())
}
