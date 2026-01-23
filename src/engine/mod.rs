mod formatter;
mod guards;

use crate::config::{AssetMatcher, Config, Rule, Severity, Validation};
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
use serde_json_path::JsonPath;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Violation {
    pub rule_id: String,
    pub file: String,
    pub message: String,
    pub severity: Severity,
    pub actual_value: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileResult {
    pub file: String,
    pub violations: Vec<Violation>,
}

pub fn run(config: &Config, root: &Path) -> Result<Vec<FileResult>> {
    let files = crate::scanner::find_json_files(root);

    let results = files
        .par_bridge()
        .map(|file_path| {
            let file_str = file_path.to_string_lossy().to_string();
            let file_res = match File::open(&file_path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("[Warning] Could not open file {}: {}", file_str, e);
                    return FileResult {
                        file: file_str,
                        violations: vec![],
                    };
                }
            };

            let json: Value = match serde_json::from_reader(file_res) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!(
                        "[Warning] Could not parse JSON from file {}: {}",
                        file_str, e
                    );
                    return FileResult {
                        file: file_str,
                        violations: vec![],
                    };
                }
            };

            let violations = config
                .rules
                .iter()
                .filter(|rule| matches_asset_type(&rule.asset, &file_path))
                .flat_map(|rule| check_rule(rule, &json, &file_path))
                .collect::<Vec<_>>();

            FileResult {
                file: file_str,
                violations,
            }
        })
        .collect();

    Ok(results)
}

fn matches_asset_type(matcher: &AssetMatcher, file_path: &Path) -> bool {
    let parent = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str());

    if let Some(folder_name) = parent {
        match matcher {
            AssetMatcher::Single(s) => folder_name.eq_ignore_ascii_case(s),
            AssetMatcher::List(list) => list.iter().any(|s| folder_name.eq_ignore_ascii_case(s)),
        }
    } else {
        false
    }
}

fn check_rule(rule: &Rule, root: &Value, file_path: &Path) -> Vec<Violation> {
    // evaluate 'when' clause if present
    if let Some(when) = &rule.when
        && !evaluate_condition(when, root)
    {
        return vec![]; // Condition not met, skip rule
    }

    // evaluate 'validate' clause
    let path = match JsonPath::parse(&rule.validate.target) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "[Warning] Could not parse JSONPath '{}' for rule '{}': {}",
                &rule.validate.target, &rule.id, e
            );
            return vec![];
        }
    };

    let nodes = path.query(root);

    nodes
        .iter()
        .filter(|node| !check_guard(node, &rule.validate.guard, &rule.validate.params))
        .map(|node| {
            let formatted_value = formatter::format_actual_value(&rule.validate.guard, node);
            Violation {
                rule_id: rule.id.clone(),
                file: file_path.to_string_lossy().to_string(),
                message: rule
                    .description
                    .clone()
                    .unwrap_or_else(|| "Rule violation".to_string()),
                severity: rule.severity,
                actual_value: Some(formatted_value),
            }
        })
        .collect()
}

fn evaluate_condition(validation: &Validation, root: &Value) -> bool {
    let path = match JsonPath::parse(&validation.target) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let nodes = path.query(root);

    if nodes.is_empty() {
        return false;
    }

    nodes
        .iter()
        .all(|node| check_guard(node, &validation.guard, &validation.params))
}

fn check_guard(node: &Value, guard: &str, params: &Value) -> bool {
    match guard {
        "PatternMatch" => guards::check_pattern_match(node, params),
        "AllowedValues" => guards::check_allowed_values(node, params),
        "Exists" => guards::check_exists(node, params),
        "Range" => guards::check_range(node, params),
        "Count" => guards::check_count(node, params),
        "StringLength" => guards::check_string_length(node, params),
        other => {
            eprintln!(
                "[Warning] Unknown guard '{}', the check will be skipped.",
                other
            );
            true // Unknown guard, pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AssetMatcher, Validation};
    use serde_json::json;

    #[test]
    fn test_pattern_match() {
        let params = json!({ "regex": "^test" });
        assert!(guards::check_pattern_match(&json!("test_value"), &params));
        assert!(!guards::check_pattern_match(&json!("value_test"), &params));

        let params_negative = json!({ "regex": "^test", "negative": true });
        assert!(!guards::check_pattern_match(
            &json!("test_value"),
            &params_negative
        ));
        assert!(guards::check_pattern_match(
            &json!("value_test"),
            &params_negative
        ));
    }

    #[test]
    fn test_allowed_values() {
        let params = json!({ "values": ["Prod", "UAT"] });
        assert!(guards::check_allowed_values(&json!("Prod"), &params));
        assert!(!guards::check_allowed_values(&json!("Dev"), &params));

        let params_ci = json!({ "values": ["Prod"], "case_sensitive": false });
        assert!(guards::check_allowed_values(&json!("prod"), &params_ci));

        let params_deny = json!({ "values": ["HTTP"], "mode": "Deny" });
        assert!(guards::check_allowed_values(&json!("HTTPS"), &params_deny));
        assert!(!guards::check_allowed_values(&json!("HTTP"), &params_deny));
    }

    #[test]
    fn test_exists() {
        let params_default = json!({});
        assert!(guards::check_exists(
            &json!({"key": "value"}),
            &params_default
        ));
        assert!(!guards::check_exists(&json!(null), &params_default));

        let params_exist = json!({ "should_exist": true });
        assert!(guards::check_exists(&json!("a value"), &params_exist));
        assert!(!guards::check_exists(&json!(null), &params_exist));

        let params_not_exist = json!({ "should_exist": false });
        assert!(guards::check_exists(&json!(null), &params_not_exist));
        assert!(!guards::check_exists(&json!(123), &params_not_exist));
    }

    #[test]
    fn test_range() {
        let params = json!({ "min": 10, "max": 20 });
        assert!(guards::check_range(&json!(15), &params));
        assert!(guards::check_range(&json!(10), &params));
        assert!(guards::check_range(&json!(20), &params));
        assert!(!guards::check_range(&json!(5), &params));
        assert!(!guards::check_range(&json!(25), &params));
        assert!(!guards::check_range(&json!("not a number"), &params));
    }

    #[test]
    fn test_count() {
        let params = json!({ "min": 1 });
        assert!(guards::check_count(&json!(["a"]), &params));
        assert!(!guards::check_count(&json!([]), &params));
        assert!(!guards::check_count(&json!("not an array"), &params));
    }

    #[test]
    fn test_string_length() {
        let params = json!({ "min": 5, "max": 10 });
        assert!(guards::check_string_length(&json!("hello"), &params));
        assert!(guards::check_string_length(&json!("hello_1234"), &params));
        assert!(guards::check_string_length(&json!("1234567"), &params));

        assert!(!guards::check_string_length(&json!("four"), &params));
        assert!(!guards::check_string_length(
            &json!("hello_world_11"),
            &params
        ));

        assert!(!guards::check_string_length(&json!(12345), &params));

        let params_unicode = json!({ "min": 3, "max": 5 });
        assert!(guards::check_string_length(&json!("áéí"), &params_unicode));
        assert!(guards::check_string_length(
            &json!("áéíóú"),
            &params_unicode
        ));
        assert!(!guards::check_string_length(&json!("áé"), &params_unicode));
        assert!(!guards::check_string_length(
            &json!("áéíóúÁ"),
            &params_unicode
        ));
    }

    #[test]
    fn test_check_rule_when_clause_met() {
        let rule = Rule {
            id: "test-when-met".to_string(),
            asset: AssetMatcher::Single("pipeline".to_string()),
            description: None,
            severity: Severity::Error,
            when: Some(Validation {
                target: "$.properties.type".to_string(),
                guard: "AllowedValues".to_string(),
                params: json!({ "values": ["MappingDataFlow"] }),
            }),
            validate: Validation {
                target: "$.name".to_string(),
                guard: "PatternMatch".to_string(),
                params: json!({ "regex": "^pl_" }),
            },
        };

        let json = json!({ "properties": { "type": "MappingDataFlow" }, "name": "wrong_name" });
        assert_eq!(
            check_rule(&rule, &json, Path::new("pipeline/test.json")).len(),
            1
        );
    }

    #[test]
    fn test_check_rule_when_clause_not_met() {
        let rule = Rule {
            id: "test-when-not-met".to_string(),
            asset: AssetMatcher::Single("pipeline".to_string()),
            description: None,
            severity: Severity::Error,
            when: Some(Validation {
                target: "$.properties.type".to_string(),
                guard: "AllowedValues".to_string(),
                params: json!({ "values": ["MappingDataFlow"] }),
            }),
            validate: Validation {
                target: "$.name".to_string(),
                guard: "PatternMatch".to_string(),
                params: json!({ "regex": "^pl_" }),
            },
        };

        let json = json!({ "properties": { "type": "ExecutePipeline" }, "name": "wrong_name" });
        assert!(check_rule(&rule, &json, Path::new("pipeline/test.json")).is_empty());
    }

    #[test]
    fn test_check_rule_no_when_clause() {
        let rule = Rule {
            id: "test-no-when".to_string(),
            asset: AssetMatcher::Single("pipeline".to_string()),
            description: None,
            severity: Severity::Error,
            when: None,
            validate: Validation {
                target: "$.name".to_string(),
                guard: "PatternMatch".to_string(),
                params: json!({ "regex": "^pl_" }),
            },
        };

        let json = json!({ "name": "wrong_name" });
        assert_eq!(
            check_rule(&rule, &json, Path::new("pipeline/test.json")).len(),
            1
        );

        let json_ok = json!({ "name": "pl_correct_name" });
        assert!(check_rule(&rule, &json_ok, Path::new("pipeline/test.json")).is_empty());
    }

    #[test]
    fn test_matches_asset_type() {
        let matcher = AssetMatcher::Single("pipeline".to_string());
        assert!(matches_asset_type(
            &matcher,
            Path::new("./pipeline/test.json")
        ));
        assert!(!matches_asset_type(
            &matcher,
            Path::new("./dataset/test.json")
        ));

        let matcher_list = AssetMatcher::List(vec!["pipeline".to_string(), "dataset".to_string()]);
        assert!(matches_asset_type(
            &matcher_list,
            Path::new("./pipeline/test.json")
        ));
        assert!(matches_asset_type(
            &matcher_list,
            Path::new("./dataset/test.json")
        ));
        assert!(!matches_asset_type(
            &matcher_list,
            Path::new("./trigger/test.json")
        ));
    }
}
