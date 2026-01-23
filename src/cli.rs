use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the ADF project to scan
    #[arg(short, long, default_value = ".")]
    pub project_path: PathBuf,

    /// Path to the configuration file
    #[arg(short, long, default_value = "guards.yaml")]
    pub config: PathBuf,

    /// Output results in JSON format
    #[arg(long, default_value_t = false)]
    pub json: bool,
}
