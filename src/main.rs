use anyhow::Result;
use clap::{Parser, Subcommand};

use jobmatch::config;
use jobmatch::pipeline::pipeline::{run_pipeline, PipelineConfig};

#[derive(Parser)]
#[command(name = "jobmatch")]
#[command(about = "JobMatch - Job-candidate matching using NLP with RAG enhancement")]
#[command(version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Reprocess all files (ignore duplicates)
    #[arg(long)]
    force: bool,

    /// Disable RAG enhancement (faster, less accurate)
    #[arg(long, name = "no-rag")]
    no_rag: bool,

    /// Minimum match score to include (default: 30)
    #[arg(long, name = "min-score", default_value = "30.0")]
    min_score: f64,

    /// Number of top matches to display (default: 10)
    #[arg(long, name = "top-n", default_value = "10")]
    top_n: usize,

    /// Custom jobs directory
    #[arg(long, name = "jobs-dir")]
    jobs_dir: Option<String>,

    /// Custom CVs directory
    #[arg(long, name = "cvs-dir")]
    cvs_dir: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the full pipeline (default)
    Run,

    /// Ingest data only (no matching)
    Ingest {
        /// Process only job CSVs
        #[arg(long)]
        jobs: bool,

        /// Process only CV PDFs
        #[arg(long)]
        cvs: bool,

        /// Reprocess all files
        #[arg(long)]
        force: bool,

        /// Export for RAG after ingestion
        #[arg(long, name = "export-rag")]
        export_rag: bool,

        /// Show ingestion status
        #[arg(long)]
        status: bool,
    },

    /// Show database status
    Status,
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Ingest {
            jobs,
            cvs,
            force,
            export_rag,
            status,
        }) => {
            let db = jobmatch::db::database::Database::new(None)?;

            if status {
                let statuses = jobmatch::db::ingestion::get_ingestion_status(&db)?;
                println!("\nIngestion Status:");
                println!("{:-<70}", "");
                for row in &statuses {
                    println!(
                        "  {} | {} | {} | records: {} | skills: {}",
                        row.get("file_path").unwrap_or(&String::new()),
                        row.get("file_type").unwrap_or(&String::new()),
                        row.get("status").unwrap_or(&String::new()),
                        row.get("records_processed").unwrap_or(&String::new()),
                        row.get("skills_extracted").unwrap_or(&String::new()),
                    );
                }
                return Ok(());
            }

            let jobs_dir = config::jobs_path().to_string_lossy().to_string();
            let cvs_dir = config::resumes_path().to_string_lossy().to_string();

            if !cvs || jobs {
                println!("Ingesting jobs from: {}", jobs_dir);
                let result = jobmatch::db::ingestion::ingest_jobs_directory(&db, &jobs_dir, force)?;
                println!(
                    "  {} jobs, {} skills extracted",
                    result.records, result.skills_extracted
                );
            }

            if !jobs || cvs {
                println!("Ingesting CVs from: {}", cvs_dir);
                let result = jobmatch::db::ingestion::ingest_cvs_directory(&db, &cvs_dir, force)?;
                println!(
                    "  {} candidates, {} skills extracted",
                    result.records, result.skills_extracted
                );
            }

            if export_rag {
                let export_dir = config::export_path().to_string_lossy().to_string();
                std::fs::create_dir_all(&export_dir)?;
                let rag_file = format!("{}/rag_documents.jsonl", export_dir);
                let count = jobmatch::rag::rag_support::export_for_rag(&db, &rag_file)?;
                println!("Exported {} RAG documents to {}", count, rag_file);
            }
        }

        Some(Commands::Status) => {
            let db = jobmatch::db::database::Database::new(None)?;
            let stats = db.get_database_stats()?;

            println!("\nDatabase Status:");
            println!("{:-<40}", "");
            for (table, count) in &stats {
                println!("  {:<25} {}", table, count);
            }
        }

        Some(Commands::Run) | None => {
            let pipeline_config = PipelineConfig {
                jobs_dir: cli
                    .jobs_dir
                    .unwrap_or_else(|| config::jobs_path().to_string_lossy().to_string()),
                cvs_dir: cli
                    .cvs_dir
                    .unwrap_or_else(|| config::resumes_path().to_string_lossy().to_string()),
                export_dir: config::export_path().to_string_lossy().to_string(),
                min_match_score: cli.min_score,
                top_n_matches: cli.top_n,
                use_rag_enhancement: !cli.no_rag,
                force_reprocess: cli.force,
            };

            let result = run_pipeline(&pipeline_config)?;

            std::process::exit(if result.matches_found > 0 { 0 } else { 1 });
        }
    }

    Ok(())
}
