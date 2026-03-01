use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// Extract all text content from a PDF file.
pub fn parse_pdf(filepath: &str) -> String {
    if !Path::new(filepath).is_file() {
        log::warn!("PDF file not found: {}", filepath);
        return String::new();
    }

    match pdf_extract::extract_text(filepath) {
        Ok(text) => text.trim().to_string(),
        Err(e) => {
            log::error!("Error parsing PDF {}: {}", filepath, e);
            String::new()
        }
    }
}

/// Parse a resume PDF and extract structured information.
pub fn parse_resume(filepath: &str) -> HashMap<String, String> {
    let text = parse_pdf(filepath);

    if text.is_empty() {
        return HashMap::new();
    }

    let sections = extract_sections(&text);
    let (email, phone, location) = extract_contact_info(&text);
    let name = extract_name(&text);

    let mut result = HashMap::new();
    result.insert("name".to_string(), name);
    result.insert("email".to_string(), email);
    result.insert("phone".to_string(), phone);
    result.insert("location".to_string(), location);
    result.insert("raw_text".to_string(), text);

    // Add sections
    for (key, value) in &sections {
        result.insert(format!("{}_section", key), value.clone());
    }

    result.insert(
        "skills_section".to_string(),
        sections.get("skills").cloned().unwrap_or_default(),
    );
    result.insert(
        "experience_section".to_string(),
        sections.get("experience").cloned().unwrap_or_default(),
    );
    result.insert(
        "education_section".to_string(),
        sections.get("education").cloned().unwrap_or_default(),
    );

    result
}

/// Identify and extract common resume sections.
pub fn extract_sections(text: &str) -> HashMap<String, String> {
    let section_patterns: Vec<(&str, Regex)> = vec![
        (
            "skills",
            Regex::new(r"(?i)(skills|technical skills|core competencies|technologies|tools)\s*:?\s*\n").unwrap(),
        ),
        (
            "experience",
            Regex::new(r"(?i)(experience|work experience|employment|professional experience)\s*:?\s*\n").unwrap(),
        ),
        (
            "education",
            Regex::new(r"(?i)(education|academic|qualifications|degrees)\s*:?\s*\n").unwrap(),
        ),
        (
            "summary",
            Regex::new(r"(?i)(summary|profile|objective|about)\s*:?\s*\n").unwrap(),
        ),
        (
            "projects",
            Regex::new(r"(?i)(projects|portfolio)\s*:?\s*\n").unwrap(),
        ),
        (
            "certifications",
            Regex::new(r"(?i)(certifications|certificates|licenses)\s*:?\s*\n").unwrap(),
        ),
        (
            "languages",
            Regex::new(r"(?i)(languages|language skills)\s*:?\s*\n").unwrap(),
        ),
    ];

    let lines: Vec<&str> = text.lines().collect();
    let mut current_section = "header".to_string();
    let mut section_content: HashMap<String, Vec<String>> = HashMap::new();
    section_content.insert("header".to_string(), Vec::new());

    for line in &lines {
        let line_with_newline = format!("{}\n", line);
        let mut found_section = false;

        for (name, pattern) in &section_patterns {
            if pattern.is_match(&line_with_newline) {
                current_section = name.to_string();
                section_content
                    .entry(name.to_string())
                    .or_insert_with(Vec::new);
                found_section = true;
                break;
            }
        }

        if !found_section && !line.trim().is_empty() {
            section_content
                .entry(current_section.clone())
                .or_insert_with(Vec::new)
                .push(line.to_string());
        }
    }

    let mut sections = HashMap::new();
    for (name, lines) in section_content {
        sections.insert(name, lines.join("\n"));
    }
    sections
}

/// Extract email, phone, and location from resume text.
pub fn extract_contact_info(text: &str) -> (String, String, String) {
    // Email
    let email = Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}")
        .ok()
        .and_then(|re| re.find(text).map(|m| m.as_str().to_string()))
        .unwrap_or_default();

    // Phone
    let phone = Regex::new(
        r"[+]?[(]?[0-9]{1,3}[)]?[-\s.]?[0-9]{2,4}[-\s.]?[0-9]{2,4}[-\s.]?[0-9]{2,6}",
    )
    .ok()
    .and_then(|re| re.find(text).map(|m| m.as_str().to_string()))
    .unwrap_or_default();

    // Location
    let swedish_cities = [
        "gothenburg", "göteborg", "stockholm", "malmö", "malmo", "uppsala", "linköping", "lund",
        "umeå", "umea", "västerås",
    ];
    let text_lower = text.to_lowercase();
    let location = swedish_cities
        .iter()
        .find(|city| text_lower.contains(*city))
        .map(|city| titlecase(city))
        .unwrap_or_default();

    (email, phone, location)
}

/// Extract candidate name from resume (usually at the top).
pub fn extract_name(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let re_resume = Regex::new(r"(?i)(resume|cv|curriculum vitae)").unwrap();
    let re_email_phone = Regex::new(r"@|[0-9]{6,}").unwrap();
    let re_name_word = Regex::new(r"^[A-Za-zÀ-ÿ\-']+$").unwrap();

    for line in lines.iter().take(5) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if re_resume.is_match(line) {
            continue;
        }
        if re_email_phone.is_match(line) {
            continue;
        }

        let words: Vec<&str> = line.split_whitespace().collect();
        if words.len() >= 2 && words.len() <= 4 && words.iter().all(|w| re_name_word.is_match(w)) {
            return line.to_string();
        }
    }

    String::new()
}

/// Parse all PDF files in a directory.
pub fn parse_all_resumes(directory: &str) -> Vec<HashMap<String, String>> {
    let path = Path::new(directory);
    if !path.is_dir() {
        log::warn!("Directory not found: {}", directory);
        return Vec::new();
    }

    let mut results = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if let Some(ext) = file_path.extension() {
                if ext.to_str().map(|s| s.to_lowercase()) == Some("pdf".to_string()) {
                    let filepath = file_path.to_string_lossy().to_string();
                    log::info!("Parsing: {}", filepath);

                    let mut data = parse_resume(&filepath);
                    if !data.is_empty() {
                        data.insert("resume_path".to_string(), filepath);
                        data.insert(
                            "filename".to_string(),
                            entry.file_name().to_string_lossy().to_string(),
                        );
                        results.push(data);
                    }
                }
            }
        }
    }

    log::info!("Parsed {} resumes", results.len());
    results
}

/// Simple titlecase helper
fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
    }
}
