//! Per-language rules for search keys. Must match
//! `pipeline/qd_pipeline/languages.py`; both are tested against
//! `spec/form-key-vectors.json`.

const FRENCH_PREFIXES: &[&str] = &[
    "que ", "qu'", "je ", "j'", "tu ", "il/elle/on ", "il ", "elle ", "on ", "nous ", "vous ",
    "ils/elles ", "ils ", "elles ", "me ", "m'", "te ", "t'", "se ", "s'",
];

const FRENCH_SUFFIXES: &[&str] = &[
    "-moi", "-toi", "-nous", "-vous", "-le", "-la", "-les", "-lui", "-leur", "-en", "-y",
];

/// Strips what users don't type from a normalized inflected form:
/// "que nous lancions" -> "lancions", "abats-toi" -> "abats".
pub fn form_key<'a>(lang: &str, form: &'a str) -> &'a str {
    if lang != "fr" {
        return form;
    }
    let mut key = form;
    while let Some(rest) = FRENCH_PREFIXES.iter().find_map(|p| key.strip_prefix(p)) {
        key = rest;
    }
    FRENCH_SUFFIXES
        .iter()
        .find_map(|s| key.strip_suffix(s))
        .unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn shared_vectors() {
        let vectors: HashMap<String, Vec<(String, String)>> =
            serde_json::from_str(include_str!("../../../spec/form-key-vectors.json")).unwrap();
        for (lang, cases) in vectors {
            for (form, expected) in cases {
                assert_eq!(form_key(&lang, &form), expected, "{lang}: {form:?}");
            }
        }
    }
}
