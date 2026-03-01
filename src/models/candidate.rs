use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents a job candidate with parsed resume data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub candidate_id: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub skills: String,
    pub experience_years: i32,
    pub education: String,
    pub current_title: String,
    pub location: String,
    pub summary: String,
    pub languages: String,
    pub resume_path: String,
    pub resume_text: String,
    pub parsed_at: String,
    pub updated_at: String,
}

impl Default for Candidate {
    fn default() -> Self {
        let now = Local::now().to_string();
        Self {
            candidate_id: Uuid::new_v4().to_string(),
            name: String::new(),
            email: String::new(),
            phone: String::new(),
            skills: String::new(),
            experience_years: 0,
            education: String::new(),
            current_title: String::new(),
            location: String::new(),
            summary: String::new(),
            languages: String::new(),
            resume_path: String::new(),
            resume_text: String::new(),
            parsed_at: now.clone(),
            updated_at: now,
        }
    }
}

impl Candidate {
    /// Parse skills string into list of individual skills.
    pub fn get_skills_list(&self) -> Vec<String> {
        if self.skills.is_empty() {
            return Vec::new();
        }
        self.skills
            .split(&[',', ';', '|'][..])
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Convert Candidate to dictionary/map.
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("candidate_id".into(), self.candidate_id.clone());
        m.insert("name".into(), self.name.clone());
        m.insert("email".into(), self.email.clone());
        m.insert("phone".into(), self.phone.clone());
        m.insert("skills".into(), self.skills.clone());
        m.insert("experience_years".into(), self.experience_years.to_string());
        m.insert("education".into(), self.education.clone());
        m.insert("current_title".into(), self.current_title.clone());
        m.insert("location".into(), self.location.clone());
        m.insert("summary".into(), self.summary.clone());
        m.insert("languages".into(), self.languages.clone());
        m.insert("resume_path".into(), self.resume_path.clone());
        m.insert("parsed_at".into(), self.parsed_at.clone());
        m.insert("updated_at".into(), self.updated_at.clone());
        m
    }
}
