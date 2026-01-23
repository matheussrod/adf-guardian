use crate::{config::Severity, engine};
use colored::*;
use std::time::Instant;

pub fn print_human_report(results: &[engine::FileResult], start_time: Instant) {
    println!(
        "{} {} v{}",
        "⛊".bold(),
        "adf-guardian".bold(),
        env!("CARGO_PKG_VERSION")
    );
    println!();

    let mut total_errors_count = 0;
    let mut total_warnings_count = 0;

    for result in results.iter() {
        if result.violations.is_empty() {
            continue;
        }

        let errors_count = result
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count();
        let warnings_count = result
            .violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count();

        total_errors_count += errors_count;
        total_warnings_count += warnings_count;

        let file_symbol = "›".bold();
        println!("{} {}", file_symbol, result.file.bold());

        for v in &result.violations {
            let (rule_symbol, rule_id, message) = match v.severity {
                Severity::Error => (
                    "×".bright_red(),
                    v.rule_id.bright_red(),
                    v.message.bright_red(),
                ),
                Severity::Warning => ("•".yellow(), v.rule_id.yellow(), v.message.yellow()),
            };

            println!("  {} [{}] {}", rule_symbol, rule_id, message);

            if let Some(val) = &v.actual_value {
                println!(
                    "    {} {}",
                    "Actual value:".dimmed(),
                    val.to_string().dimmed()
                );
            }
            println!();
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();

    let mut summary_parts = vec![format!("{} scanned", results.len())];

    if total_errors_count > 0 {
        summary_parts.push(format!("{} failed", total_errors_count).red().to_string());
    } else {
        summary_parts.push("0 failed".to_string().green().to_string());
    }

    if total_warnings_count > 0 {
        summary_parts.push(
            format!("{} warning(s)", total_warnings_count)
                .yellow()
                .to_string(),
        );
    } else {
        summary_parts.push("0 warning(s)".green().to_string());
    }

    summary_parts.push(format!("{:.2}s", elapsed));

    println!(
        "Done: {}",
        summary_parts.join(&format!(" {} ", "·".dimmed()))
    );
}

pub fn print_json_report(results: &[engine::FileResult]) {
    let all_violations: Vec<&engine::Violation> =
        results.iter().flat_map(|r| &r.violations).collect();
    match serde_json::to_string_pretty(&all_violations) {
        Ok(json_output) => println!("{}", json_output),
        Err(e) => print_json_error(&format!("Failed to serialize results to JSON: {}", e)),
    }
}

pub fn print_json_error(msg: &str) {
    let error_json = serde_json::json!({
        "error": msg
    });
    println!("{}", serde_json::to_string_pretty(&error_json).unwrap());
}
