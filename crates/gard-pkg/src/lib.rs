pub mod manifest_diff;
pub mod scorer;
pub mod tier1_osv;
pub mod tier2_meta;
pub mod tier3_analyzer;

pub use tier3_analyzer::{Finding, Severity};

use chrono::Utc;
use gard_core::{Config, Package, PackageResult, TierResult, Verdict};
use std::path::Path;

// ── AI tool detection ─────────────────────────────────────────────────────────

fn detect_ai_tool() -> Option<String> {
    if std::env::var("CLAUDE_CODE").is_ok() || std::env::var("ANTHROPIC_CLAUDE_CODE").is_ok() {
        return Some("claude-code".into());
    }
    if std::env::var("CURSOR_TRACE_ID").is_ok() {
        return Some("cursor".into());
    }
    if std::env::var("COPILOT_AGENT").is_ok() {
        return Some("github-copilot".into());
    }
    None
}

// ── Main scan pipeline ────────────────────────────────────────────────────────

/// Scan a single package through the three-tier detection pipeline.
///
/// T1 and T2 run in parallel. T3 runs only when T2 score ≥ 61 and `pkg_path` is available.
pub async fn scan_package(
    client: &reqwest::Client,
    pkg: &Package,
    pkg_path: Option<&Path>,
    cfg: &Config,
) -> PackageResult {
    let ai_tool = detect_ai_tool();

    // ── Tier 1 + Tier 2 in parallel ──────────────────────────────────────────
    let (tier1, t2) = tokio::join!(
        tier1_osv::check(client, pkg),
        tier2_meta::check(client, pkg, cfg),
    );

    let tier2 = t2.result.clone();
    let tier2_score = t2.score;

    // Short-circuit: T1 block dominates regardless of T2
    if tier1.is_blocking() {
        return PackageResult {
            package: pkg.clone(),
            tier1,
            tier2,
            tier2_score,
            tier2_age_days: t2.age_days,
            tier2_downloads: t2.downloads,
            tier3: TierResult::Skipped,
            verdict: Verdict::Block,
            checked_at: Utc::now(),
            ai_tool,
        };
    }

    // ── Tier 3: source analysis (only when score ≥ 61 and path available) ────
    let tier3 = if tier2_score >= 61 {
        match pkg_path {
            Some(path) => tier3_analyzer::analyze(path, &pkg.ecosystem, cfg),
            None => TierResult::Skipped,
        }
    } else {
        TierResult::Skipped
    };

    let verdict = scorer::aggregate(&tier1, &tier2, &tier3);

    PackageResult {
        package: pkg.clone(),
        tier1,
        tier2,
        tier2_score,
        tier2_age_days: t2.age_days,
        tier2_downloads: t2.downloads,
        tier3,
        verdict,
        checked_at: Utc::now(),
        ai_tool,
    }
}
