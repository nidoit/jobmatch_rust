#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

use jobmatch::config;
use jobmatch::db::database::Database;
use jobmatch::db::ingestion;
use jobmatch::matching::matcher;
use jobmatch::models::candidate::Candidate;
use jobmatch::models::job::Job;
use jobmatch::nlp::skill_extractor;
use jobmatch::pipeline::pipeline::{run_pipeline, PipelineConfig};
use jobmatch::rag::rag_support;

/// Shared application state holding the database connection.
struct AppState {
    db: Mutex<Database>,
}

// ─── Response types ────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
struct DbStats {
    jobs: i64,
    candidates: i64,
    skills_kb: i64,
    skill_cooccurrence: i64,
    matches: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct MatchRow {
    job_id: String,
    job_title: String,
    candidate_id: String,
    candidate_name: String,
    overall_score: f64,
    skill_score: f64,
    matched_skills: String,
    missing_skills: String,
    rag_boost: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillRow {
    skill_name: String,
    canonical_name: String,
    category: String,
    frequency: i64,
}

#[derive(Serialize, Deserialize, Clone)]
struct PipelineResponse {
    success: bool,
    jobs_processed: i64,
    candidates_processed: i64,
    skills_learned: i64,
    matches_found: usize,
    execution_time: f64,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct IngestResponse {
    success: bool,
    records: i64,
    skills_extracted: i64,
    message: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct SkillAnalysis {
    skills: Vec<String>,
    categories: HashMap<String, Vec<String>>,
}

// ─── Tauri commands ────────────────────────────────────────────────────

#[tauri::command]
fn get_db_stats(state: State<AppState>) -> Result<DbStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let stats = db.get_database_stats().map_err(|e| e.to_string())?;

    Ok(DbStats {
        jobs: *stats.get("jobs").unwrap_or(&0),
        candidates: *stats.get("candidates").unwrap_or(&0),
        skills_kb: *stats.get("skills_kb").unwrap_or(&0),
        skill_cooccurrence: *stats.get("skill_cooccurrence").unwrap_or(&0),
        matches: *stats.get("matches").unwrap_or(&0),
    })
}

#[tauri::command]
fn run_full_pipeline(
    force: bool,
    use_rag: bool,
    min_score: f64,
    top_n: usize,
) -> Result<PipelineResponse, String> {
    let pipeline_config = PipelineConfig {
        jobs_dir: config::jobs_path().to_string_lossy().to_string(),
        cvs_dir: config::resumes_path().to_string_lossy().to_string(),
        export_dir: config::export_path().to_string_lossy().to_string(),
        min_match_score: min_score,
        top_n_matches: top_n,
        use_rag_enhancement: use_rag,
        force_reprocess: force,
    };

    match run_pipeline(&pipeline_config) {
        Ok(result) => Ok(PipelineResponse {
            success: true,
            jobs_processed: result.jobs_processed,
            candidates_processed: result.candidates_processed,
            skills_learned: result.skills_learned,
            matches_found: result.matches_found,
            execution_time: result.execution_time,
            message: format!(
                "Pipeline completed: {} matches found in {:.2}s",
                result.matches_found, result.execution_time
            ),
        }),
        Err(e) => Ok(PipelineResponse {
            success: false,
            jobs_processed: 0,
            candidates_processed: 0,
            skills_learned: 0,
            matches_found: 0,
            execution_time: 0.0,
            message: format!("Pipeline error: {}", e),
        }),
    }
}

#[tauri::command]
fn get_matches(state: State<AppState>) -> Result<Vec<MatchRow>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .query(
            "SELECT m.*, j.title as job_title, c.name as candidate_name
         FROM matches m
         LEFT JOIN jobs j ON m.job_id = j.job_id
         LEFT JOIN candidates c ON m.candidate_id = c.candidate_id
         ORDER BY m.overall_score DESC
         LIMIT 100",
            &[],
        )
        .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| MatchRow {
            job_id: r.get("job_id").cloned().unwrap_or_default(),
            job_title: r.get("job_title").cloned().unwrap_or_default(),
            candidate_id: r.get("candidate_id").cloned().unwrap_or_default(),
            candidate_name: r.get("candidate_name").cloned().unwrap_or_default(),
            overall_score: r
                .get("overall_score")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            skill_score: r
                .get("skill_score")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            matched_skills: r.get("matched_skills").cloned().unwrap_or_default(),
            missing_skills: r.get("missing_skills").cloned().unwrap_or_default(),
            rag_boost: String::new(),
        })
        .collect())
}

#[tauri::command]
fn get_jobs(state: State<AppState>) -> Result<Vec<HashMap<String, String>>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.query("SELECT * FROM jobs ORDER BY title", &[])
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_candidates(state: State<AppState>) -> Result<Vec<HashMap<String, String>>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.query("SELECT * FROM candidates ORDER BY name", &[])
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_skills(state: State<AppState>) -> Result<Vec<SkillRow>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let rows = db
        .query(
            "SELECT skill_name, canonical_name, category, frequency FROM skills_kb ORDER BY frequency DESC LIMIT 200",
            &[],
        )
        .map_err(|e| e.to_string())?;

    Ok(rows
        .iter()
        .map(|r| SkillRow {
            skill_name: r.get("skill_name").cloned().unwrap_or_default(),
            canonical_name: r.get("canonical_name").cloned().unwrap_or_default(),
            category: r.get("category").cloned().unwrap_or_default(),
            frequency: r
                .get("frequency")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        })
        .collect())
}

#[tauri::command]
fn ingest_jobs(state: State<AppState>, force: bool) -> Result<IngestResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let jobs_dir = config::jobs_path().to_string_lossy().to_string();

    match ingestion::ingest_jobs_directory(&db, &jobs_dir, force) {
        Ok(result) => Ok(IngestResponse {
            success: result.success,
            records: result.records,
            skills_extracted: result.skills_extracted,
            message: result.message,
        }),
        Err(e) => Ok(IngestResponse {
            success: false,
            records: 0,
            skills_extracted: 0,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
fn ingest_cvs(state: State<AppState>, force: bool) -> Result<IngestResponse, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let cvs_dir = config::resumes_path().to_string_lossy().to_string();

    match ingestion::ingest_cvs_directory(&db, &cvs_dir, force) {
        Ok(result) => Ok(IngestResponse {
            success: result.success,
            records: result.records,
            skills_extracted: result.skills_extracted,
            message: result.message,
        }),
        Err(e) => Ok(IngestResponse {
            success: false,
            records: 0,
            skills_extracted: 0,
            message: e.to_string(),
        }),
    }
}

#[tauri::command]
fn analyze_text(text: String) -> Result<SkillAnalysis, String> {
    let skills = skill_extractor::extract_skills_from_text(&text);
    let categories = skill_extractor::categorize_skills(&skills);

    Ok(SkillAnalysis { skills, categories })
}

#[tauri::command]
fn match_single(
    _state: State<AppState>,
    candidate_skills: String,
    job_skills: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let cand = Candidate {
        name: "Manual Input".to_string(),
        skills: candidate_skills,
        ..Default::default()
    };

    let job = Job {
        title: "Manual Input".to_string(),
        skills: job_skills,
        ..Default::default()
    };

    let result = matcher::match_candidate_to_job(&cand, &job);

    let mut response = HashMap::new();
    response.insert(
        "overall_score".to_string(),
        serde_json::json!(result.overall_score),
    );
    response.insert(
        "skill_score".to_string(),
        serde_json::json!(result.skill_score),
    );
    response.insert(
        "matched_skills".to_string(),
        serde_json::json!(result.matched_skills),
    );
    response.insert(
        "missing_skills".to_string(),
        serde_json::json!(result.missing_skills),
    );

    Ok(response)
}

#[tauri::command]
fn export_rag(state: State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let export_dir = config::export_path().to_string_lossy().to_string();
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let rag_file = format!("{}/rag_documents.jsonl", export_dir);
    let count = rag_support::export_for_rag(&db, &rag_file).map_err(|e| e.to_string())?;

    let taxonomy_file = format!("{}/skills_taxonomy.json", export_dir);
    rag_support::export_skills_taxonomy(&db, &taxonomy_file).map_err(|e| e.to_string())?;

    Ok(format!(
        "Exported {} RAG documents and skills taxonomy to {}",
        count, export_dir
    ))
}

fn main() {
    let db = Database::new(None).expect("Failed to initialize database");

    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(db),
        })
        .invoke_handler(tauri::generate_handler![
            get_db_stats,
            run_full_pipeline,
            get_matches,
            get_jobs,
            get_candidates,
            get_skills,
            ingest_jobs,
            ingest_cvs,
            analyze_text,
            match_single,
            export_rag,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
