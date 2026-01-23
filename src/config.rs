use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Rule {
    pub id: String,
    pub asset: AssetMatcher,
    pub description: Option<String>,
    #[serde(default)]
    pub severity: Severity,
    pub when: Option<Validation>,
    pub validate: Validation,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum AssetMatcher {
    Single(String),
    List(Vec<String>),
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "Error"),
            Severity::Warning => write!(f, "Warning"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Validation {
    pub target: String,
    pub guard: String,
    pub params: serde_json::Value,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())
            .with_context(|| format!("Failed to open config file: {:?}", path.as_ref()))?;

        let config: Config =
            serde_yaml::from_reader(file).context("Failed to parse configuration file")?;

        Ok(config)
    }
}
