/// Compute Levenshtein distance between two strings
pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    // Use two rows instead of full matrix for space efficiency
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1) // deletion
                .min(curr[j - 1] + 1) // insertion
                .min(prev[j - 1] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// Find the most similar name from candidates within max_distance
/// Returns suggestion text like "did you mean `foo`?"
pub fn find_similar<'a>(
    name: &str,
    candidates: impl Iterator<Item = &'a str>,
    max_distance: usize,
) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;

    for candidate in candidates {
        // Skip if too different in length (optimization)
        let len_diff = (name.len() as isize - candidate.len() as isize).unsigned_abs();
        if len_diff > max_distance {
            continue;
        }

        let dist = levenshtein(name, candidate);
        if dist <= max_distance {
            match best {
                None => best = Some((candidate, dist)),
                Some((_, best_dist)) if dist < best_dist => best = Some((candidate, dist)),
                _ => {}
            }
        }
    }

    best.map(|(similar, _)| format!("did you mean `{similar}`?"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("a", ""), 1);
        assert_eq!(levenshtein("", "a"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("count", "cont"), 1); // delete 'u'
        assert_eq!(levenshtein("foo", "bar"), 3);
    }

    #[test]
    fn test_find_similar() {
        let candidates = ["count", "counter", "amount", "total"];

        assert_eq!(
            find_similar("cont", candidates.iter().copied(), 2),
            Some("did you mean `count`?".to_string())
        );

        assert_eq!(
            find_similar("count", candidates.iter().copied(), 2),
            Some("did you mean `count`?".to_string())
        );

        // Too far from any candidate
        assert_eq!(find_similar("xyz", candidates.iter().copied(), 2), None);
    }
}
