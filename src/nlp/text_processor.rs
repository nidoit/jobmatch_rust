use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// English stopwords
static STOPWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "from", "as", "is", "was", "are", "were", "been",
        "be", "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "must", "shall", "can", "need", "dare", "ought",
        "used", "it", "its", "this", "that", "these", "those", "i", "you", "he",
        "she", "we", "they", "me", "him", "her", "us", "them", "my", "your",
        "his", "our", "their", "mine", "yours", "hers", "ours", "theirs",
        "what", "which", "who", "whom", "whose", "where", "when", "why", "how",
        "all", "each", "every", "both", "few", "more", "most", "other", "some",
        "such", "no", "nor", "not", "only", "own", "same", "so", "than", "too",
        "very", "just", "also", "now", "here", "there", "then", "once", "always",
        "never", "sometimes", "often", "usually", "etc", "ie", "eg", "via",
    ]
    .into_iter()
    .collect()
});

static RE_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+").unwrap());
static RE_EMAIL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap());
static RE_SPECIAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"[^\w\s\-+#.]").unwrap());
static RE_WHITESPACE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());

/// Clean and normalize text for NLP processing.
pub fn preprocess_text(text: &str) -> String {
    let text = text.to_lowercase();
    let text = RE_URL.replace_all(&text, " ");
    let text = RE_EMAIL.replace_all(&text, " ");
    let text = RE_SPECIAL.replace_all(&text, " ");
    let text = RE_WHITESPACE.replace_all(&text, " ");
    text.trim().to_string()
}

/// Split text into tokens (words).
pub fn tokenize(text: &str) -> Vec<String> {
    let clean = preprocess_text(text);
    clean
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Remove common stopwords from token list.
pub fn remove_stopwords(tokens: &[String]) -> Vec<String> {
    tokens
        .iter()
        .filter(|t| !STOPWORDS.contains(t.to_lowercase().as_str()) && t.len() > 1)
        .cloned()
        .collect()
}

/// Full preprocessing pipeline: preprocess, tokenize, remove stopwords.
pub fn normalize_text(text: &str) -> Vec<String> {
    let tokens = tokenize(text);
    remove_stopwords(&tokens)
}

/// Extract n-grams from token list.
pub fn extract_ngrams(tokens: &[String], n: usize) -> Vec<String> {
    if tokens.len() < n {
        return Vec::new();
    }
    (0..=tokens.len() - n)
        .map(|i| tokens[i..i + n].join(" "))
        .collect()
}

/// Extract most important terms from text using TF-like scoring.
pub fn get_important_terms(text: &str, top_n: usize) -> Vec<(String, f64)> {
    let tokens = normalize_text(text);

    let mut term_freq: HashMap<String, usize> = HashMap::new();
    for token in &tokens {
        *term_freq.entry(token.clone()).or_insert(0) += 1;
    }

    let mut sorted_terms: Vec<(String, usize)> = term_freq.into_iter().collect();
    sorted_terms.sort_by(|a, b| b.1.cmp(&a.1));

    let max_freq = sorted_terms.first().map(|t| t.1).unwrap_or(1) as f64;

    sorted_terms
        .into_iter()
        .take(top_n)
        .map(|(term, freq)| (term, freq as f64 / max_freq))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_text() {
        let result = preprocess_text("Hello WORLD! This is a TEST.");
        assert_eq!(result, "hello world this is a test.");
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("Python Java SQL");
        assert_eq!(tokens.len(), 3);
        assert!(tokens.contains(&"python".to_string()));
    }

    #[test]
    fn test_remove_stopwords() {
        let tokens: Vec<String> = vec!["the", "python", "is", "great"]
            .into_iter()
            .map(String::from)
            .collect();
        let filtered = remove_stopwords(&tokens);
        assert!(!filtered.contains(&"the".to_string()));
        assert!(filtered.contains(&"python".to_string()));
    }

    #[test]
    fn test_normalize_text() {
        let result = normalize_text("The quick Python programmer");
        assert!(result.contains(&"python".to_string()));
        assert!(!result.contains(&"the".to_string()));
    }
}
