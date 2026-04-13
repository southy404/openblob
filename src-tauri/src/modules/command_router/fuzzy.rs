use strsim::jaro_winkler;

pub fn fuzzy_has_any_strings(tokens: &[&str], words: &[String], threshold: f32) -> bool {
    tokens
        .iter()
        .any(|t| words.iter().any(|w| jaro_winkler(t, w) >= threshold as f64))
}

pub fn fuzzy_count_strings(tokens: &[&str], words: &[String], threshold: f32) -> usize {
    tokens
        .iter()
        .filter(|t| words.iter().any(|w| jaro_winkler(t, w) >= threshold as f64))
        .count()
}

pub fn score_strings(tokens: &[&str], words: &[String], threshold: f32, weight: f32) -> f32 {
    fuzzy_count_strings(tokens, words, threshold) as f32 * weight
}

pub fn best_similarity(token: &str, words: &[&str]) -> f32 {
    words
        .iter()
        .map(|w| jaro_winkler(token, w) as f32)
        .fold(0.0_f32, f32::max)
}

pub fn fuzzy_has_any(tokens: &[&str], words: &[&str], threshold: f32) -> bool {
    tokens.iter().any(|t| best_similarity(t, words) >= threshold)
}

pub fn fuzzy_count(tokens: &[&str], words: &[&str], threshold: f32) -> usize {
    tokens.iter().filter(|t| best_similarity(t, words) >= threshold).count()
}

pub fn contains_any_phrase(normalized: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| normalized.contains(p))
}

pub fn score(tokens: &[&str], words: &[&str], threshold: f32, weight: f32) -> f32 {
    fuzzy_count(tokens, words, threshold) as f32 * weight
}