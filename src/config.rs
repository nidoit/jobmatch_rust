use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::PathBuf;

/// Base directory (project root)
pub fn base_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Data directory
pub fn data_dir() -> PathBuf {
    base_dir().join("data")
}

/// Jobs data path
pub fn jobs_path() -> PathBuf {
    data_dir().join("jobs")
}

/// Candidates data path
pub fn candidates_path() -> PathBuf {
    data_dir().join("candidates")
}

/// Resumes path
pub fn resumes_path() -> PathBuf {
    candidates_path().join("resumes")
}

/// Matches data path
pub fn matches_path() -> PathBuf {
    data_dir().join("matches")
}

/// Export path
pub fn export_path() -> PathBuf {
    data_dir().join("exports")
}

/// Database path
pub fn db_path() -> PathBuf {
    data_dir().join("db").join("jobmatch.duckdb")
}

/// Job list CSV file
pub fn job_list_file() -> PathBuf {
    jobs_path().join("job_postings_list.csv")
}

/// Job detail CSV file
pub fn job_detail_file() -> PathBuf {
    jobs_path().join("job_postings_detail.csv")
}

/// Candidates CSV file
pub fn candidates_file() -> PathBuf {
    candidates_path().join("candidates.csv")
}

/// Matches CSV file
pub fn matches_file() -> PathBuf {
    matches_path().join("match_results.csv")
}

/// Matching algorithm weights
pub struct MatchingWeights {
    pub skill: f64,
    pub experience: f64,
    pub location: f64,
    pub education: f64,
}

pub const MATCHING_WEIGHTS: MatchingWeights = MatchingWeights {
    skill: 0.40,
    experience: 0.25,
    location: 0.20,
    education: 0.15,
};

/// Skill synonyms dictionary (skill -> canonical name)
pub static SKILL_SYNONYMS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // JavaScript variants
    m.insert("js", "javascript");
    m.insert("ecmascript", "javascript");
    m.insert("es6", "javascript");
    m.insert("es2015", "javascript");
    // TypeScript
    m.insert("ts", "typescript");
    // Python
    m.insert("py", "python");
    m.insert("python3", "python");
    m.insert("python2", "python");
    // C/C++
    m.insert("c++", "cpp");
    m.insert("cplusplus", "cpp");
    // C#
    m.insert("c#", "csharp");
    m.insert("c sharp", "csharp");
    // Database
    m.insert("postgresql", "postgres");
    m.insert("psql", "postgres");
    m.insert("mysql", "sql");
    m.insert("mssql", "sql");
    m.insert("sql server", "sql");
    // Cloud
    m.insert("amazon web services", "aws");
    m.insert("azure", "microsoft azure");
    m.insert("gcp", "google cloud");
    m.insert("google cloud platform", "google cloud");
    // Frameworks
    m.insert("reactjs", "react");
    m.insert("react.js", "react");
    m.insert("vuejs", "vue");
    m.insert("vue.js", "vue");
    m.insert("angularjs", "angular");
    m.insert("angular.js", "angular");
    m.insert("node", "nodejs");
    m.insert("node.js", "nodejs");
    // DevOps
    m.insert("k8s", "kubernetes");
    m.insert("ci/cd", "cicd");
    m.insert("ci cd", "cicd");
    // Embedded/Hardware
    m.insert("embedded c", "embedded systems");
    m.insert("embedded software", "embedded systems");
    m.insert("hw", "hardware");
    m.insert("pcb", "hardware");
    // CAD/Engineering
    m.insert("solidworks", "cad");
    m.insert("autocad", "cad");
    m.insert("catia", "cad");
    m
});

/// Experience level mapping
pub static EXPERIENCE_LEVELS: Lazy<HashMap<&'static str, i32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("entry", 1);
    m.insert("junior", 2);
    m.insert("professional", 3);
    m.insert("experienced", 4);
    m.insert("senior", 5);
    m.insert("specialist", 6);
    m.insert("expert", 7);
    m.insert("senior expert", 8);
    m.insert("lead", 9);
    m.insert("chief", 10);
    m
});

/// Swedish cities for location matching
pub static SWEDEN_CITIES: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        "gothenburg", "göteborg", "stockholm", "malmö", "malmo",
        "uppsala", "västerås", "vasteras", "örebro", "orebro",
        "linköping", "linkoping", "helsingborg", "jönköping", "jonkoping",
        "norrköping", "norrkoping", "lund", "umeå", "umea",
        "gävle", "gavle", "borås", "boras", "södertälje", "sodertalje",
        "eskilstuna", "karlstad", "täby", "taby", "växjö", "vaxjo",
        "halmstad", "sundsvall", "luleå", "lulea", "trollhättan", "trollhattan",
        "östersund", "ostersund", "borlänge", "borlange", "falun",
        "kalmar", "skövde", "skovde", "karlskrona", "kristianstad",
        "skellefteå", "skelleftea", "uddevalla", "varberg", "örnsköldsvik",
        "ornskoldsvik", "landskrona", "nyköping", "nykoping", "motala",
        "kiruna", "ängelholm", "angelholm", "trelleborg", "piteå", "pitea",
        "sandviken", "karlskoga", "lidingö", "lidingo", "alingsås", "alingsas",
        "enköping", "enkoping", "tumba", "mariestad", "köping", "koping",
        "mora", "katrineholm", "vara", "flen", "braas",
    ]
});
