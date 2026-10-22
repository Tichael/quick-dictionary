//! Search-key normalization. Must match `pipeline/qd_pipeline/normalize.py`;
//! both are tested against `spec/normalization-vectors.json`.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::canonical_combining_class;

/// Turns text into a search key: no accents, lowercase, unified
/// apostrophes and dashes, single spaces.
pub fn normalize(text: &str) -> String {
    let stripped: String = text
        .nfkd()
        .filter(|&c| canonical_combining_class(c) == 0)
        .collect();
    let mut mapped = String::with_capacity(stripped.len());
    for c in stripped.to_lowercase().chars() {
        match c {
            'œ' => mapped.push_str("oe"),
            'æ' => mapped.push_str("ae"),
            'ß' => mapped.push_str("ss"),
            'ø' => mapped.push('o'),
            'đ' => mapped.push('d'),
            'ł' => mapped.push('l'),
            '’' | '‘' | 'ʼ' | '′' => mapped.push('\''),
            '‐' | '‑' | '–' | '—' => mapped.push('-'),
            _ => mapped.push(c),
        }
    }
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_vectors() {
        let vectors: Vec<(String, String)> =
            serde_json::from_str(include_str!("../../../spec/normalization-vectors.json")).unwrap();
        for (raw, expected) in vectors {
            assert_eq!(normalize(&raw), expected, "normalizing {raw:?}");
        }
    }
}
