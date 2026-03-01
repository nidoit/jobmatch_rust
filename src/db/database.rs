use anyhow::Result;
use duckdb::Connection;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::config;

/// Thread-safe database connection wrapper
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Initialize database with schema.
    pub fn new(db_path: Option<&Path>) -> Result<Self> {
        let path = db_path
            .map(PathBuf::from)
            .unwrap_or_else(config::db_path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        db.create_schema()?;
        log::info!("Database initialized at {:?}", db.path);

        Ok(db)
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        };
        db.create_schema()?;
        Ok(db)
    }

    /// Get a reference to the connection
    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }

    /// Execute a statement (INSERT, UPDATE, DELETE)
    pub fn execute(&self, sql: &str, params: &[&dyn duckdb::ToSql]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(sql, params)?;
        Ok(())
    }

    /// Execute a query and return rows as Vec<HashMap>
    pub fn query(
        &self,
        sql: &str,
        params: &[&dyn duckdb::ToSql],
    ) -> Result<Vec<HashMap<String, String>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).map_or(String::new(), |v| v.to_string()))
            .collect();

        let mut rows = Vec::new();
        let mut result_rows = stmt.query(params)?;
        while let Some(row) = result_rows.next()? {
            let mut map = HashMap::new();
            for (i, name) in column_names.iter().enumerate() {
                let val: String = row.get::<_, String>(i).unwrap_or_default();
                map.insert(name.clone(), val);
            }
            rows.push(map);
        }

        Ok(rows)
    }

    /// Query returning a single count value
    pub fn query_count(&self, sql: &str, params: &[&dyn duckdb::ToSql]) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        if let Some(row) = rows.next()? {
            Ok(row.get::<_, i64>(0).unwrap_or(0))
        } else {
            Ok(0)
        }
    }

    /// Get table row count
    pub fn get_table_count(&self, table_name: &str) -> Result<i64> {
        self.query_count(&format!("SELECT COUNT(*) FROM {}", table_name), &[])
    }

    /// Get database statistics
    pub fn get_database_stats(&self) -> Result<HashMap<String, i64>> {
        let tables = [
            "jobs",
            "candidates",
            "skills_kb",
            "skill_synonyms",
            "job_skills",
            "candidate_skills",
            "matches",
            "skill_cooccurrence",
        ];
        let mut stats = HashMap::new();
        for table in tables {
            stats.insert(table.to_string(), self.get_table_count(table)?);
        }
        Ok(stats)
    }

    /// Create all tables if they don't exist.
    fn create_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Jobs table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                job_id VARCHAR PRIMARY KEY,
                title VARCHAR,
                buyer VARCHAR,
                location VARCHAR,
                status VARCHAR,
                description TEXT,
                requirements TEXT,
                responsibilities TEXT,
                skills_raw TEXT,
                experience_level VARCHAR,
                start_date VARCHAR,
                end_date VARCHAR,
                respond_by VARCHAR,
                positions INTEGER DEFAULT 1,
                business_unit VARCHAR,
                contingent_type VARCHAR,
                buyer_reference VARCHAR,
                detail_url VARCHAR,
                source_file VARCHAR,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        // Candidates table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS candidates (
                candidate_id VARCHAR PRIMARY KEY,
                name VARCHAR,
                email VARCHAR,
                phone VARCHAR,
                skills_raw TEXT,
                experience_years INTEGER DEFAULT 0,
                education TEXT,
                current_title VARCHAR,
                location VARCHAR,
                summary TEXT,
                languages VARCHAR,
                resume_path VARCHAR,
                resume_text TEXT,
                resume_hash VARCHAR,
                source_file VARCHAR,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        // Skills Knowledge Base
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skills_kb (
                skill_id VARCHAR PRIMARY KEY,
                skill_name VARCHAR NOT NULL,
                canonical_name VARCHAR NOT NULL,
                category VARCHAR,
                frequency INTEGER DEFAULT 1,
                first_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                last_seen_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                source_type VARCHAR,
                related_skills TEXT,
                description TEXT
            )",
        )?;

        // Skill synonyms
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_synonyms (
                synonym_id VARCHAR PRIMARY KEY,
                skill_name VARCHAR NOT NULL,
                canonical_name VARCHAR NOT NULL,
                confidence FLOAT DEFAULT 1.0,
                source VARCHAR,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )?;

        // Job-Skill associations
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS job_skills (
                id VARCHAR PRIMARY KEY,
                job_id VARCHAR NOT NULL,
                skill_name VARCHAR NOT NULL,
                is_required BOOLEAN DEFAULT TRUE,
                extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(job_id, skill_name)
            )",
        )?;

        // Candidate-Skill associations
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS candidate_skills (
                id VARCHAR PRIMARY KEY,
                candidate_id VARCHAR NOT NULL,
                skill_name VARCHAR NOT NULL,
                proficiency_level VARCHAR,
                years_experience INTEGER,
                extracted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(candidate_id, skill_name)
            )",
        )?;

        // Match results
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS matches (
                match_id VARCHAR PRIMARY KEY,
                candidate_id VARCHAR NOT NULL,
                job_id VARCHAR NOT NULL,
                overall_score FLOAT,
                skill_score FLOAT,
                experience_score FLOAT,
                location_score FLOAT,
                education_score FLOAT,
                matched_skills TEXT,
                missing_skills TEXT,
                matched_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(candidate_id, job_id)
            )",
        )?;

        // RAG embeddings
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS embeddings (
                embedding_id VARCHAR PRIMARY KEY,
                entity_type VARCHAR NOT NULL,
                entity_id VARCHAR NOT NULL,
                content_type VARCHAR,
                content_hash VARCHAR,
                embedding_vector TEXT,
                embedding_model VARCHAR,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(entity_type, entity_id, content_type)
            )",
        )?;

        // Skill co-occurrence matrix
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_cooccurrence (
                id VARCHAR PRIMARY KEY,
                skill_a VARCHAR NOT NULL,
                skill_b VARCHAR NOT NULL,
                cooccurrence_count INTEGER DEFAULT 1,
                source_type VARCHAR,
                last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(skill_a, skill_b)
            )",
        )?;

        // Ingestion log
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ingestion_log (
                log_id VARCHAR PRIMARY KEY,
                file_path VARCHAR NOT NULL,
                file_hash VARCHAR,
                file_type VARCHAR,
                records_processed INTEGER DEFAULT 0,
                skills_extracted INTEGER DEFAULT 0,
                status VARCHAR DEFAULT 'pending',
                error_message TEXT,
                started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                completed_at TIMESTAMP,
                UNIQUE(file_hash)
            )",
        )?;

        // Create indexes
        conn.execute_batch("CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)")?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_skills_kb_canonical ON skills_kb(canonical_name)",
        )?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_job_skills_job ON job_skills(job_id)",
        )?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_candidate_skills_cand ON candidate_skills(candidate_id)",
        )?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_matches_score ON matches(overall_score DESC)",
        )?;

        log::info!("Database schema created/verified");
        Ok(())
    }
}
