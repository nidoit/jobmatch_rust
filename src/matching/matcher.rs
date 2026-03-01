use crate::matching::scorer::{
    compute_education_score, compute_experience_score_from_title, compute_location_score,
    compute_overall_score,
};
use crate::matching::skill_matcher::{
    compute_skill_match_score, find_matching_skills, find_missing_skills,
};
use crate::models::candidate::Candidate;
use crate::models::job::Job;
use crate::nlp::skill_extractor::extract_skills_from_text;
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Structure to hold match result between a candidate and a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub match_id: String,
    pub candidate_id: String,
    pub job_id: String,
    pub candidate_name: String,
    pub job_title: String,
    pub overall_score: f64,
    pub skill_score: f64,
    pub experience_score: f64,
    pub location_score: f64,
    pub education_score: f64,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub matched_at: String,
}

/// Compute match between a single candidate and job.
pub fn match_candidate_to_job(candidate: &Candidate, job: &Job) -> MatchResult {
    // Extract skills
    let mut candidate_skills = candidate.get_skills_list();
    if candidate_skills.is_empty() && !candidate.resume_text.is_empty() {
        candidate_skills = extract_skills_from_text(&candidate.resume_text);
    }

    let mut job_skills = job.get_skills_list();
    if job_skills.is_empty() && !job.description.is_empty() {
        let combined = format!("{} {}", job.description, job.requirements);
        job_skills = extract_skills_from_text(&combined);
    }

    // Compute skill match
    let skill_score = compute_skill_match_score(&candidate_skills, &job_skills, 0.7);

    // Find matching and missing skills
    let matches = find_matching_skills(&candidate_skills, &job_skills, 0.7);
    let matched_skill_names: Vec<String> = matches.iter().map(|m| m.0.clone()).collect();
    let missing = find_missing_skills(&candidate_skills, &job_skills, 0.7);

    // Compute location match
    let location_score = compute_location_score(&candidate.location, &job.location);

    // Compute experience match
    let exp_score =
        compute_experience_score_from_title(&job.title, candidate.experience_years);

    // Compute education match
    let edu_score = compute_education_score(&candidate.education, &job.requirements);

    // Compute overall score
    let overall = compute_overall_score(skill_score, exp_score, location_score, edu_score);

    MatchResult {
        match_id: Uuid::new_v4().to_string(),
        candidate_id: candidate.candidate_id.clone(),
        job_id: job.job_id.clone(),
        candidate_name: candidate.name.clone(),
        job_title: job.title.clone(),
        overall_score: overall,
        skill_score: (skill_score * 10000.0).round() / 100.0,
        experience_score: (exp_score * 10000.0).round() / 100.0,
        location_score: (location_score * 10000.0).round() / 100.0,
        education_score: (edu_score * 10000.0).round() / 100.0,
        matched_skills: matched_skill_names,
        missing_skills: missing,
        matched_at: Local::now().to_string(),
    }
}

/// Match all candidates against all jobs.
pub fn match_all_candidates(
    candidates: &[Candidate],
    jobs: &[Job],
) -> Vec<MatchResult> {
    let total = candidates.len() * jobs.len();
    log::info!("Computing {} candidate-job matches...", total);

    let mut results = Vec::with_capacity(total);

    for (i, candidate) in candidates.iter().enumerate() {
        for job in jobs {
            let result = match_candidate_to_job(candidate, job);
            results.push(result);
        }

        if (i + 1) % 10 == 0 {
            log::info!("Processed {} / {} candidates", i + 1, candidates.len());
        }
    }

    log::info!("Completed {} matches", results.len());
    results
}

/// Find the best matching candidates for a specific job.
pub fn find_best_candidates(
    job: &Job,
    candidates: &[Candidate],
    top_n: usize,
    min_score: f64,
) -> Vec<MatchResult> {
    let mut results: Vec<MatchResult> = candidates
        .iter()
        .map(|c| match_candidate_to_job(c, job))
        .filter(|r| r.overall_score >= min_score)
        .collect();

    results.sort_by(|a, b| {
        b.overall_score
            .partial_cmp(&a.overall_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(top_n);
    results
}

/// Find the best matching jobs for a specific candidate.
pub fn find_best_jobs(
    candidate: &Candidate,
    jobs: &[Job],
    top_n: usize,
    min_score: f64,
) -> Vec<MatchResult> {
    let mut results: Vec<MatchResult> = jobs
        .iter()
        .map(|j| match_candidate_to_job(candidate, j))
        .filter(|r| r.overall_score >= min_score)
        .collect();

    results.sort_by(|a, b| {
        b.overall_score
            .partial_cmp(&a.overall_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(top_n);
    results
}

/// Generate a human-readable match report.
pub fn generate_match_report(results: &[MatchResult], top_n: usize) -> String {
    if results.is_empty() {
        return "No matches found.".to_string();
    }

    let mut sorted = results.to_vec();
    sorted.sort_by(|a, b| {
        b.overall_score
            .partial_cmp(&a.overall_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let n = top_n.min(sorted.len());

    let mut report = format!(
        r#"
╔══════════════════════════════════════════════════════════════════╗
║                    JOBMATCH RESULTS REPORT                       ║
╚══════════════════════════════════════════════════════════════════╝

Total Matches: {}
Report Generated: {}

════════════════════════════════════════════════════════════════════
                        TOP {} MATCHES
════════════════════════════════════════════════════════════════════

"#,
        results.len(),
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        n
    );

    for (i, result) in sorted.iter().take(n).enumerate() {
        let matched_str = if result.matched_skills.is_empty() {
            "None".to_string()
        } else {
            result.matched_skills[..result.matched_skills.len().min(5)].join(", ")
        };
        let missing_str = if result.missing_skills.is_empty() {
            "None".to_string()
        } else {
            result.missing_skills[..result.missing_skills.len().min(5)].join(", ")
        };

        report.push_str(&format!(
            r#"┌──────────────────────────────────────────────────────────────────┐
│ #{}: {}
│     → {}
├──────────────────────────────────────────────────────────────────┤
│ Overall Score: {}%
│ ├─ Skills:     {}%
│ ├─ Experience: {}%
│ ├─ Location:   {}%
│ └─ Education:  {}%
│
│ Matched Skills: {}
│ Missing Skills: {}
└──────────────────────────────────────────────────────────────────┘

"#,
            i + 1,
            result.candidate_name,
            result.job_title,
            result.overall_score,
            result.skill_score,
            result.experience_score,
            result.location_score,
            result.education_score,
            matched_str,
            missing_str,
        ));
    }

    report
}
