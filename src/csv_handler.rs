use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// Read a CSV file and return rows as Vec<HashMap>.
pub fn read_csv(filepath: &str) -> Result<Vec<HashMap<String, String>>> {
    if !Path::new(filepath).is_file() {
        log::warn!("File not found: {}", filepath);
        return Ok(Vec::new());
    }

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(filepath)?;

    let headers = rdr.headers()?.clone();
    let mut rows = Vec::new();

    for result in rdr.records() {
        let record = result?;
        let row: HashMap<String, String> = headers
            .iter()
            .zip(record.iter())
            .map(|(h, v)| {
                let val = if v == "NA" || v == "N/A" {
                    String::new()
                } else {
                    v.to_string()
                };
                (h.to_string(), val)
            })
            .collect();
        rows.push(row);
    }

    Ok(rows)
}

/// Write rows (Vec<HashMap>) to CSV file. Creates directory if needed.
pub fn write_csv(filepath: &str, rows: &[HashMap<String, String>]) -> Result<bool> {
    if rows.is_empty() {
        return Ok(false);
    }

    // Ensure directory exists
    if let Some(parent) = Path::new(filepath).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Collect all unique headers
    let mut headers: Vec<String> = rows
        .first()
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default();
    headers.sort();

    let mut wtr = csv::Writer::from_path(filepath)?;
    wtr.write_record(&headers)?;

    for row in rows {
        let values: Vec<String> = headers
            .iter()
            .map(|h| row.get(h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&values)?;
    }

    wtr.flush()?;
    log::info!("Successfully wrote {} rows to {}", rows.len(), filepath);
    Ok(true)
}

/// Write match results to CSV in ordered column format.
pub fn write_matches_csv(
    filepath: &str,
    matches: &[HashMap<String, String>],
) -> Result<bool> {
    if matches.is_empty() {
        return Ok(false);
    }

    if let Some(parent) = Path::new(filepath).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let headers = [
        "job_id",
        "job_title",
        "candidate_id",
        "candidate_name",
        "overall_score",
        "skill_score",
        "rag_boost",
        "matched_skills",
        "related_skills_matched",
        "missing_skills",
    ];

    let mut wtr = csv::Writer::from_path(filepath)?;
    wtr.write_record(&headers)?;

    for row in matches {
        let values: Vec<String> = headers
            .iter()
            .map(|h| row.get(*h).cloned().unwrap_or_default())
            .collect();
        wtr.write_record(&values)?;
    }

    wtr.flush()?;
    log::info!("Successfully wrote {} matches to {}", matches.len(), filepath);
    Ok(true)
}

/// Read job postings from CSV.
pub fn read_jobs(filepath: Option<&str>) -> Result<Vec<HashMap<String, String>>> {
    let path = filepath.unwrap_or("data/jobs/job_postings_list.csv");
    read_csv(path)
}

/// Read candidates from CSV.
pub fn read_candidates(filepath: Option<&str>) -> Result<Vec<HashMap<String, String>>> {
    let path = filepath.unwrap_or("data/candidates/candidates.csv");
    read_csv(path)
}

/// Read match results from CSV.
pub fn read_matches(filepath: Option<&str>) -> Result<Vec<HashMap<String, String>>> {
    let path = filepath.unwrap_or("data/matches/match_results.csv");
    read_csv(path)
}
