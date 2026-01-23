mod cli;
mod config;
mod engine;
mod reporter;
mod scanner;

use anyhow::{Context, Result};
use clap::Parser;
use cli::Cli;
use config::{Config, Severity};
use std::process::exit;
use std::time::Instant;

fn main() -> Result<()> {
    let start_time = Instant::now();
    let args = Cli::parse();

    if !args.config.exists() {
        if args.json {
            reporter::print_json_error("Config file not found");
        } else {
            eprintln!("Error: Config file not found at {:?}", args.config);
        }
        exit(1);
    }

    let config = Config::load(&args.config)
        .with_context(|| format!("Failed to load configuration from {:?}", args.config))?;

    let results = engine::run(&config, &args.project_path)?;

    if args.json {
        reporter::print_json_report(&results);
    } else {
        reporter::print_human_report(&results, start_time);
    }

    let has_errors = results
        .iter()
        .any(|r| r.violations.iter().any(|v| v.severity == Severity::Error));
    if has_errors {
        exit(1);
    }

    Ok(())
}
