use crate::config::{MATCHING_WEIGHTS, SWEDEN_CITIES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structure to hold detailed match scoring breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchScore {
    pub overall: f64,
    pub skill_score: f64,
    pub experience_score: f64,
    pub location_score: f64,
    pub education_score: f64,
}

/// Normalize Swedish city names by replacing special characters.
fn normalize_swedish_city(city: &str) -> String {
    city.to_lowercase()
        .replace('ö', "o")
        .replace('ä', "a")
        .replace('å', "a")
        .replace('é', "e")
        .replace('ü', "u")
}

/// Compute location match score.
pub fn compute_location_score(candidate_location: &str, job_location: &str) -> f64 {
    let c_loc = candidate_location.trim().to_lowercase();
    let j_loc = job_location.trim().to_lowercase();

    // Empty locations
    if c_loc.is_empty() || j_loc.is_empty() {
        return 0.5;
    }

    // Exact match
    if c_loc == j_loc {
        return 1.0;
    }

    // Normalize Swedish city names
    let c_normalized = normalize_swedish_city(&c_loc);
    let j_normalized = normalize_swedish_city(&j_loc);

    if c_normalized == j_normalized {
        return 1.0;
    }

    // Check if in same region (both in Sweden)
    let c_in_sweden = SWEDEN_CITIES.iter().any(|city| c_loc.contains(city));
    let j_in_sweden = SWEDEN_CITIES.iter().any(|city| j_loc.contains(city));

    if c_in_sweden && j_in_sweden {
        return 0.7;
    }

    // Check for "remote"
    if c_loc.contains("remote") || j_loc.contains("remote") {
        return 0.8;
    }

    // Different locations
    0.3
}

/// Compute experience match score.
pub fn compute_experience_score(candidate_years: i32, job_level: &str) -> f64 {
    let level_lower = job_level.trim().to_lowercase();

    let level_years: HashMap<&str, (i32, i32)> = [
        ("entry", (0, 2)),
        ("junior", (1, 3)),
        ("professional", (2, 5)),
        ("experienced", (3, 6)),
        ("senior", (5, 10)),
        ("specialist", (5, 12)),
        ("expert", (7, 15)),
        ("senior expert", (10, 20)),
        ("lead", (8, 20)),
        ("chief", (10, 25)),
    ]
    .into_iter()
    .collect();

    // Find matching level
    let expected_range = level_years
        .iter()
        .find(|(&level, _)| level_lower.contains(level))
        .map(|(_, &range)| range);

    let Some((min_years, max_years)) = expected_range else {
        return 0.5;
    };

    if candidate_years >= min_years && candidate_years <= max_years {
        1.0
    } else if candidate_years > max_years {
        let overage = candidate_years - max_years;
        (1.0 - overage as f64 * 0.05).max(0.6)
    } else {
        let shortage = min_years - candidate_years;
        (1.0 - shortage as f64 * 0.2).max(0.0)
    }
}

/// Extract experience level from job title and compute score.
pub fn compute_experience_score_from_title(job_title: &str, candidate_years: i32) -> f64 {
    // Extract level from Volvo-style titles (SE_Role_Level)
    let parts: Vec<&str> = job_title.split('_').collect();

    if parts.len() >= 3 {
        let level = parts.last().unwrap_or(&"");
        return compute_experience_score(candidate_years, level);
    }

    // Try to find level keywords in title
    let title_lower = job_title.to_lowercase();
    let levels = [
        "senior expert",
        "expert",
        "senior",
        "specialist",
        "lead",
        "chief",
        "experienced",
        "professional",
        "junior",
        "entry",
    ];

    for level in levels {
        if title_lower.contains(level) {
            return compute_experience_score(candidate_years, level);
        }
    }

    0.5 // Unknown level
}

/// Compute education/qualification match score.
pub fn compute_education_score(candidate_education: &str, job_requirements: &str) -> f64 {
    let c_edu = candidate_education.to_lowercase();
    let j_req = job_requirements.to_lowercase();

    let levels: Vec<(&str, i32)> = vec![
        ("phd", 5),
        ("doctorate", 5),
        ("master", 4),
        ("msc", 4),
        ("bachelor", 3),
        ("bsc", 3),
        ("degree", 3),
        ("diploma", 2),
        ("certificate", 1),
    ];

    // Find highest education level for candidate
    let c_level = levels
        .iter()
        .filter(|(keyword, _)| c_edu.contains(keyword))
        .map(|(_, level)| *level)
        .max()
        .unwrap_or(0);

    // Find required level from job
    let j_level = levels
        .iter()
        .filter(|(keyword, _)| j_req.contains(keyword))
        .map(|(_, level)| *level)
        .max()
        .unwrap_or(0);

    // If no specific requirements, neutral score
    if j_level == 0 {
        return 0.7;
    }

    // Compare levels
    if c_level >= j_level {
        1.0
    } else if c_level == j_level - 1 {
        0.7
    } else {
        0.4
    }
}

/// Compute weighted overall match score. Returns 0-100 scale.
pub fn compute_overall_score(
    skill_score: f64,
    experience_score: f64,
    location_score: f64,
    education_score: f64,
) -> f64 {
    let overall = MATCHING_WEIGHTS.skill * skill_score
        + MATCHING_WEIGHTS.experience * experience_score
        + MATCHING_WEIGHTS.location * location_score
        + MATCHING_WEIGHTS.education * education_score;

    (overall * 100.0 * 100.0).round() / 100.0
}

/// Create a MatchScore with all components.
pub fn create_match_score(
    skill_score: f64,
    exp_score: f64,
    loc_score: f64,
    edu_score: f64,
) -> MatchScore {
    let overall = compute_overall_score(skill_score, exp_score, loc_score, edu_score);

    MatchScore {
        overall,
        skill_score: (skill_score * 10000.0).round() / 100.0,
        experience_score: (exp_score * 10000.0).round() / 100.0,
        location_score: (loc_score * 10000.0).round() / 100.0,
        education_score: (edu_score * 10000.0).round() / 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_same_city() {
        assert_eq!(
            compute_location_score("Gothenburg", "Gothenburg"),
            1.0
        );
    }

    #[test]
    fn test_location_swedish_cities() {
        let score = compute_location_score("Gothenburg", "Stockholm");
        assert!(score >= 0.6);
    }

    #[test]
    fn test_location_empty() {
        assert_eq!(compute_location_score("", "Gothenburg"), 0.5);
    }

    #[test]
    fn test_experience_perfect() {
        assert_eq!(compute_experience_score(7, "Senior"), 1.0);
    }

    #[test]
    fn test_experience_under() {
        let score = compute_experience_score(2, "Senior");
        assert!(score < 0.7);
    }

    #[test]
    fn test_experience_over() {
        let score = compute_experience_score(15, "Senior");
        assert!(score > 0.5 && score < 1.0);
    }

    #[test]
    fn test_experience_from_title() {
        let score = compute_experience_score_from_title("SE_Mechanical Engineer_Senior", 7);
        assert!(score >= 0.8);
    }

    #[test]
    fn test_overall_perfect() {
        let score = compute_overall_score(1.0, 1.0, 1.0, 1.0);
        assert_eq!(score, 100.0);
    }

    #[test]
    fn test_overall_mixed() {
        let score = compute_overall_score(0.8, 0.7, 0.6, 0.5);
        assert!(score > 60.0 && score < 80.0);
    }
}
