use crate::config::SKILL_SYNONYMS;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// Technical skills dictionary
pub static TECH_SKILLS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // Programming Languages
        "python", "java", "javascript", "typescript", "c", "cpp", "c++", "csharp", "c#",
        "ruby", "go", "golang", "rust", "swift", "kotlin", "scala", "php", "perl",
        "r", "matlab", "julia", "lua", "haskell", "erlang", "elixir", "clojure",
        "assembly", "vhdl", "verilog", "fortran", "cobol", "pascal", "delphi",
        // Web Technologies
        "html", "css", "sass", "scss", "less", "react", "reactjs",
        "angular", "vue", "vuejs", "svelte", "nextjs", "nuxtjs", "gatsby",
        "nodejs", "express", "fastify", "nestjs", "django", "flask", "fastapi",
        "rails", "spring", "springboot", "asp.net", "laravel", "symfony",
        // Databases
        "sql", "mysql", "postgresql", "postgres", "mongodb", "redis", "elasticsearch",
        "cassandra", "dynamodb", "sqlite", "oracle", "mariadb", "mssql", "neo4j",
        "influxdb", "timescaledb", "cockroachdb", "firebase",
        // Cloud & DevOps
        "aws", "azure", "gcp", "google cloud", "docker", "kubernetes", "k8s",
        "terraform", "ansible", "jenkins", "gitlab ci", "github actions", "circleci",
        "prometheus", "grafana", "datadog", "splunk", "elk", "nginx", "apache",
        // Data & ML
        "machine learning", "deep learning", "tensorflow", "pytorch", "keras",
        "scikit-learn", "pandas", "numpy", "scipy", "spark", "hadoop", "kafka",
        "airflow", "mlflow", "databricks", "snowflake", "bigquery", "tableau",
        "power bi", "looker", "nlp", "computer vision", "neural networks",
        // Embedded & Hardware
        "embedded systems", "embedded c", "microcontrollers", "arm", "rtos",
        "firmware", "fpga", "pcb", "can bus", "autosar", "simulink",
        "labview", "plc", "scada", "modbus", "spi", "i2c", "uart",
        // CAD & Engineering
        "cad", "catia", "solidworks", "autocad", "nx", "creo", "inventor",
        "ansys", "abaqus", "comsol", "fea", "cfd", "gd&t",
        // Version Control & Tools
        "git", "github", "gitlab", "bitbucket", "svn", "mercurial",
        "jira", "confluence", "trello", "asana", "slack", "teams",
        // Testing
        "unit testing", "integration testing", "selenium", "cypress", "jest",
        "pytest", "junit", "testng", "cucumber", "postman", "soapui",
        // Mobile
        "ios", "android", "react native", "flutter", "xamarin", "cordova",
        "objective-c",
        // Security
        "cybersecurity", "penetration testing", "owasp", "ssl", "tls", "oauth",
        "jwt", "encryption", "firewall", "vpn", "siem",
        // Methodologies
        "agile", "scrum", "kanban", "waterfall", "devops", "cicd", "ci/cd",
        "tdd", "bdd", "pair programming", "code review",
    ]
    .into_iter()
    .collect()
});

/// Soft skills dictionary
pub static SOFT_SKILLS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "communication", "teamwork", "leadership", "problem solving",
        "critical thinking", "time management", "project management",
        "analytical", "creativity", "adaptability", "collaboration",
        "presentation", "negotiation", "decision making", "attention to detail",
        "self-motivated", "initiative", "interpersonal", "organization",
        "multitasking", "flexibility", "customer service", "mentoring",
    ]
    .into_iter()
    .collect()
});

/// Normalize a skill string using synonym mapping.
pub fn normalize_skill(skill: &str) -> String {
    let skill = skill.trim().to_lowercase();
    if let Some(&canonical) = SKILL_SYNONYMS.get(skill.as_str()) {
        canonical.to_string()
    } else {
        skill
    }
}

/// Escape special regex characters in a string.
fn escape_regex(s: &str) -> String {
    let special = r"\.[]{}()*+?^$|";
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special.contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

/// Extract skills from text using dictionary matching and NLP.
pub fn extract_skills_from_text(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut found_skills: HashSet<String> = HashSet::new();

    // Match against technical skills dictionary
    for &skill in TECH_SKILLS.iter() {
        let pattern = format!(r"(?i)\b{}\b", escape_regex(skill));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&text_lower) {
                found_skills.insert(normalize_skill(skill));
            }
        }
    }

    // Match against soft skills
    for &skill in SOFT_SKILLS.iter() {
        let pattern = format!(r"(?i)\b{}\b", escape_regex(skill));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&text_lower) {
                found_skills.insert(normalize_skill(skill));
            }
        }
    }

    // Also check for skill synonyms
    for (&synonym, &canonical) in SKILL_SYNONYMS.iter() {
        let pattern = format!(r"(?i)\b{}\b", escape_regex(synonym));
        if let Ok(re) = Regex::new(&pattern) {
            if re.is_match(&text_lower) {
                found_skills.insert(canonical.to_string());
            }
        }
    }

    found_skills.into_iter().collect()
}

/// Get all synonyms for a given skill.
pub fn get_skill_synonyms(skill: &str) -> Vec<String> {
    let normalized = normalize_skill(skill);
    let mut synonyms: Vec<String> = vec![skill.to_string(), normalized.clone()];

    // Find all keys that map to this canonical name
    for (&syn, &canonical) in SKILL_SYNONYMS.iter() {
        if canonical == normalized {
            synonyms.push(syn.to_string());
        }
    }

    synonyms.sort();
    synonyms.dedup();
    synonyms
}

/// Match tokens against a skill dictionary.
pub fn match_against_dictionary(tokens: &[String], dictionary: &HashSet<&str>) -> Vec<String> {
    let mut matched = Vec::new();

    for token in tokens {
        let normalized = normalize_skill(token);
        if dictionary.contains(normalized.as_str()) {
            matched.push(normalized);
        }
    }

    // Also check bigrams
    if tokens.len() >= 2 {
        for i in 0..tokens.len() - 1 {
            let bigram = format!("{} {}", tokens[i], tokens[i + 1]);
            let normalized = normalize_skill(&bigram);
            if dictionary.contains(normalized.as_str()) {
                matched.push(normalized);
            }
        }
    }

    // Also check trigrams
    if tokens.len() >= 3 {
        for i in 0..tokens.len() - 2 {
            let trigram = format!("{} {} {}", tokens[i], tokens[i + 1], tokens[i + 2]);
            let normalized = normalize_skill(&trigram);
            if dictionary.contains(normalized.as_str()) {
                matched.push(normalized);
            }
        }
    }

    matched.sort();
    matched.dedup();
    matched
}

/// Categorize skills into technical and soft skills.
pub fn categorize_skills(skills: &[String]) -> HashMap<String, Vec<String>> {
    let mut tech = Vec::new();
    let mut soft = Vec::new();
    let mut other = Vec::new();

    for skill in skills {
        let normalized = normalize_skill(skill);
        if TECH_SKILLS.contains(normalized.as_str()) {
            tech.push(normalized);
        } else if SOFT_SKILLS.contains(normalized.as_str()) {
            soft.push(normalized);
        } else {
            other.push(skill.clone());
        }
    }

    tech.sort();
    tech.dedup();
    soft.sort();
    soft.dedup();
    other.sort();
    other.dedup();

    let mut result = HashMap::new();
    result.insert("technical".to_string(), tech);
    result.insert("soft".to_string(), soft);
    result.insert("other".to_string(), other);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_skill() {
        assert_eq!(normalize_skill("JS"), "javascript");
        assert_eq!(normalize_skill("Python"), "python");
        assert_eq!(normalize_skill("k8s"), "kubernetes");
    }

    #[test]
    fn test_extract_skills_from_text() {
        let text =
            "Experience with Python, JavaScript, and AWS. Knowledge of Docker and Kubernetes.";
        let skills = extract_skills_from_text(text);
        assert!(skills.contains(&"python".to_string()));
        assert!(skills.contains(&"javascript".to_string()));
        assert!(skills.contains(&"aws".to_string()));
        assert!(skills.contains(&"docker".to_string()));
    }

    #[test]
    fn test_get_skill_synonyms() {
        let synonyms = get_skill_synonyms("javascript");
        assert!(synonyms.contains(&"js".to_string()));
        assert!(synonyms.contains(&"javascript".to_string()));
    }
}
