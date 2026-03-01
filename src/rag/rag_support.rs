use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use crate::db::database::Database;
use crate::db::skills_kb::{get_related_skills, get_skills_for_rag};

/// Structure for a document prepared for RAG embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGDocument {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Convert a job record to a RAG-ready document.
fn prepare_job_for_embedding(row: &HashMap<String, String>) -> RAGDocument {
    let mut content_parts = Vec::new();
    let title = row.get("title").cloned().unwrap_or_default();

    if !title.is_empty() {
        content_parts.push(format!("Job Title: {}", title));
    }

    let location = row.get("location").cloned().unwrap_or_default();
    if !location.is_empty() {
        content_parts.push(format!("Location: {}", location));
    }

    let skills_raw = row.get("skills_raw").cloned().unwrap_or_default();
    if !skills_raw.is_empty() {
        content_parts.push(format!("Required Skills: {}", skills_raw));
    }

    let description = row.get("description").cloned().unwrap_or_default();
    if !description.is_empty() {
        content_parts.push(format!("Description: {}", description));
    }

    let requirements = row.get("requirements").cloned().unwrap_or_default();
    if !requirements.is_empty() {
        content_parts.push(format!("Requirements: {}", requirements));
    }

    let mut metadata = HashMap::new();
    metadata.insert(
        "buyer".to_string(),
        serde_json::Value::String(row.get("buyer").cloned().unwrap_or_default()),
    );
    metadata.insert(
        "status".to_string(),
        serde_json::Value::String(row.get("status").cloned().unwrap_or_default()),
    );
    metadata.insert(
        "business_unit".to_string(),
        serde_json::Value::String(row.get("business_unit").cloned().unwrap_or_default()),
    );

    RAGDocument {
        id: Uuid::new_v4().to_string(),
        entity_type: "job".to_string(),
        entity_id: row.get("job_id").cloned().unwrap_or_default(),
        title,
        content: content_parts.join("\n"),
        metadata,
    }
}

/// Convert a candidate record to a RAG-ready document.
fn prepare_candidate_for_embedding(row: &HashMap<String, String>) -> RAGDocument {
    let mut content_parts = Vec::new();
    let name = row.get("name").cloned().unwrap_or_default();

    if !name.is_empty() {
        content_parts.push(format!("Candidate: {}", name));
    }

    let skills_raw = row.get("skills_raw").cloned().unwrap_or_default();
    if !skills_raw.is_empty() {
        content_parts.push(format!("Skills: {}", skills_raw));
    }

    let exp_years = row
        .get("experience_years")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);
    if exp_years > 0 {
        content_parts.push(format!("Experience: {} years", exp_years));
    }

    let location = row.get("location").cloned().unwrap_or_default();
    if !location.is_empty() {
        content_parts.push(format!("Location: {}", location));
    }

    let education = row.get("education").cloned().unwrap_or_default();
    if !education.is_empty() {
        content_parts.push(format!("Education: {}", education));
    }

    let summary = row.get("summary").cloned().unwrap_or_default();
    if !summary.is_empty() {
        content_parts.push(format!("Summary: {}", summary));
    }

    let mut metadata = HashMap::new();
    metadata.insert(
        "email".to_string(),
        serde_json::Value::String(row.get("email").cloned().unwrap_or_default()),
    );
    metadata.insert(
        "experience_years".to_string(),
        serde_json::json!(exp_years),
    );

    RAGDocument {
        id: Uuid::new_v4().to_string(),
        entity_type: "candidate".to_string(),
        entity_id: row.get("candidate_id").cloned().unwrap_or_default(),
        title: name,
        content: content_parts.join("\n"),
        metadata,
    }
}

/// Create RAG documents from all jobs and candidates in database.
pub fn create_rag_documents(db: &Database) -> Result<Vec<RAGDocument>> {
    let mut documents = Vec::new();

    // Get all jobs
    let jobs = db.query("SELECT * FROM jobs", &[])?;
    for row in &jobs {
        documents.push(prepare_job_for_embedding(row));
    }

    // Get all candidates
    let candidates = db.query("SELECT * FROM candidates", &[])?;
    for row in &candidates {
        documents.push(prepare_candidate_for_embedding(row));
    }

    // Add skill documents
    let skills = get_skills_for_rag(db, 1, true)?;
    for skill_data in &skills {
        let skill_name = skill_data
            .get("skill_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let rag_text = skill_data
            .get("rag_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut metadata = HashMap::new();
        if let Some(cat) = skill_data.get("category") {
            metadata.insert("category".to_string(), cat.clone());
        }
        if let Some(freq) = skill_data.get("frequency") {
            metadata.insert("frequency".to_string(), freq.clone());
        }

        documents.push(RAGDocument {
            id: Uuid::new_v4().to_string(),
            entity_type: "skill".to_string(),
            entity_id: skill_name.to_string(),
            title: skill_name.to_string(),
            content: rag_text.to_string(),
            metadata,
        });
    }

    Ok(documents)
}

/// Store an embedding vector in the database.
pub fn store_embedding(
    db: &Database,
    entity_type: &str,
    entity_id: &str,
    embedding: &[f64],
    content_type: &str,
    model: &str,
) -> Result<()> {
    let embedding_id = Uuid::new_v4().to_string();
    let embedding_json = serde_json::to_string(embedding)?;
    let content_hash = hex::encode(Sha256::digest(
        format!("{}{}", entity_id, content_type).as_bytes(),
    ));
    let now = Local::now().to_string();

    db.execute(
        "INSERT INTO embeddings (embedding_id, entity_type, entity_id, content_type, content_hash, embedding_vector, embedding_model)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT (entity_type, entity_id, content_type) DO UPDATE SET
            embedding_vector = EXCLUDED.embedding_vector,
            embedding_model = EXCLUDED.embedding_model,
            created_at = ?",
        &[
            &embedding_id.as_str(),
            &entity_type,
            &entity_id,
            &content_type,
            &content_hash.as_str(),
            &embedding_json.as_str(),
            &model,
            &now.as_str(),
        ],
    )?;

    Ok(())
}

/// Export all data in JSONL format suitable for external RAG systems.
pub fn export_for_rag(db: &Database, filepath: &str) -> Result<usize> {
    let documents = create_rag_documents(db)?;
    log::info!("Exporting {} documents for RAG...", documents.len());

    let file = std::fs::File::create(filepath)?;
    let mut writer = std::io::BufWriter::new(file);

    use std::io::Write;
    for doc in &documents {
        let doc_json = serde_json::json!({
            "id": doc.id,
            "entity_type": doc.entity_type,
            "entity_id": doc.entity_id,
            "title": doc.title,
            "content": doc.content,
            "metadata": doc.metadata,
        });
        writeln!(writer, "{}", serde_json::to_string(&doc_json)?)?;
    }

    log::info!("Exported to {}", filepath);
    Ok(documents.len())
}

/// Export skills taxonomy with relationships.
pub fn export_skills_taxonomy(db: &Database, filepath: &str) -> Result<()> {
    let skills = db.query(
        "SELECT s.*, COUNT(DISTINCT js.job_id) as job_count, COUNT(DISTINCT cs.candidate_id) as candidate_count
         FROM skills_kb s
         LEFT JOIN job_skills js ON s.canonical_name = js.skill_name
         LEFT JOIN candidate_skills cs ON s.canonical_name = cs.skill_name
         GROUP BY s.skill_id, s.skill_name, s.canonical_name, s.category,
                  s.frequency, s.first_seen_at, s.last_seen_at, s.source_type,
                  s.related_skills, s.description
         ORDER BY s.frequency DESC",
        &[],
    )?;

    let cooc = db.query(
        "SELECT skill_a, skill_b, cooccurrence_count FROM skill_cooccurrence WHERE cooccurrence_count >= 2 ORDER BY cooccurrence_count DESC",
        &[],
    )?;

    let synonyms = db.query("SELECT * FROM skill_synonyms", &[])?;

    let taxonomy = serde_json::json!({
        "skills": skills,
        "cooccurrence": cooc.iter().map(|row| {
            serde_json::json!({
                "skill_a": row.get("skill_a").cloned().unwrap_or_default(),
                "skill_b": row.get("skill_b").cloned().unwrap_or_default(),
                "count": row.get("cooccurrence_count").and_then(|c| c.parse::<i64>().ok()).unwrap_or(0),
            })
        }).collect::<Vec<_>>(),
        "synonyms": synonyms.iter().map(|row| {
            serde_json::json!({
                "synonym": row.get("skill_name").cloned().unwrap_or_default(),
                "canonical": row.get("canonical_name").cloned().unwrap_or_default(),
            })
        }).collect::<Vec<_>>(),
        "statistics": {
            "total_skills": skills.len(),
            "total_relationships": cooc.len(),
            "total_synonyms": synonyms.len(),
        },
        "exported_at": Local::now().to_string(),
    });

    let file = std::fs::File::create(filepath)?;
    serde_json::to_writer_pretty(file, &taxonomy)?;

    log::info!("Skills taxonomy exported to {}", filepath);
    Ok(())
}

/// Get relevant context from the knowledge base for RAG.
pub fn get_rag_context(
    db: &Database,
    query_skills: &[String],
    max_results: usize,
) -> Result<Vec<HashMap<String, serde_json::Value>>> {
    let mut context = Vec::new();

    for skill in query_skills {
        let related = get_related_skills(db, skill, 5)?;
        let mut skill_info = HashMap::new();
        skill_info.insert(
            "skill".to_string(),
            serde_json::Value::String(skill.clone()),
        );
        skill_info.insert("related".to_string(), serde_json::json!(related));
        context.push(skill_info);
    }

    // Get jobs matching these skills
    let skills_pattern = format!("%{}%", query_skills.join("%|%"));
    let max_results_i64 = max_results as i64;
    let jobs = db.query(
        "SELECT job_id, title, location, skills_raw FROM jobs WHERE LOWER(skills_raw) LIKE LOWER(?) LIMIT ?",
        &[&skills_pattern.as_str(), &max_results_i64],
    )?;

    for row in &jobs {
        let mut job_info = HashMap::new();
        job_info.insert(
            "type".to_string(),
            serde_json::Value::String("job".to_string()),
        );
        job_info.insert(
            "job_id".to_string(),
            serde_json::Value::String(row.get("job_id").cloned().unwrap_or_default()),
        );
        job_info.insert(
            "title".to_string(),
            serde_json::Value::String(row.get("title").cloned().unwrap_or_default()),
        );
        job_info.insert(
            "skills".to_string(),
            serde_json::Value::String(row.get("skills_raw").cloned().unwrap_or_default()),
        );
        context.push(job_info);
    }

    Ok(context)
}
