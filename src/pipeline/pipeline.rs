use anyhow::Result;
use chrono::Local;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use uuid::Uuid;

use crate::config;
use crate::db::database::Database;
use crate::db::ingestion::{ingest_cvs_directory, ingest_jobs_directory};
use crate::db::skills_kb::{get_related_skills, get_skill_synonyms_from_db};
use crate::nlp::skill_extractor::normalize_skill;
use crate::rag::rag_support::{export_for_rag, export_skills_taxonomy};

/// Configuration for the pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub jobs_dir: String,
    pub cvs_dir: String,
    pub export_dir: String,
    pub min_match_score: f64,
    pub top_n_matches: usize,
    pub use_rag_enhancement: bool,
    pub force_reprocess: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            jobs_dir: config::jobs_path().to_string_lossy().to_string(),
            cvs_dir: config::resumes_path().to_string_lossy().to_string(),
            export_dir: config::export_path().to_string_lossy().to_string(),
            min_match_score: 30.0,
            top_n_matches: 10,
            use_rag_enhancement: true,
            force_reprocess: false,
        }
    }
}

/// Result from pipeline execution
#[derive(Debug)]
pub struct PipelineResult {
    pub jobs_processed: i64,
    pub candidates_processed: i64,
    pub skills_learned: i64,
    pub matches_found: usize,
    pub execution_time: f64,
}

/// Run the complete JobMatch pipeline.
pub fn run_pipeline(config: &PipelineConfig) -> Result<PipelineResult> {
    let start = Instant::now();

    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║              JOBMATCH - UNIFIED PIPELINE (Rust)                  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Step 1: Initialize Database
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STEP 1: Initializing Database");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let db = Database::new(None)?;
    println!("   Database ready");
    println!();

    // Step 2: Ingest Data & Learn Skills
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STEP 2: Ingesting Data & Learning Skills");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut total_jobs: i64 = 0;
    let mut total_candidates: i64 = 0;
    let mut total_skills: i64 = 0;

    if std::path::Path::new(&config.jobs_dir).is_dir() {
        println!("   Processing jobs from: {}", config.jobs_dir);
        let jobs_result =
            ingest_jobs_directory(&db, &config.jobs_dir, config.force_reprocess)?;
        total_jobs = jobs_result.records;
        total_skills += jobs_result.skills_extracted;
        println!("   Jobs: {}", total_jobs);
        println!("   Skills from jobs: {}", jobs_result.skills_extracted);
    } else {
        println!("   Jobs directory not found: {}", config.jobs_dir);
    }

    if std::path::Path::new(&config.cvs_dir).is_dir() {
        println!("   Processing CVs from: {}", config.cvs_dir);
        let cvs_result =
            ingest_cvs_directory(&db, &config.cvs_dir, config.force_reprocess)?;
        total_candidates = cvs_result.records;
        total_skills += cvs_result.skills_extracted;
        println!("   Candidates: {}", total_candidates);
        println!("   Skills from CVs: {}", cvs_result.skills_extracted);
    } else {
        println!("   CVs directory not found: {}", config.cvs_dir);
    }
    println!();

    // Step 3: RAG-Enhanced Matching
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STEP 3: RAG-Enhanced Matching");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let matches = run_rag_matching(&db, config)?;
    println!("   Total matches computed: {}", matches.len());
    println!();

    // Step 4: Export Results
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STEP 4: Exporting Results");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    std::fs::create_dir_all(&config.export_dir)?;

    // Export matches
    if !matches.is_empty() {
        let matches_file = format!(
            "{}/matches_{}.csv",
            config.export_dir,
            Local::now().format("%Y%m%d_%H%M%S")
        );
        crate::csv_handler::write_matches_csv(&matches_file, &matches)?;
        println!("   Matches exported: {}", matches_file);
    }

    // Export for RAG
    let rag_file = format!("{}/rag_documents.jsonl", config.export_dir);
    let doc_count = export_for_rag(&db, &rag_file)?;
    println!("   RAG documents: {} ({} docs)", rag_file, doc_count);

    // Export skills taxonomy
    let taxonomy_file = format!("{}/skills_taxonomy.json", config.export_dir);
    export_skills_taxonomy(&db, &taxonomy_file)?;
    println!("   Skills taxonomy: {}", taxonomy_file);
    println!();

    // Summary
    let execution_time = start.elapsed().as_secs_f64();
    let stats = db.get_database_stats()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  PIPELINE SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "   Jobs in database:       {}",
        stats.get("jobs").unwrap_or(&0)
    );
    println!(
        "   Candidates in database: {}",
        stats.get("candidates").unwrap_or(&0)
    );
    println!(
        "   Skills in KB:           {}",
        stats.get("skills_kb").unwrap_or(&0)
    );
    println!(
        "   Skill relationships:    {}",
        stats.get("skill_cooccurrence").unwrap_or(&0)
    );
    println!("   Matches computed:       {}", matches.len());
    println!(
        "   Execution time:         {:.2}s",
        execution_time
    );
    println!();

    // Show top matches
    if !matches.is_empty() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  TOP MATCHES");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        display_top_matches(&matches, config.top_n_matches);
    }

    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                    PIPELINE COMPLETE                              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    Ok(PipelineResult {
        jobs_processed: total_jobs,
        candidates_processed: total_candidates,
        skills_learned: total_skills,
        matches_found: matches.len(),
        execution_time,
    })
}

/// Run matching with RAG enhancement.
fn run_rag_matching(
    db: &Database,
    config: &PipelineConfig,
) -> Result<Vec<HashMap<String, String>>> {
    let jobs = db.query("SELECT * FROM jobs", &[])?;
    let candidates = db.query("SELECT * FROM candidates", &[])?;

    if jobs.is_empty() || candidates.is_empty() {
        println!("   No jobs or candidates to match");
        return Ok(Vec::new());
    }

    println!(
        "   Matching {} candidates to {} jobs...",
        candidates.len(),
        jobs.len()
    );

    let mut matches: Vec<HashMap<String, String>> = Vec::new();

    for job_row in &jobs {
        let job_id = job_row.get("job_id").cloned().unwrap_or_default();
        let job_skills = get_entity_skills(db, "job_skills", "job_id", &job_id)?;

        // Get RAG context for this job's skills
        let mut rag_related: HashMap<String, Vec<String>> = HashMap::new();
        if config.use_rag_enhancement {
            for skill in &job_skills {
                if let Ok(related) = get_related_skills(db, skill, 5) {
                    if !related.is_empty() {
                        rag_related
                            .insert(skill.clone(), related.into_iter().map(|r| r.0).collect());
                    }
                }
            }
        }

        for cand_row in &candidates {
            let candidate_id = cand_row.get("candidate_id").cloned().unwrap_or_default();
            let candidate_skills =
                get_entity_skills(db, "candidate_skills", "candidate_id", &candidate_id)?;

            let match_result = compute_rag_enhanced_match(
                db,
                job_row,
                &job_skills,
                &rag_related,
                cand_row,
                &candidate_skills,
                config.use_rag_enhancement,
            )?;

            let overall_score: f64 = match_result
                .get("overall_score")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);

            if overall_score >= config.min_match_score {
                store_match(db, &match_result)?;
                matches.push(match_result);
            }
        }
    }

    // Sort by score
    matches.sort_by(|a, b| {
        let sa: f64 = a
            .get("overall_score")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let sb: f64 = b
            .get("overall_score")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(matches)
}

/// Get skills for an entity from the database.
fn get_entity_skills(
    db: &Database,
    table: &str,
    id_column: &str,
    id_value: &str,
) -> Result<Vec<String>> {
    let rows = db.query(
        &format!("SELECT skill_name FROM {} WHERE {} = ?", table, id_column),
        &[&id_value],
    )?;
    Ok(rows
        .iter()
        .filter_map(|r| r.get("skill_name").cloned())
        .collect())
}

/// Check if there's a synonym match in the knowledge base.
fn has_synonym_match(
    db: &Database,
    skill: &str,
    candidate_skills: &HashSet<String>,
) -> bool {
    if let Ok(synonyms) = get_skill_synonyms_from_db(db, skill) {
        for syn in &synonyms {
            if candidate_skills.contains(&normalize_skill(syn)) {
                return true;
            }
        }
    }
    false
}

/// Compute RAG-enhanced match score.
fn compute_rag_enhanced_match(
    db: &Database,
    job_row: &HashMap<String, String>,
    job_skills: &[String],
    rag_related: &HashMap<String, Vec<String>>,
    cand_row: &HashMap<String, String>,
    candidate_skills: &[String],
    use_rag: bool,
) -> Result<HashMap<String, String>> {
    let mut matched_skills: Vec<String> = Vec::new();
    let mut missing_skills: Vec<String> = Vec::new();
    let mut related_matched: Vec<String> = Vec::new();

    let candidate_skills_normalized: HashSet<String> =
        candidate_skills.iter().map(|s| normalize_skill(s)).collect();

    for job_skill in job_skills {
        let job_skill_norm = normalize_skill(job_skill);

        if candidate_skills_normalized.contains(&job_skill_norm) {
            matched_skills.push(job_skill.clone());
        } else if has_synonym_match(db, &job_skill_norm, &candidate_skills_normalized) {
            matched_skills.push(job_skill.clone());
        } else if use_rag {
            if let Some(related) = rag_related.get(job_skill) {
                let mut found = false;
                for rel_skill in related {
                    if candidate_skills_normalized.contains(&normalize_skill(rel_skill)) {
                        related_matched.push(format!("{} (via {})", job_skill, rel_skill));
                        found = true;
                        break;
                    }
                }
                if !found {
                    missing_skills.push(job_skill.clone());
                }
            } else {
                missing_skills.push(job_skill.clone());
            }
        } else {
            missing_skills.push(job_skill.clone());
        }
    }

    // Calculate scores
    let total_required = job_skills.len() as f64;
    let skill_score = if total_required == 0.0 {
        100.0
    } else {
        let direct_score = matched_skills.len() as f64 / total_required;
        let related_score = related_matched.len() as f64 / total_required * 0.7;
        (direct_score + related_score) * 100.0
    };

    let rag_boost = if use_rag {
        related_matched.len() as f64 * 5.0
    } else {
        0.0
    };

    let overall_score = (skill_score + rag_boost).min(100.0);

    let mut result = HashMap::new();
    result.insert(
        "job_id".to_string(),
        job_row.get("job_id").cloned().unwrap_or_default(),
    );
    result.insert(
        "job_title".to_string(),
        job_row.get("title").cloned().unwrap_or_default(),
    );
    result.insert(
        "candidate_id".to_string(),
        cand_row.get("candidate_id").cloned().unwrap_or_default(),
    );
    result.insert(
        "candidate_name".to_string(),
        cand_row.get("name").cloned().unwrap_or_default(),
    );
    result.insert(
        "overall_score".to_string(),
        format!("{:.2}", overall_score),
    );
    result.insert("skill_score".to_string(), format!("{:.2}", skill_score));
    result.insert("rag_boost".to_string(), format!("{:.2}", rag_boost));
    result.insert("matched_skills".to_string(), matched_skills.join(", "));
    result.insert(
        "related_skills_matched".to_string(),
        related_matched.join(", "),
    );
    result.insert("missing_skills".to_string(), missing_skills.join(", "));

    Ok(result)
}

/// Store match result in database.
fn store_match(db: &Database, match_data: &HashMap<String, String>) -> Result<()> {
    let match_id = Uuid::new_v4().to_string();
    let now = Local::now().to_string();

    let candidate_id = match_data.get("candidate_id").cloned().unwrap_or_default();
    let job_id = match_data.get("job_id").cloned().unwrap_or_default();
    let overall_score: f64 = match_data
        .get("overall_score")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let skill_score: f64 = match_data
        .get("skill_score")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let matched_skills = match_data.get("matched_skills").cloned().unwrap_or_default();
    let missing_skills = match_data.get("missing_skills").cloned().unwrap_or_default();

    db.execute(
        "INSERT INTO matches (match_id, candidate_id, job_id, overall_score, skill_score, matched_skills, missing_skills, matched_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (candidate_id, job_id) DO UPDATE SET
            overall_score = EXCLUDED.overall_score,
            skill_score = EXCLUDED.skill_score,
            matched_skills = EXCLUDED.matched_skills,
            missing_skills = EXCLUDED.missing_skills,
            matched_at = ?",
        &[
            &match_id.as_str(),
            &candidate_id.as_str(),
            &job_id.as_str(),
            &overall_score,
            &skill_score,
            &matched_skills.as_str(),
            &missing_skills.as_str(),
            &now.as_str(),
            &now.as_str(),
        ],
    )?;

    Ok(())
}

/// Display top matches in a formatted way.
fn display_top_matches(matches: &[HashMap<String, String>], n: usize) {
    for (i, row) in matches.iter().take(n).enumerate() {
        println!();
        println!(
            "   #{} | Score: {}%",
            i + 1,
            row.get("overall_score").unwrap_or(&String::new())
        );
        println!(
            "      | Job: {}",
            row.get("job_title").unwrap_or(&String::new())
        );
        println!(
            "      | Candidate: {}",
            row.get("candidate_name").unwrap_or(&String::new())
        );
        println!(
            "      | Skills matched: {}",
            row.get("matched_skills").unwrap_or(&String::new())
        );

        let related = row.get("related_skills_matched").unwrap_or(&String::new()).clone();
        if !related.is_empty() {
            println!(
                "      | RAG boost (+{}): {}",
                row.get("rag_boost").unwrap_or(&String::new()),
                related
            );
        }

        let missing = row.get("missing_skills").unwrap_or(&String::new()).clone();
        if !missing.is_empty() {
            println!("      | Missing: {}", missing);
        }
    }
    println!();
}
