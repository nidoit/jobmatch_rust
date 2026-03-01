use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a job posting from SAP Fieldglass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_id: String,
    pub title: String,
    pub buyer: String,
    pub location: String,
    pub status: String,
    pub description: String,
    pub requirements: String,
    pub responsibilities: String,
    pub skills: String,
    pub experience_level: String,
    pub start_date: String,
    pub end_date: String,
    pub respond_by: String,
    pub positions: i32,
    pub business_unit: String,
    pub scraped_at: String,
    pub detail_url: String,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            job_id: String::new(),
            title: String::new(),
            buyer: String::new(),
            location: String::new(),
            status: String::new(),
            description: String::new(),
            requirements: String::new(),
            responsibilities: String::new(),
            skills: String::new(),
            experience_level: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            respond_by: String::new(),
            positions: 1,
            business_unit: String::new(),
            scraped_at: Local::now().to_string(),
            detail_url: String::new(),
        }
    }
}

impl Job {
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

    /// Extract experience level from job title.
    /// E.g., "SE_Mechanical Engineer_Senior" -> "Senior"
    pub fn extract_experience_level(title: &str) -> String {
        let levels = [
            "Entry",
            "Professional",
            "Experienced",
            "Senior",
            "Specialist",
            "Expert",
            "Senior Expert",
            "Lead",
            "Chief",
        ];

        let title_lower = title.to_lowercase();
        for level in levels.iter().rev() {
            if title_lower.contains(&level.to_lowercase()) {
                return level.to_string();
            }
        }
        String::new()
    }

    /// Convert Job to dictionary/map.
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("job_id".into(), self.job_id.clone());
        m.insert("title".into(), self.title.clone());
        m.insert("buyer".into(), self.buyer.clone());
        m.insert("location".into(), self.location.clone());
        m.insert("status".into(), self.status.clone());
        m.insert("description".into(), self.description.clone());
        m.insert("requirements".into(), self.requirements.clone());
        m.insert("responsibilities".into(), self.responsibilities.clone());
        m.insert("skills".into(), self.skills.clone());
        m.insert("experience_level".into(), self.experience_level.clone());
        m.insert("start_date".into(), self.start_date.clone());
        m.insert("end_date".into(), self.end_date.clone());
        m.insert("respond_by".into(), self.respond_by.clone());
        m.insert("positions".into(), self.positions.to_string());
        m.insert("business_unit".into(), self.business_unit.clone());
        m.insert("scraped_at".into(), self.scraped_at.clone());
        m.insert("detail_url".into(), self.detail_url.clone());
        m
    }
}
