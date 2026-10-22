//! Searches the small packs in tests/packs, built by the real pipeline from
//! pipeline/tests/fixtures (see pipeline/make_test_packs.py).

use std::path::Path;

use qd_core::{Dictionary, MatchKind, SearchResult};

fn dictionary() -> Dictionary {
    Dictionary::open_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/packs"), None).unwrap()
}

fn words(results: &[SearchResult]) -> Vec<(&str, &str, &str)> {
    results
        .iter()
        .map(|r| (r.lang.as_str(), r.entry.word.as_str(), r.entry.pos.as_str()))
        .collect()
}

#[test]
fn opens_every_pack() {
    let dictionary = dictionary();
    let langs: Vec<_> = dictionary.packs.iter().map(|p| p.lang.as_str()).collect();
    assert_eq!(langs, ["en", "fr"]);
}

#[test]
fn ignores_accents_case_and_ligatures() {
    let dictionary = dictionary();
    for query in ["ecole", "École", "ÉCOLE"] {
        let results = dictionary.search(query, 5).unwrap();
        assert_eq!(words(&results)[0], ("fr", "école", "noun"), "{query}");
    }
    let results = dictionary.search("coeur", 5).unwrap();
    assert_eq!(words(&results), [("fr", "cœur", "noun")]);
}

#[test]
fn finds_the_lemma_of_an_inflected_form() {
    let dictionary = dictionary();
    let results = dictionary.search("lançons", 5).unwrap();
    assert_eq!(words(&results), [("fr", "lancer", "verb")]);
    let form = results[0].matched_form.as_ref().unwrap();
    assert_eq!(form.form, "nous lançons");
    assert_eq!(form.tags, ["indicative", "present"]);

    let results = dictionary.search("children", 5).unwrap();
    assert_eq!(words(&results), [("en", "child", "noun")]);
}

#[test]
fn strips_pronouns_from_the_query() {
    let results = dictionary().search("que nous lancions", 5).unwrap();
    assert_eq!(words(&results), [("fr", "lancer", "verb")]);
}

#[test]
fn ranks_common_meanings_before_rare_ones() {
    // "went" is also an obsolete noun and a river name: the verb comes first.
    let results = dictionary().search("went", 5).unwrap();
    assert_eq!(
        words(&results),
        [("en", "go", "verb"), ("en", "went", "noun"), ("en", "Went", "name")]
    );
    assert_eq!(results[0].kind, MatchKind::Form);
}

#[test]
fn prefers_the_spelling_typed() {
    // "colour" is a form of "color" too, and "color" is more frequent.
    let results = dictionary().search("colour", 5).unwrap();
    assert_eq!(words(&results), [("en", "colour", "noun"), ("en", "color", "noun")]);
}

#[test]
fn follows_misspellings_to_the_right_word() {
    let results = dictionary().search("recieve", 5).unwrap();
    assert_eq!(words(&results), [("en", "receive", "verb")]);
    assert_eq!(results[0].kind, MatchKind::FormOf);
    assert_eq!(results[0].note.as_deref(), Some("Misspelling of receive."));
}

#[test]
fn corrects_typos() {
    let dictionary = dictionary();
    // Two edits away: "eu" swapped for "ue".
    let results = dictionary.search("acceuil", 5).unwrap();
    assert_eq!(words(&results)[0], ("fr", "accueil", "noun"));
    assert_eq!(results[0].distance, 2);

    // A frequent word one edit away beats a rare one at the same distance.
    let results = dictionary.search("acommodate", 5).unwrap();
    assert_eq!(words(&results)[0], ("en", "accommodate", "verb"));
}

#[test]
fn does_not_guess_on_short_queries() {
    assert!(dictionary().search("xq", 5).unwrap().is_empty());
}

#[test]
fn searches_every_language() {
    // "lance" is only French here; the English pack has no match.
    let results = dictionary().search("lance", 5).unwrap();
    assert_eq!(words(&results), [("fr", "lancer", "verb"), ("fr", "lance", "noun")]);
}

#[test]
fn completes_prefixes_by_frequency() {
    let suggestions = dictionary().complete("lan", 3);
    let keys: Vec<_> = suggestions.iter().map(|s| s.key.as_str()).collect();
    // Ties (both unknown, frequency 0) are broken alphabetically.
    assert_eq!(keys, ["lancer", "lance", "lancames"]);
}

#[test]
fn caches_the_search_index() {
    let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/packs");
    let cache = std::env::temp_dir().join(format!("qd-core-test-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);

    let first = Dictionary::open_dir(&packs, Some(&cache)).unwrap();
    let cached: Vec<_> = std::fs::read_dir(&cache).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(cached.len(), 2, "{cached:?}");

    let second = Dictionary::open_dir(&packs, Some(&cache)).unwrap();
    assert_eq!(
        words(&first.search("acceuil", 5).unwrap()),
        words(&second.search("acceuil", 5).unwrap())
    );
    std::fs::remove_dir_all(&cache).unwrap();
}
