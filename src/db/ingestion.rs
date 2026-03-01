use anyhow::Result;
use chrono::Local;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::db::database::Database;
use crate::db::skills_kb::learn_from_text;
use crate::parsers::pdf_parser;

/// Compute SHA256 hash of file for duplicate detection.
pub fn compute_file_hash(filepath: &str) -> Result<String> {
    let data = std::fs::read(filepath)?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}

/// Check if a file has already been processed.
fn is_file_processed(db: &Database, file_hash: &str) -> Result<bool> {
    let count = db.query_count(
        "SELECT COUNT(*) FROM ingestion_log WHERE file_hash = ? AND status = 'completed'",
        &[&file_hash],
    )?;
    Ok(count > 0)
}

/// Log the start of file ingestion.
fn log_ingestion_start(
    db: &Database,
    filepath: &str,
    file_hash: &str,
    file_type: &str,
) -> Result<String> {
    let log_id = Uuid::new_v4().to_string();
    let now = Local::now().to_string();

    // Try insert, on conflict update
    let _ = db.execute(
        "INSERT INTO ingestion_log (log_id, file_path, file_hash, file_type, status, started_at)
         VALUES (?, ?, ?, ?, 'processing', ?)
         ON CONFLICT (file_hash) DO UPDATE SET status = 'processing', started_at = EXCLUDED.started_at",
        &[
            &log_id.as_str(),
            &filepath,
            &file_hash,
            &file_type,
            &now.as_str(),
        ],
    );

    Ok(log_id)
}

/// Log successful completion of ingestion.
fn log_ingestion_complete(db: &Database, log_id: &str, records: i64, skills: i64) -> Result<()> {
    let now = Local::now().to_string();
    db.execute(
        "UPDATE ingestion_log SET status = 'completed', records_processed = ?, skills_extracted = ?, completed_at = ? WHERE log_id = ?",
        &[&records, &skills, &now.as_str(), &log_id],
    )?;
    Ok(())
}

/// Log ingestion error.
fn log_ingestion_error(db: &Database, log_id: &str, error_msg: &str) -> Result<()> {
    let now = Local::now().to_string();
    db.execute(
        "UPDATE ingestion_log SET status = 'error', error_message = ?, completed_at = ? WHERE log_id = ?",
        &[&error_msg, &now.as_str(), &log_id],
    )?;
    Ok(())
}

/// Extract years of experience from resume text.
fn extract_experience_years(text: &str) -> i32 {
    let patterns = [
        r"(\d+)\+?\s*years?\s*(?:of\s*)?(?:experience|exp)",
        r"(?:experience|exp)\s*:?\s*(\d+)\+?\s*years?",
        r"(\d+)\+?\s*years?\s*(?:in|of|working)",
    ];

    let text_lower = text.to_lowercase();
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(&text_lower) {
                if let Some(m) = caps.get(1) {
                    if let Ok(years) = m.as_str().parse::<i32>() {
                        return years;
                    }
                }
            }
        }
    }
    0
}

/// Result from ingesting data
#[derive(Debug, Clone)]
pub struct IngestionResult {
    pub success: bool,
    pub skipped: bool,
    pub message: String,
    pub records: i64,
    pub skills_extracted: i64,
}

/// Ingest a job posting CSV file into the database.
pub fn ingest_job_csv(db: &Database, filepath: &str, force: bool) -> Result<IngestionResult> {
    if !Path::new(filepath).is_file() {
        return Ok(IngestionResult {
            success: false,
            skipped: false,
            message: format!("File not found: {}", filepath),
            records: 0,
            skills_extracted: 0,
        });
    }

    let file_hash = compute_file_hash(filepath)?;

    if !force && is_file_processed(db, &file_hash)? {
        return Ok(IngestionResult {
            success: true,
            skipped: true,
            message: "File already processed".to_string(),
            records: 0,
            skills_extracted: 0,
        });
    }

    let log_id = log_ingestion_start(db, filepath, &file_hash, "job_csv")?;

    match ingest_job_csv_inner(db, filepath, &file_hash) {
        Ok(result) => {
            log_ingestion_complete(db, &log_id, result.records, result.skills_extracted)?;
            Ok(result)
        }
        Err(e) => {
            log_ingestion_error(db, &log_id, &e.to_string())?;
            Ok(IngestionResult {
                success: false,
                skipped: false,
                message: e.to_string(),
                records: 0,
                skills_extracted: 0,
            })
        }
    }
}

fn ingest_job_csv_inner(
    db: &Database,
    filepath: &str,
    _file_hash: &str,
) -> Result<IngestionResult> {
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(filepath)?;

    let headers = rdr.headers()?.clone();
    let mut jobs_added: i64 = 0;
    let mut skills_extracted: i64 = 0;

    for result in rdr.records() {
        let record = result?;
        let row: HashMap<String, String> = headers
            .iter()
            .zip(record.iter())
            .map(|(h, v)| (h.to_string(), v.to_string()))
            .collect();

        let job_id = row
            .get("ID")
            .or_else(|| row.get("job_id"))
            .cloned()
            .unwrap_or_default();

        if job_id.is_empty() {
            continue;
        }

        // Build text for skill extraction
        let mut text_parts = Vec::new();
        for field in ["Title", "title", "description", "requirements", "responsibilities", "skills"]
        {
            if let Some(val) = row.get(field) {
                if !val.is_empty() {
                    text_parts.push(val.clone());
                }
            }
        }
        let combined_text = text_parts.join(" ");

        // Learn skills
        let job_skills = learn_from_text(db, &combined_text, "job", &job_id)?;
        skills_extracted += job_skills.len() as i64;

        // Helper closures
        let get_str = |keys: &[&str]| -> String {
            for k in keys {
                if let Some(v) = row.get(*k) {
                    if !v.is_empty() && v != "NA" && v != "N/A" {
                        return v.clone();
                    }
                }
            }
            String::new()
        };

        let get_int = |keys: &[&str], default: i64| -> i64 {
            for k in keys {
                if let Some(v) = row.get(*k) {
                    if let Ok(n) = v.parse::<i64>() {
                        return n;
                    }
                }
            }
            default
        };

        let title = get_str(&["Title", "title"]);
        let buyer = get_str(&["Buyer", "buyer"]);
        let location = get_str(&["Site", "location"]);
        let status = get_str(&["Status", "status"]);
        let start_date = get_str(&["Start", "start_date"]);
        let end_date = get_str(&["End", "end_date"]);
        let respond_by = get_str(&["Respond By", "respond_by"]);
        let positions = get_int(&["Positions", "positions"], 1);
        let business_unit = get_str(&["Business Unit", "business_unit"]);
        let contingent_type = get_str(&["Contingent Type", "contingent_type"]);
        let buyer_reference = get_str(&["Buyer Reference", "buyer_reference"]);
        let skills_raw = job_skills.join(", ");
        let now = Local::now().to_string();

        db.execute(
            "INSERT INTO jobs (job_id, title, buyer, location, status, start_date, end_date, respond_by, positions, business_unit, contingent_type, buyer_reference, source_file, skills_raw)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (job_id) DO UPDATE SET updated_at = ?, skills_raw = EXCLUDED.skills_raw",
            &[
                &job_id.as_str(),
                &title.as_str(),
                &buyer.as_str(),
                &location.as_str(),
                &status.as_str(),
                &start_date.as_str(),
                &end_date.as_str(),
                &respond_by.as_str(),
                &positions,
                &business_unit.as_str(),
                &contingent_type.as_str(),
                &buyer_reference.as_str(),
                &filepath,
                &skills_raw.as_str(),
                &now.as_str(),
            ],
        )?;

        // Add job-skill associations
        for skill in &job_skills {
            let id = Uuid::new_v4().to_string();
            let _ = db.execute(
                "INSERT INTO job_skills (id, job_id, skill_name) VALUES (?, ?, ?) ON CONFLICT (job_id, skill_name) DO NOTHING",
                &[&id.as_str(), &job_id.as_str(), &skill.as_str()],
            );
        }

        jobs_added += 1;
    }

    Ok(IngestionResult {
        success: true,
        skipped: false,
        message: format!("{} jobs added, {} skills extracted", jobs_added, skills_extracted),
        records: jobs_added,
        skills_extracted,
    })
}

/// Ingest a CV/resume PDF file into the database.
pub fn ingest_cv_pdf(db: &Database, filepath: &str, force: bool) -> Result<IngestionResult> {
    if !Path::new(filepath).is_file() {
        return Ok(IngestionResult {
            success: false,
            skipped: false,
            message: format!("File not found: {}", filepath),
            records: 0,
            skills_extracted: 0,
        });
    }

    let file_hash = compute_file_hash(filepath)?;

    if !force && is_file_processed(db, &file_hash)? {
        return Ok(IngestionResult {
            success: true,
            skipped: true,
            message: "File already processed".to_string(),
            records: 0,
            skills_extracted: 0,
        });
    }

    let log_id = log_ingestion_start(db, filepath, &file_hash, "cv_pdf")?;

    match ingest_cv_pdf_inner(db, filepath, &file_hash) {
        Ok(result) => {
            log_ingestion_complete(db, &log_id, result.records, result.skills_extracted)?;
            Ok(result)
        }
        Err(e) => {
            log_ingestion_error(db, &log_id, &e.to_string())?;
            Ok(IngestionResult {
                success: false,
                skipped: false,
                message: e.to_string(),
                records: 0,
                skills_extracted: 0,
            })
        }
    }
}

fn ingest_cv_pdf_inner(
    db: &Database,
    filepath: &str,
    file_hash: &str,
) -> Result<IngestionResult> {
    let parsed = pdf_parser::parse_resume(filepath);

    if parsed.is_empty() {
        return Ok(IngestionResult {
            success: false,
            skipped: false,
            message: "Could not parse PDF".to_string(),
            records: 0,
            skills_extracted: 0,
        });
    }

    let candidate_id = Uuid::new_v4().to_string();
    let raw_text = parsed.get("raw_text").cloned().unwrap_or_default();

    // Learn skills
    let candidate_skills = learn_from_text(db, &raw_text, "cv", &candidate_id)?;
    let experience_years = extract_experience_years(&raw_text);

    let name = parsed.get("name").cloned().unwrap_or_default();
    let email = parsed.get("email").cloned().unwrap_or_default();
    let phone = parsed.get("phone").cloned().unwrap_or_default();
    let location = parsed.get("location").cloned().unwrap_or_default();
    let education = parsed.get("education_section").cloned().unwrap_or_default();
    let summary = parsed.get("summary_section").cloned().unwrap_or_default();
    let skills_raw = candidate_skills.join(", ");

    db.execute(
        "INSERT INTO candidates (candidate_id, name, email, phone, skills_raw, experience_years, education, location, summary, resume_path, resume_text, resume_hash, source_file)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        &[
            &candidate_id.as_str(),
            &name.as_str(),
            &email.as_str(),
            &phone.as_str(),
            &skills_raw.as_str(),
            &(experience_years as i64),
            &education.as_str(),
            &location.as_str(),
            &summary.as_str(),
            &filepath,
            &raw_text.as_str(),
            &file_hash,
            &filepath,
        ],
    )?;

    // Add candidate-skill associations
    for skill in &candidate_skills {
        let id = Uuid::new_v4().to_string();
        let _ = db.execute(
            "INSERT INTO candidate_skills (id, candidate_id, skill_name) VALUES (?, ?, ?) ON CONFLICT (candidate_id, skill_name) DO NOTHING",
            &[&id.as_str(), &candidate_id.as_str(), &skill.as_str()],
        );
    }

    Ok(IngestionResult {
        success: true,
        skipped: false,
        message: format!("Candidate {} added with {} skills", name, candidate_skills.len()),
        records: 1,
        skills_extracted: candidate_skills.len() as i64,
    })
}

/// Ingest all CSV files from a directory.
pub fn ingest_jobs_directory(
    db: &Database,
    dirpath: &str,
    force: bool,
) -> Result<IngestionResult> {
    let path = Path::new(dirpath);
    if !path.is_dir() {
        return Ok(IngestionResult {
            success: false,
            skipped: false,
            message: format!("Directory not found: {}", dirpath),
            records: 0,
            skills_extracted: 0,
        });
    }

    let mut total_jobs: i64 = 0;
    let mut total_skills: i64 = 0;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();
        if let Some(ext) = file_path.extension() {
            if ext.to_str().map(|s| s.to_lowercase()) == Some("csv".to_string()) {
                let filepath = file_path.to_string_lossy().to_string();
                log::info!("Ingesting: {}", filepath);
                let result = ingest_job_csv(db, &filepath, force)?;
                if result.success && !result.skipped {
                    total_jobs += result.records;
                    total_skills += result.skills_extracted;
                }
            }
        }
    }

    Ok(IngestionResult {
        success: true,
        skipped: false,
        message: format!("{} jobs, {} skills", total_jobs, total_skills),
        records: total_jobs,
        skills_extracted: total_skills,
    })
}

/// Ingest all PDF files from a directory.
pub fn ingest_cvs_directory(
    db: &Database,
    dirpath: &str,
    force: bool,
) -> Result<IngestionResult> {
    let path = Path::new(dirpath);
    if !path.is_dir() {
        return Ok(IngestionResult {
            success: false,
            skipped: false,
            message: format!("Directory not found: {}", dirpath),
            records: 0,
            skills_extracted: 0,
        });
    }

    let mut total_candidates: i64 = 0;
    let mut total_skills: i64 = 0;

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();
        if let Some(ext) = file_path.extension() {
            if ext.to_str().map(|s| s.to_lowercase()) == Some("pdf".to_string()) {
                let filepath = file_path.to_string_lossy().to_string();
                log::info!("Ingesting: {}", filepath);
                let result = ingest_cv_pdf(db, &filepath, force)?;
                if result.success && !result.skipped {
                    total_candidates += result.records;
                    total_skills += result.skills_extracted;
                }
            }
        }
    }

    Ok(IngestionResult {
        success: true,
        skipped: false,
        message: format!("{} candidates, {} skills", total_candidates, total_skills),
        records: total_candidates,
        skills_extracted: total_skills,
    })
}

/// Get status of all file ingestions.
pub fn get_ingestion_status(db: &Database) -> Result<Vec<HashMap<String, String>>> {
    db.query(
        "SELECT file_path, file_type, status, records_processed, skills_extracted, started_at, completed_at
         FROM ingestion_log ORDER BY started_at DESC",
        &[],
    )
}
