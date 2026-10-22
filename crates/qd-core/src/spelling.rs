//! How far apart two spellings are.

/// Levenshtein distance between two strings, counted in characters.
pub fn edit_distance(a: &str, b: &str) -> u32 {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<u32> = (0..=b.len() as u32).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i as u32 + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { prev } else { prev + 1 };
            prev = row[j + 1];
            row[j + 1] = cost.min(row[j] + 1).min(prev + 1);
        }
    }
    row[b.len()]
}

/// The cost of turning a typed word into a candidate spelling, in half
/// edits, weighted by how people actually misspell:
///
/// - a substitution, or a letter added or removed: 2;
/// - a doubled letter added or removed ("apeller" -> "appeler"): 1;
/// - two adjacent letters swapped ("acceuil" -> "accueil"): 2.
pub fn spelling_cost(typed: &str, candidate: &str) -> u32 {
    let a: Vec<char> = typed.chars().collect();
    let b: Vec<char> = candidate.chars().collect();
    let doubled = |s: &[char], k: usize| {
        (k > 0 && s[k - 1] == s[k]) || (k + 1 < s.len() && s[k + 1] == s[k])
    };
    let indel = |s: &[char], k: usize| if doubled(s, k) { 1 } else { 2 };

    // d[i][j]: cost of turning a[..i] into b[..j] (optimal string alignment).
    let width = b.len() + 1;
    let mut d = vec![0u32; (a.len() + 1) * width];
    for j in 1..=b.len() {
        d[j] = d[j - 1] + indel(&b, j - 1);
    }
    for i in 1..=a.len() {
        d[i * width] = d[(i - 1) * width] + indel(&a, i - 1);
        for j in 1..=b.len() {
            let substitution = if a[i - 1] == b[j - 1] { 0 } else { 2 };
            let mut best = (d[(i - 1) * width + j - 1] + substitution)
                .min(d[(i - 1) * width + j] + indel(&a, i - 1))
                .min(d[i * width + j - 1] + indel(&b, j - 1));
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[(i - 2) * width + j - 2] + 2);
            }
            d[i * width + j] = best;
        }
    }
    d[a.len() * width + b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        assert_eq!(edit_distance("acceuil", "accueil"), 2);
        assert_eq!(edit_distance("ecole", "ecoles"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("chat", "chat"), 0);
    }

    #[test]
    fn doubled_letters_are_cheap() {
        assert_eq!(spelling_cost("apeller", "appeler"), 2);
        assert_eq!(spelling_cost("apeller", "aller"), 4);
        assert_eq!(spelling_cost("occurence", "occurrence"), 1);
        assert_eq!(spelling_cost("acommodate", "accommodate"), 1);
    }

    #[test]
    fn swaps_count_as_one_edit() {
        assert_eq!(spelling_cost("acceuil", "accueil"), 2);
        assert_eq!(spelling_cost("teh", "the"), 2);
    }

    #[test]
    fn plain_edits() {
        assert_eq!(spelling_cost("chat", "chat"), 0);
        assert_eq!(spelling_cost("chat", "chats"), 2);
        assert_eq!(spelling_cost("chat", "chut"), 2);
        assert_eq!(spelling_cost("", "ab"), 4);
    }
}
