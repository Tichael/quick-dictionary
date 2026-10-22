//! Searching every installed pack at once, and ranking the results.

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::Error;
use crate::entry::{Entry, is_marginal};
use crate::language::form_key;
use crate::normalize::{fold_case, normalize};
use crate::pack::{KeyMatch, MatchKind, Pack};

/// How many close spellings to look up when the query has no exact match.
const MAX_FUZZY_KEYS: usize = 6;
/// Score cost of each half edit between the query and a close spelling,
/// in frequency units (Zipf times 100). See [`crate::spelling_cost`].
const HALF_EDIT_COST: i32 = 100;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub lang: String,
    pub lang_name: String,
    pub entry: Entry,
    /// The search key that reached the entry: the query itself, or a close
    /// spelling when `distance` > 0.
    pub key: String,
    pub distance: u32,
    pub kind: MatchKind,
    /// The inflected form that matched ("nous allions"), with its tags.
    pub matched_form: Option<MatchedForm>,
    /// Why a form-of match applies ("Pluriel de œil.").
    pub note: Option<String>,
    pub score: i32,
    /// 0 for exact matches, then the rank of the close spelling.
    #[serde(skip)]
    key_rank: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchedForm {
    pub form: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub lang: String,
    pub key: String,
}

pub struct Dictionary {
    pub packs: Vec<Pack>,
}

impl Dictionary {
    /// Opens every `*.db` pack in `dir`, caching their search indexes in
    /// `cache_dir` if given (see [`Pack::open`]).
    pub fn open_dir(dir: &Path, cache_dir: Option<&Path>) -> Result<Dictionary, Error> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "db"))
            .collect();
        paths.sort();
        let packs = paths
            .iter()
            .map(|p| Pack::open(p, cache_dir))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Dictionary { packs })
    }

    /// Looks `query` up in every pack. Exact matches come first; when there
    /// are none, or only rare ones, close spellings are added after them.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, Error> {
        let normalized = normalize(query);
        if normalized.is_empty() {
            return Ok(Vec::new());
        }
        let query = Query::new(query, normalized.clone());

        let mut results = Vec::new();
        let mut best_exact_freq = None;
        for pack in &self.packs {
            for key in query_keys(&pack.lang, &normalized) {
                if let Some(freq) = pack.key_freq(&key) {
                    best_exact_freq = best_exact_freq.max(Some(freq));
                    self.collect(pack, &key, 0, 0, &query, &mut results)?;
                }
            }
        }

        // A rare exact match may be a typo of a common word ("chevals" is a
        // real misspelling entry, "chevaux" is what was meant).
        if best_exact_freq.is_none_or(|f| f < RARE_FREQ) {
            let floor = best_exact_freq.unwrap_or(0) + FUZZY_MARGIN;
            for (rank, (pack, close)) in self.close_keys(&normalized)?.into_iter().enumerate() {
                if best_exact_freq.is_none() || close.freq >= floor {
                    let (key, distance) = (&close.key, close.distance);
                    self.collect(pack, key, distance, rank + 1, &query, &mut results)?;
                }
            }
        }

        results.sort_by(|a, b| {
            a.key_rank
                .cmp(&b.key_rank)
                .then_with(|| b.score.cmp(&a.score))
                .then_with(|| a.entry.id.cmp(&b.entry.id))
        });
        // An entry reached through several keys is shown once, at its best.
        let mut seen = HashSet::new();
        results.retain(|r| seen.insert((r.lang.clone(), r.entry.id)));
        results.truncate(limit);
        Ok(results)
    }

    /// Completions for a partly typed word, most frequent first.
    pub fn complete(&self, prefix: &str, limit: usize) -> Vec<Suggestion> {
        let normalized = normalize(prefix);
        if normalized.chars().count() < 2 {
            return Vec::new();
        }
        let mut all: Vec<(u16, Suggestion)> = self
            .packs
            .iter()
            .flat_map(|pack| {
                pack.complete(&normalized, limit).into_iter().map(|m| {
                    (m.freq, Suggestion { lang: pack.lang.clone(), key: m.key })
                })
            })
            .collect();
        all.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.key.cmp(&b.1.key)));
        all.into_iter().take(limit).map(|(_, s)| s).collect()
    }

    /// The best close spellings of `normalized` across all packs, weighing
    /// frequency against the number of edits.
    fn close_keys(&self, normalized: &str) -> Result<Vec<(&Pack, KeyMatch)>, Error> {
        let max_distance = match normalized.chars().count() {
            0..=2 => return Ok(Vec::new()),
            3..=5 => 1,
            _ => 2,
        };
        let mut all = Vec::new();
        for pack in &self.packs {
            for key in query_keys(&pack.lang, normalized) {
                for m in pack.fuzzy(&key, max_distance)? {
                    all.push((pack, m));
                }
            }
        }
        let key_score = |m: &KeyMatch| i32::from(m.freq) - HALF_EDIT_COST * m.cost as i32;
        all.sort_by(|a, b| {
            key_score(&b.1)
                .cmp(&key_score(&a.1))
                .then_with(|| a.1.key.cmp(&b.1.key))
        });
        // The same spelling can come from several packs or query keys.
        let mut seen = HashSet::new();
        all.retain(|(pack, m)| seen.insert((pack.lang.as_str(), m.key.clone())));
        all.truncate(MAX_FUZZY_KEYS);
        Ok(all)
    }

    fn collect(
        &self,
        pack: &Pack,
        key: &str,
        distance: u32,
        key_rank: usize,
        query: &Query,
        results: &mut Vec<SearchResult>,
    ) -> Result<(), Error> {
        for hit in pack.hits(key)? {
            let entry = pack.entry(hit.entry_id)?;
            let matched_form = hit.form_ord.and_then(|ord| entry.form(ord)).map(|(form, group)| {
                MatchedForm {
                    form: form.to_owned(),
                    tags: group.tags.iter().chain(&group.raw_tags).cloned().collect(),
                }
            });
            // Accents are ignored to find matches, but when the user typed
            // some, a match spelled with other accents is less likely meant
            // ("acheté" vs the noun "achète").
            let spelling = match (&matched_form, hit.kind) {
                (Some(form), _) => Some(form.form.as_str()),
                (None, MatchKind::Headword) => Some(entry.word.as_str()),
                _ => None,
            };
            let accent_mismatch = distance == 0
                && query.accented.as_deref().is_some_and(|typed| {
                    spelling.is_some_and(|s| !same_spelling(&pack.lang, typed, &fold_case(s)))
                });
            let score = score(&entry, hit.kind, matched_form.as_ref(), query.lowercase, accent_mismatch);
            results.push(SearchResult {
                lang: pack.lang.clone(),
                lang_name: pack.lang_name.clone(),
                entry,
                key: key.to_owned(),
                distance,
                kind: hit.kind,
                matched_form,
                note: hit.note,
                score,
                key_rank,
            });
        }
        Ok(())
    }
}

/// Below this key frequency (Zipf 2.5, times 100), an exact match is rare
/// enough that close spellings are worth showing too.
const RARE_FREQ: u16 = 250;
/// How much more frequent a close spelling must be than a rare exact match.
const FUZZY_MARGIN: u16 = 100;

struct Query {
    /// Whether the user typed no capitals: proper names are then unlikely.
    lowercase: bool,
    /// The query with case folded but accents kept, if it has any.
    accented: Option<String>,
}

impl Query {
    fn new(raw: &str, normalized: String) -> Query {
        let folded = fold_case(raw);
        Query {
            lowercase: !raw.chars().any(char::is_uppercase),
            accented: (folded != normalized).then_some(folded),
        }
    }
}

/// Whether two case-folded spellings are the same word, ignoring pronouns
/// ("nous allions" and "allions").
fn same_spelling(lang: &str, typed: &str, candidate: &str) -> bool {
    let typed_keys = query_keys(lang, typed);
    query_keys(lang, candidate).iter().any(|k| typed_keys.contains(k))
}

/// The keys to look up for a query: the query itself, plus the query with
/// pronouns stripped ("que nous allions" -> "allions").
fn query_keys(lang: &str, normalized: &str) -> Vec<String> {
    let stripped = form_key(lang, normalized);
    if stripped != normalized && !stripped.is_empty() {
        vec![normalized.to_owned(), stripped.to_owned()]
    } else {
        vec![normalized.to_owned()]
    }
}

/// Ranks entries matching the same key: frequent words first, then demotes
/// what people rarely mean (proper names, archaic words, variants).
fn score(
    entry: &Entry,
    kind: MatchKind,
    form: Option<&MatchedForm>,
    lowercase_query: bool,
    accent_mismatch: bool,
) -> i32 {
    let mut score = i32::from(entry.freq);
    if accent_mismatch {
        score -= 150;
    }
    if lowercase_query && entry.word.chars().any(char::is_uppercase) {
        score -= 150;
    }
    if entry.pos == "name" {
        score -= 100;
    }
    if entry.is_marginal() {
        score -= 150;
    }
    if entry.is_alternative() {
        score -= 100;
    }
    if let Some(form) = form {
        if is_marginal(&form.tags) {
            score -= 150;
        }
        // "colour" is an alternative form of "color": the user typed a valid
        // spelling, so its own entry should come first.
        if form.tags.iter().any(|t| t == "alternative") {
            score -= 100;
        }
    }
    if kind == MatchKind::Redirect {
        score -= 50;
    }
    score
}
