//! Offline dictionary lookups over Quick Dictionary language packs.
//!
//! A [`Dictionary`] opens every pack in a directory and searches them all at
//! once, so the user never picks a language. See `spec/pack-format.md` for
//! the pack format.

mod entry;
mod language;
mod normalize;
mod pack;
mod search;
mod spelling;

use std::path::PathBuf;

pub use entry::{Entry, FormGroup, Sense};
pub use normalize::{fold_case, normalize};
pub use pack::{KeyMatch, MatchKind, Pack};
pub use search::{Dictionary, MatchedForm, SearchResult, Suggestion};
pub use spelling::{edit_distance, spelling_cost};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("corrupt entry data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("search index error: {0}")]
    Fst(#[from] fst::Error),
    #[error("query too long for typo search: {0}")]
    Levenshtein(#[from] fst::automaton::LevenshteinError),
    #[error("{0}: not a valid language pack ({1})")]
    InvalidPack(PathBuf, String),
}
