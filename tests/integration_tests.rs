use jobmatch::matching::matcher::*;
use jobmatch::matching::scorer::*;
use jobmatch::matching::skill_matcher::*;
use jobmatch::models::candidate::Candidate;
use jobmatch::models::job::Job;
use jobmatch::nlp::skill_extractor::*;
use jobmatch::nlp::text_processor::*;

#[test]
fn test_preprocess_text() {
    let result = preprocess_text("Hello WORLD! This is a TEST.");
    assert_eq!(result, "hello world this is a test.");
}

#[test]
fn test_tokenize() {
    let tokens = tokenize("Python Java SQL");
    assert_eq!(tokens.len(), 3);
    assert!(tokens.contains(&"python".to_string()));
}

#[test]
fn test_remove_stopwords() {
    let tokens: Vec<String> = vec!["the", "python", "is", "great"]
        .into_iter()
        .map(String::from)
        .collect();
    let filtered = remove_stopwords(&tokens);
    assert!(!filtered.contains(&"the".to_string()));
    assert!(filtered.contains(&"python".to_string()));
}

#[test]
fn test_normalize_text() {
    let result = normalize_text("The quick Python programmer");
    assert!(result.contains(&"python".to_string()));
    assert!(!result.contains(&"the".to_string()));
}

#[test]
fn test_normalize_skill() {
    assert_eq!(normalize_skill("JS"), "javascript");
    assert_eq!(normalize_skill("Python"), "python");
    assert_eq!(normalize_skill("k8s"), "kubernetes");
}

#[test]
fn test_extract_skills_from_text() {
    let text = "Experience with Python, JavaScript, and AWS. Knowledge of Docker and Kubernetes.";
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

#[test]
fn test_skill_similarity_exact() {
    assert_eq!(compute_skill_similarity("python", "python"), 1.0);
}

#[test]
fn test_skill_similarity_synonym() {
    assert!(compute_skill_similarity("js", "javascript") >= 0.9);
}

#[test]
fn test_skill_similarity_similar() {
    let sim = compute_skill_similarity("javascript", "typescript");
    assert!(sim > 0.5 && sim < 1.0);
}

#[test]
fn test_skill_similarity_different() {
    let sim = compute_skill_similarity("python", "kubernetes");
    assert!(sim < 0.5);
}

#[test]
fn test_find_matching_skills() {
    let candidate: Vec<String> = vec!["python", "java", "sql"]
        .into_iter()
        .map(String::from)
        .collect();
    let job: Vec<String> = vec!["python", "javascript", "sql"]
        .into_iter()
        .map(String::from)
        .collect();

    let matches = find_matching_skills(&candidate, &job, 0.7);
    let matched_skills: Vec<&String> = matches.iter().map(|m| &m.0).collect();

    assert!(matched_skills.contains(&&"python".to_string()));
    assert!(matched_skills.contains(&&"sql".to_string()));
}

#[test]
fn test_find_missing_skills() {
    let candidate: Vec<String> = vec!["python", "sql"].into_iter().map(String::from).collect();
    let job: Vec<String> = vec!["python", "javascript", "sql", "aws"]
        .into_iter()
        .map(String::from)
        .collect();

    let missing = find_missing_skills(&candidate, &job, 0.7);
    assert!(missing.contains(&"javascript".to_string()));
    assert!(missing.contains(&"aws".to_string()));
    assert!(!missing.contains(&"python".to_string()));
}

#[test]
fn test_skill_match_score_perfect() {
    let score = compute_skill_match_score(
        &["python".into(), "sql".into()],
        &["python".into(), "sql".into()],
        0.7,
    );
    assert!(score >= 0.9);
}

#[test]
fn test_skill_match_score_partial() {
    let score = compute_skill_match_score(
        &["python".into()],
        &["python".into(), "java".into(), "sql".into()],
        0.7,
    );
    assert!(score > 0.2 && score < 0.8);
}

#[test]
fn test_skill_match_score_none() {
    let score = compute_skill_match_score(
        &["rust".into(), "go".into()],
        &["python".into(), "java".into()],
        0.7,
    );
    assert!(score < 0.3);
}

#[test]
fn test_location_same_city() {
    assert_eq!(compute_location_score("Gothenburg", "Gothenburg"), 1.0);
}

#[test]
fn test_location_swedish_cities() {
    let score = compute_location_score("Gothenburg", "Stockholm");
    assert!(score >= 0.6);
}

#[test]
fn test_location_empty() {
    assert_eq!(compute_location_score("", "Gothenburg"), 0.5);
}

#[test]
fn test_experience_perfect() {
    assert_eq!(compute_experience_score(7, "Senior"), 1.0);
}

#[test]
fn test_experience_under() {
    let score = compute_experience_score(2, "Senior");
    assert!(score < 0.7);
}

#[test]
fn test_experience_over() {
    let score = compute_experience_score(15, "Senior");
    assert!(score > 0.5 && score < 1.0);
}

#[test]
fn test_experience_from_title() {
    let score = compute_experience_score_from_title("SE_Mechanical Engineer_Senior", 7);
    assert!(score >= 0.8);
}

#[test]
fn test_experience_entry() {
    let score = compute_experience_score_from_title("SE_Developer_Entry", 1);
    assert!(score >= 0.8);
}

#[test]
fn test_overall_score_perfect() {
    let score = compute_overall_score(1.0, 1.0, 1.0, 1.0);
    assert_eq!(score, 100.0);
}

#[test]
fn test_overall_score_mixed() {
    let score = compute_overall_score(0.8, 0.7, 0.6, 0.5);
    assert!(score > 60.0 && score < 80.0);
}

#[test]
fn test_job_creation() {
    let job = Job {
        job_id: "TEST001".to_string(),
        title: "Software Engineer".to_string(),
        buyer: "Test Company".to_string(),
        location: "Gothenburg".to_string(),
        ..Default::default()
    };
    assert_eq!(job.job_id, "TEST001");
    assert_eq!(job.title, "Software Engineer");
}

#[test]
fn test_job_get_skills_list() {
    let job = Job {
        job_id: "TEST".to_string(),
        title: "Test".to_string(),
        skills: "Python, Java, SQL".to_string(),
        ..Default::default()
    };
    let skills = job.get_skills_list();
    assert_eq!(skills.len(), 3);
    assert!(skills.contains(&"python".to_string()));
}

#[test]
fn test_extract_experience_level() {
    let level = Job::extract_experience_level("SE_Mechanical Engineer_Senior");
    assert_eq!(level, "Senior");

    let level = Job::extract_experience_level("SE_Developer_Expert");
    assert_eq!(level, "Expert");
}

#[test]
fn test_candidate_creation() {
    let candidate = Candidate {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        skills: "Python, Java".to_string(),
        ..Default::default()
    };
    assert_eq!(candidate.name, "John Doe");
    assert!(!candidate.candidate_id.is_empty());
}

#[test]
fn test_candidate_get_skills_list() {
    let candidate = Candidate {
        name: "Test".to_string(),
        skills: "Python, Java, SQL".to_string(),
        ..Default::default()
    };
    let skills = candidate.get_skills_list();
    assert_eq!(skills.len(), 3);
}

#[test]
fn test_candidate_to_map() {
    let candidate = Candidate {
        name: "Test".to_string(),
        skills: "Python".to_string(),
        ..Default::default()
    };
    let d = candidate.to_map();
    assert_eq!(d.get("name").unwrap(), "Test");
    assert_eq!(d.get("skills").unwrap(), "Python");
}
