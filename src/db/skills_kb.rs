use anyhow::Result;
use chrono::Local;
use std::collections::HashMap;
use strsim::normalized_levenshtein;
use uuid::Uuid;

use crate::db::database::Database;
use crate::nlp::skill_extractor::{extract_skills_from_text, normalize_skill, SOFT_SKILLS, TECH_SKILLS};

/// Skill categories
pub fn skill_categories() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();
    m.insert(
        "programming",
        vec!["python", "java", "javascript", "c++", "rust", "go", "ruby"],
    );
    m.insert(
        "web",
        vec!["react", "angular", "vue", "nodejs", "html", "css", "django", "flask"],
    );
    m.insert(
        "database",
        vec!["sql", "postgresql", "mongodb", "redis", "mysql", "elasticsearch"],
    );
    m.insert(
        "cloud",
        vec!["aws", "azure", "gcp", "kubernetes", "docker", "terraform"],
    );
    m.insert(
        "data",
        vec!["machine learning", "deep learning", "pandas", "numpy", "spark", "hadoop"],
    );
    m.insert(
        "embedded",
        vec!["embedded systems", "c", "firmware", "rtos", "microcontrollers"],
    );
    m.insert(
        "devops",
        vec!["cicd", "jenkins", "gitlab", "ansible", "prometheus", "grafana"],
    );
    m.insert(
        "testing",
        vec!["unit testing", "selenium", "pytest", "jest", "cypress"],
    );
    m.insert(
        "soft_skills",
        vec!["communication", "teamwork", "leadership", "problem solving"],
    );
    m
}

/// Infer skill category based on known mappings.
fn infer_category(skill: &str) -> String {
    let skill_lower = skill.to_lowercase();

    for (category, skills) in skill_categories() {
        if skills.contains(&skill_lower.as_str()) {
            return category.to_string();
        }
    }

    if TECH_SKILLS.contains(skill_lower.as_str()) {
        return "technical".to_string();
    }
    if SOFT_SKILLS.contains(skill_lower.as_str()) {
        return "soft_skills".to_string();
    }

    "other".to_string()
}

/// Add a new skill to the knowledge base or update frequency if exists.
pub fn add_skill(
    db: &Database,
    skill_name: &str,
    category: &str,
    source_type: &str,
) -> Result<String> {
    let canonical = normalize_skill(skill_name);

    let rows = db.query(
        "SELECT skill_id, frequency FROM skills_kb WHERE canonical_name = ?",
        &[&canonical],
    )?;

    if let Some(row) = rows.first() {
        let skill_id = row.get("skill_id").cloned().unwrap_or_default();
        let freq: i64 = row
            .get("frequency")
            .and_then(|f| f.parse().ok())
            .unwrap_or(0)
            + 1;
        let now = Local::now().to_string();
        db.execute(
            "UPDATE skills_kb SET frequency = ?, last_seen_at = ? WHERE skill_id = ?",
            &[&freq, &now.as_str(), &skill_id.as_str()],
        )?;
        Ok(skill_id)
    } else {
        let skill_id = Uuid::new_v4().to_string();
        let cat = if category.is_empty() {
            infer_category(&canonical)
        } else {
            category.to_string()
        };

        db.execute(
            "INSERT INTO skills_kb (skill_id, skill_name, canonical_name, category, frequency, source_type) VALUES (?, ?, ?, ?, 1, ?)",
            &[&skill_id.as_str(), &skill_name, &canonical.as_str(), &cat.as_str(), &source_type],
        )?;
        Ok(skill_id)
    }
}

/// Add a skill synonym mapping.
pub fn add_synonym(
    db: &Database,
    synonym: &str,
    canonical: &str,
    confidence: f64,
    source: &str,
) -> Result<()> {
    let synonym_lower = synonym.to_lowercase();
    let canonical_norm = normalize_skill(canonical);

    let count = db.query_count(
        "SELECT COUNT(*) FROM skill_synonyms WHERE skill_name = ? AND canonical_name = ?",
        &[&synonym_lower.as_str(), &canonical_norm.as_str()],
    )?;

    if count == 0 {
        let synonym_id = Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO skill_synonyms (synonym_id, skill_name, canonical_name, confidence, source) VALUES (?, ?, ?, ?, ?)",
            &[&synonym_id.as_str(), &synonym_lower.as_str(), &canonical_norm.as_str(), &confidence, &source],
        )?;
    }
    Ok(())
}

/// Extract skills from text and add them to knowledge base.
pub fn learn_from_text(
    db: &Database,
    text: &str,
    source_type: &str,
    _source_id: &str,
) -> Result<Vec<String>> {
    let skills = extract_skills_from_text(text);
    let mut learned = Vec::new();

    for skill in &skills {
        add_skill(db, skill, "", source_type)?;
        learned.push(skill.clone());
    }

    if skills.len() > 1 {
        learn_cooccurrence(db, &skills, source_type)?;
    }

    Ok(learned)
}

/// Update skill co-occurrence matrix.
fn learn_cooccurrence(db: &Database, skills: &[String], source_type: &str) -> Result<()> {
    for i in 0..skills.len() {
        for j in (i + 1)..skills.len() {
            let mut skill_a = normalize_skill(&skills[i]);
            let mut skill_b = normalize_skill(&skills[j]);

            // Ensure consistent ordering
            if skill_a > skill_b {
                std::mem::swap(&mut skill_a, &mut skill_b);
            }

            let rows = db.query(
                "SELECT id, cooccurrence_count FROM skill_cooccurrence WHERE skill_a = ? AND skill_b = ?",
                &[&skill_a.as_str(), &skill_b.as_str()],
            )?;

            if let Some(row) = rows.first() {
                let id = row.get("id").cloned().unwrap_or_default();
                let new_count: i64 = row
                    .get("cooccurrence_count")
                    .and_then(|c| c.parse().ok())
                    .unwrap_or(0)
                    + 1;
                let now = Local::now().to_string();
                db.execute(
                    "UPDATE skill_cooccurrence SET cooccurrence_count = ?, last_updated = ? WHERE id = ?",
                    &[&new_count, &now.as_str(), &id.as_str()],
                )?;
            } else {
                let id = Uuid::new_v4().to_string();
                db.execute(
                    "INSERT INTO skill_cooccurrence (id, skill_a, skill_b, cooccurrence_count, source_type) VALUES (?, ?, ?, 1, ?)",
                    &[&id.as_str(), &skill_a.as_str(), &skill_b.as_str(), &source_type],
                )?;
            }
        }
    }
    Ok(())
}

/// Get skills that frequently co-occur with the given skill.
pub fn get_related_skills(
    db: &Database,
    skill: &str,
    top_n: usize,
) -> Result<Vec<(String, i64)>> {
    let canonical = normalize_skill(skill);
    let top_n_i64 = top_n as i64;

    let rows = db.query(
        "SELECT
            CASE WHEN skill_a = ? THEN skill_b ELSE skill_a END as related_skill,
            cooccurrence_count
        FROM skill_cooccurrence
        WHERE skill_a = ? OR skill_b = ?
        ORDER BY cooccurrence_count DESC
        LIMIT ?",
        &[
            &canonical.as_str(),
            &canonical.as_str(),
            &canonical.as_str(),
            &top_n_i64,
        ],
    )?;

    Ok(rows
        .iter()
        .map(|row| {
            let skill = row.get("related_skill").cloned().unwrap_or_default();
            let count: i64 = row
                .get("cooccurrence_count")
                .and_then(|c| c.parse().ok())
                .unwrap_or(0);
            (skill, count)
        })
        .collect())
}

/// Find skills similar to the given skill using string similarity.
pub fn find_similar_skills(
    db: &Database,
    skill: &str,
    threshold: f64,
    top_n: usize,
) -> Result<Vec<(String, f64)>> {
    let canonical = normalize_skill(skill);

    let rows = db.query(
        "SELECT canonical_name FROM skills_kb",
        &[],
    )?;

    let mut similar: Vec<(String, f64)> = rows
        .iter()
        .filter_map(|row| {
            let other = row.get("canonical_name")?.clone();
            if other == canonical {
                return None;
            }
            let sim = normalized_levenshtein(&canonical, &other);
            if sim >= threshold {
                Some((other, sim))
            } else {
                None
            }
        })
        .collect();

    similar.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    similar.truncate(top_n);
    Ok(similar)
}

/// Get all known synonyms for a skill from database.
pub fn get_skill_synonyms_from_db(db: &Database, skill: &str) -> Result<Vec<String>> {
    let canonical = normalize_skill(skill);
    let rows = db.query(
        "SELECT skill_name FROM skill_synonyms WHERE canonical_name = ?",
        &[&canonical.as_str()],
    )?;

    Ok(rows
        .iter()
        .filter_map(|row| row.get("skill_name").cloned())
        .collect())
}

/// Get skills for RAG embedding export.
pub fn get_skills_for_rag(
    db: &Database,
    min_frequency: i64,
    include_related: bool,
) -> Result<Vec<HashMap<String, serde_json::Value>>> {
    let rows = db.query(
        "SELECT * FROM skills_kb WHERE frequency >= ? ORDER BY frequency DESC",
        &[&min_frequency],
    )?;

    let mut rag_skills = Vec::new();

    for row in &rows {
        let canonical = row.get("canonical_name").cloned().unwrap_or_default();
        let category = row.get("category").cloned().unwrap_or_default();
        let frequency: i64 = row
            .get("frequency")
            .and_then(|f| f.parse().ok())
            .unwrap_or(0);

        let synonyms = get_skill_synonyms_from_db(db, &canonical)?;

        let mut skill_data: HashMap<String, serde_json::Value> = HashMap::new();
        skill_data.insert(
            "skill_name".to_string(),
            serde_json::Value::String(canonical.clone()),
        );
        skill_data.insert(
            "category".to_string(),
            serde_json::Value::String(category.clone()),
        );
        skill_data.insert(
            "frequency".to_string(),
            serde_json::json!(frequency),
        );
        skill_data.insert(
            "synonyms".to_string(),
            serde_json::json!(synonyms),
        );

        if include_related {
            let related = get_related_skills(db, &canonical, 5)?;
            let related_names: Vec<String> = related.into_iter().map(|r| r.0).collect();
            skill_data.insert(
                "related_skills".to_string(),
                serde_json::json!(related_names),
            );
        }

        // Generate RAG-friendly text description
        let mut parts = vec![format!("Skill: {}", canonical)];
        if !category.is_empty() {
            parts.push(format!("Category: {}", category));
        }
        if !synonyms.is_empty() {
            parts.push(format!("Also known as: {}", synonyms.join(", ")));
        }
        if let Some(serde_json::Value::Array(related)) = skill_data.get("related_skills") {
            let related_strs: Vec<String> = related
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !related_strs.is_empty() {
                parts.push(format!("Related skills: {}", related_strs.join(", ")));
            }
        }
        skill_data.insert(
            "rag_text".to_string(),
            serde_json::Value::String(parts.join(". ")),
        );

        rag_skills.push(skill_data);
    }

    Ok(rag_skills)
}

/// Export the entire knowledge base to a JSON file.
pub fn export_knowledge_base(db: &Database, filepath: &str) -> Result<()> {
    let skills = get_skills_for_rag(db, 1, true)?;

    let synonyms_rows = db.query("SELECT * FROM skill_synonyms", &[])?;
    let cooc_rows = db.query(
        "SELECT * FROM skill_cooccurrence ORDER BY cooccurrence_count DESC LIMIT 1000",
        &[],
    )?;

    let export_data = serde_json::json!({
        "skills": skills,
        "synonyms": synonyms_rows,
        "cooccurrence": cooc_rows.iter().map(|row| {
            serde_json::json!({
                "skill_a": row.get("skill_a").cloned().unwrap_or_default(),
                "skill_b": row.get("skill_b").cloned().unwrap_or_default(),
                "count": row.get("cooccurrence_count").and_then(|c| c.parse::<i64>().ok()).unwrap_or(0),
            })
        }).collect::<Vec<_>>(),
        "exported_at": Local::now().to_string(),
        "total_skills": skills.len(),
    });

    let file = std::fs::File::create(filepath)?;
    serde_json::to_writer_pretty(file, &export_data)?;

    log::info!(
        "Knowledge base exported to {} ({} skills)",
        filepath,
        skills.len()
    );
    Ok(())
}
