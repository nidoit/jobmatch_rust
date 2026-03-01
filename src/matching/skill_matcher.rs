use crate::nlp::skill_extractor::{get_skill_synonyms, normalize_skill};
use regex::Regex;
use strsim::{jaro_winkler, normalized_levenshtein};
use std::collections::HashSet;

/// Compute Jaccard similarity on character trigrams.
fn jaccard_trigram_similarity(s1: &str, s2: &str) -> f64 {
    if s1.is_empty() && s2.is_empty() {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let trigrams = |s: &str| -> HashSet<String> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 3 {
            let mut set = HashSet::new();
            set.insert(s.to_string());
            return set;
        }
        (0..=chars.len() - 3)
            .map(|i| chars[i..i + 3].iter().collect::<String>())
            .collect()
    };

    let t1 = trigrams(s1);
    let t2 = trigrams(s2);

    let intersection = t1.intersection(&t2).count() as f64;
    let union = t1.union(&t2).count() as f64;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Compute similarity between two skills using multiple metrics.
/// Returns a score between 0.0 and 1.0.
pub fn compute_skill_similarity(skill1: &str, skill2: &str) -> f64 {
    let s1 = normalize_skill(skill1);
    let s2 = normalize_skill(skill2);

    // Exact match after normalization
    if s1 == s2 {
        return 1.0;
    }

    // Check if one is a synonym of the other
    let synonyms1 = get_skill_synonyms(&s1);
    let synonyms2 = get_skill_synonyms(&s2);
    if synonyms1.contains(&s2) || synonyms2.contains(&s1) {
        return 0.95;
    }

    // Normalized Levenshtein similarity
    let lev_sim = normalized_levenshtein(&s1, &s2);

    // Jaro-Winkler similarity
    let jw_sim = jaro_winkler(&s1, &s2);

    // Jaccard trigram similarity
    let jaccard_sim = jaccard_trigram_similarity(&s1, &s2);

    // Weighted average
    0.4 * lev_sim + 0.35 * jw_sim + 0.25 * jaccard_sim
}

/// Find matching skills between candidate and job requirements.
/// Returns tuples of (candidate_skill, job_skill, similarity_score).
pub fn find_matching_skills(
    candidate_skills: &[String],
    job_skills: &[String],
    threshold: f64,
) -> Vec<(String, String, f64)> {
    let mut matches = Vec::new();

    for c_skill in candidate_skills {
        let mut best_match: Option<&String> = None;
        let mut best_score: f64 = 0.0;

        for j_skill in job_skills {
            let score = compute_skill_similarity(c_skill, j_skill);
            if score > best_score {
                best_score = score;
                best_match = Some(j_skill);
            }
        }

        if best_score >= threshold {
            if let Some(matched) = best_match {
                matches.push((c_skill.clone(), matched.clone(), best_score));
            }
        }
    }

    matches.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    matches
}

/// Find job skills that the candidate is missing.
pub fn find_missing_skills(
    candidate_skills: &[String],
    job_skills: &[String],
    threshold: f64,
) -> Vec<String> {
    let mut missing = Vec::new();

    for j_skill in job_skills {
        let is_matched = candidate_skills
            .iter()
            .any(|c_skill| compute_skill_similarity(c_skill, j_skill) >= threshold);
        if !is_matched {
            missing.push(j_skill.clone());
        }
    }

    missing
}

/// Compute overall skill match score between candidate and job.
/// Returns a score between 0.0 and 1.0.
pub fn compute_skill_match_score(
    candidate_skills: &[String],
    job_skills: &[String],
    threshold: f64,
) -> f64 {
    if job_skills.is_empty() {
        return 1.0;
    }
    if candidate_skills.is_empty() {
        return 0.0;
    }

    let matches = find_matching_skills(candidate_skills, job_skills, threshold);

    // Count unique matched job skills
    let matched_job_skills: HashSet<&String> = matches.iter().map(|m| &m.1).collect();

    // Calculate coverage
    let coverage = matched_job_skills.len() as f64 / job_skills.len() as f64;

    // Calculate average match quality
    let avg_quality = if matches.is_empty() {
        0.0
    } else {
        matches.iter().map(|m| m.2).sum::<f64>() / matches.len() as f64
    };

    // Combined score: 70% coverage, 30% quality
    0.7 * coverage + 0.3 * avg_quality
}

/// Rank skills by how frequently they appear in job descriptions.
pub fn rank_skills_by_importance(
    skills: &[String],
    job_descriptions: &[String],
) -> Vec<(String, f64)> {
    let mut skill_counts: Vec<(String, f64)> = Vec::new();
    let n_docs = job_descriptions.len().max(1) as f64;

    for skill in skills {
        let pattern = format!(r"(?i)\b{}\b", regex::escape(skill));
        let count = if let Ok(re) = Regex::new(&pattern) {
            job_descriptions
                .iter()
                .filter(|desc| re.is_match(desc))
                .count()
        } else {
            0
        };
        skill_counts.push((skill.clone(), count as f64 / n_docs));
    }

    skill_counts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    skill_counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert_eq!(compute_skill_similarity("python", "python"), 1.0);
    }

    #[test]
    fn test_synonym_match() {
        assert!(compute_skill_similarity("js", "javascript") >= 0.9);
    }

    #[test]
    fn test_similar_skills() {
        let sim = compute_skill_similarity("javascript", "typescript");
        assert!(sim > 0.5 && sim < 1.0);
    }

    #[test]
    fn test_different_skills() {
        let sim = compute_skill_similarity("python", "kubernetes");
        assert!(sim < 0.5);
    }

    #[test]
    fn test_find_matching() {
        let candidate: Vec<String> = vec!["python", "java", "sql"]
            .into_iter()
            .map(String::from)
            .collect();
        let job: Vec<String> = vec!["python", "javascript", "sql"]
            .into_iter()
            .map(String::from)
            .collect();

        let matches = find_matching_skills(&candidate, &job, 0.7);
        let matched_skills: Vec<&String> = matches.iter().map(|m| &m.0).collect();

        assert!(matched_skills.contains(&&"python".to_string()));
        assert!(matched_skills.contains(&&"sql".to_string()));
    }

    #[test]
    fn test_find_missing() {
        let candidate: Vec<String> = vec!["python", "sql"].into_iter().map(String::from).collect();
        let job: Vec<String> = vec!["python", "javascript", "sql", "aws"]
            .into_iter()
            .map(String::from)
            .collect();

        let missing = find_missing_skills(&candidate, &job, 0.7);
        assert!(missing.contains(&"javascript".to_string()));
        assert!(missing.contains(&"aws".to_string()));
        assert!(!missing.contains(&"python".to_string()));
    }

    #[test]
    fn test_skill_match_score() {
        // Perfect match
        let score = compute_skill_match_score(
            &["python".into(), "sql".into()],
            &["python".into(), "sql".into()],
            0.7,
        );
        assert!(score >= 0.9);

        // Partial match
        let score = compute_skill_match_score(
            &["python".into()],
            &["python".into(), "java".into(), "sql".into()],
            0.7,
        );
        assert!(score > 0.2 && score < 0.8);

        // No match
        let score = compute_skill_match_score(
            &["rust".into(), "go".into()],
            &["python".into(), "java".into()],
            0.7,
        );
        assert!(score < 0.3);
    }
}
