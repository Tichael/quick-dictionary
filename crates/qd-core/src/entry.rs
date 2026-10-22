//! Dictionary entries, as stored in a pack's `entries` table.
//! See `spec/pack-format.md`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: i64,
    pub word: String,
    /// Part of speech, as a wiktextract code: `noun`, `verb`, `adj`, `name`…
    pub pos: String,
    pub ipa: Option<String>,
    pub tags: Vec<String>,
    /// Zipf frequency of the word, times 100 (0 when unknown).
    pub freq: u16,
    pub senses: Vec<Sense>,
    pub forms: Vec<FormGroup>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Sense {
    /// From general to specific: a sub-sense lists its parent first.
    pub glosses: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_of: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alt_of: Vec<String>,
}

/// Consecutive forms sharing the same tags: a plural, or one tense of a
/// conjugation table ("je vais", "tu vas", …).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FormGroup {
    pub forms: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub raw_tags: Vec<String>,
}

/// Usage labels that mark a word or form as one people rarely mean.
const MARGINAL_TAGS: &[&str] = &[
    "archaic",
    "obsolete",
    "dated",
    "rare",
    "dialectal",
    "nonstandard",
    "misspelling",
    "pronunciation-spelling",
    "eye-dialect",
];

pub(crate) fn is_marginal(tags: &[String]) -> bool {
    tags.iter().any(|t| MARGINAL_TAGS.contains(&t.as_str()))
}

impl Entry {
    /// The form at `ord`, counted across groups, with its group's tags.
    pub fn form(&self, ord: usize) -> Option<(&str, &FormGroup)> {
        self.forms
            .iter()
            .flat_map(|g| g.forms.iter().map(move |f| (f.as_str(), g)))
            .nth(ord)
    }

    /// True when every sense carries a marginal label (archaic, rare…).
    pub fn is_marginal(&self) -> bool {
        is_marginal(&self.tags)
            || (!self.senses.is_empty() && self.senses.iter().all(|s| is_marginal(&s.tags)))
    }

    /// True for entries that only point to another spelling
    /// ("Ecole": typographic variant of "École").
    pub fn is_alternative(&self) -> bool {
        self.tags.iter().any(|t| t == "alt-of")
            || (!self.senses.is_empty()
                && self.senses.iter().all(|s| s.tags.iter().any(|t| t == "alt-of")))
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct EntryData {
    pub senses: Vec<Sense>,
    pub forms: Vec<FormGroup>,
}
