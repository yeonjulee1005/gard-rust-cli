use gard_core::{PackageResult, Verdict};
use serde::Serialize;

#[derive(Serialize)]
struct JsonReport<'a> {
    generated_at: String,
    total: usize,
    blocked: usize,
    warned: usize,
    passed: usize,
    results: &'a [PackageResult],
}

pub fn render(results: &[PackageResult]) -> anyhow::Result<String> {
    let blocked = results
        .iter()
        .filter(|r| r.verdict == Verdict::Block)
        .count();
    let warned = results
        .iter()
        .filter(|r| r.verdict == Verdict::Warn)
        .count();
    let passed = results
        .iter()
        .filter(|r| r.verdict == Verdict::Pass)
        .count();

    let report = JsonReport {
        generated_at: chrono::Utc::now().to_rfc3339(),
        total: results.len(),
        blocked,
        warned,
        passed,
        results,
    };
    Ok(serde_json::to_string_pretty(&report)?)
}
