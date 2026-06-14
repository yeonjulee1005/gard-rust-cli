use console::style;
use gard_core::{PackageResult, TierResult, Verdict};

// ── Runtime context ───────────────────────────────────────────────────────────

pub enum RuntimeContext {
    Standard,
    ClaudeCode,
    Cursor,
    GitHubActions,
}

pub fn detect_runtime_context() -> RuntimeContext {
    if std::env::var("CLAUDE_CODE").is_ok() || std::env::var("ANTHROPIC_CLAUDE_CODE").is_ok() {
        return RuntimeContext::ClaudeCode;
    }
    if std::env::var("CURSOR_TRACE_ID").is_ok() {
        return RuntimeContext::Cursor;
    }
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        return RuntimeContext::GitHubActions;
    }
    RuntimeContext::Standard
}

// ── Entry points ──────────────────────────────────────────────────────────────

pub fn print_results(results: &[PackageResult], ctx: &RuntimeContext) {
    match ctx {
        RuntimeContext::Standard => print_standard(results),
        RuntimeContext::ClaudeCode | RuntimeContext::Cursor => print_compact(results),
        RuntimeContext::GitHubActions => print_ci(results),
    }
}

pub fn print_init_success(ci_detected: bool, ci_name: Option<&str>) {
    eprintln!();
    eprintln!(
        "  {}  v{}  SIGN · SCAN · PROTECT",
        style("gard").green().bold(),
        env!("CARGO_PKG_VERSION")
    );
    eprintln!();
    eprintln!("  {} git hooks installed   (pre-push)", style("✓").green());
    eprintln!(
        "  {} manifest created      (.gard/manifest.json)",
        style("✓").green()
    );
    eprintln!(
        "  {} config written        (.gard/config.toml)",
        style("✓").green()
    );
    if ci_detected {
        eprintln!();
        if let Some(name) = ci_name {
            eprintln!("  {} {} detected", style("✓").green(), name);
            eprintln!("  {} workflow created", style("✓").green());
        }
    }
    eprintln!();
    eprintln!(
        "  {}  gard is protecting this repo — local + remote.",
        style("🛡").green()
    );
    eprintln!();
}

// ── Standard (Aura × Retro) ───────────────────────────────────────────────────

fn print_standard(results: &[PackageResult]) {
    let n = results.len();
    eprintln!();
    eprintln!(
        "  {}  checking {} package{} ...",
        style("gard").green().bold(),
        n,
        if n == 1 { "" } else { "s" }
    );
    eprintln!();

    for r in results {
        print_package_row(r);
    }
    eprintln!();

    let blocked: Vec<_> = results
        .iter()
        .filter(|r| r.verdict == Verdict::Block)
        .collect();
    let warned: Vec<_> = results
        .iter()
        .filter(|r| r.verdict == Verdict::Warn)
        .collect();

    if !blocked.is_empty() {
        for r in &blocked {
            let reason = block_reason(r);
            eprintln!(
                "  {}  {}  ({})",
                style("🚨 BLOCK").red().bold(),
                style(r.package.to_string()).red(),
                reason
            );
        }
        eprintln!("  {} push rejected.", style("✗").red().bold());
        eprintln!();
        eprintln!(
            "  {} run `gard explain {}` for details",
            style("💡").yellow(),
            blocked[0].package.name
        );
        eprintln!(
            "     run `gard allow {}` to override (not recommended)",
            blocked[0].package.name
        );
    } else if !warned.is_empty() {
        eprintln!("  {} manifest updated → .gard/", style("✍").cyan());
        eprintln!(
            "  → push allowed with {} warning{}",
            warned.len(),
            if warned.len() == 1 { "" } else { "s" }
        );
    } else {
        eprintln!("  {} manifest updated → .gard/", style("✍").cyan());
        eprintln!("  push allowed.");
    }
    eprintln!();
}

fn print_package_row(r: &PackageResult) {
    let name_col = format!("{:<35}", r.package.to_string());
    let info = format_tier_info(r);
    match r.verdict {
        Verdict::Pass => eprintln!(
            "  {}  {}  {}",
            style("✓").green(),
            style(name_col).white(),
            style(info).dim()
        ),
        Verdict::Warn => eprintln!(
            "  {}  {}  {}",
            style("⚠").yellow(),
            style(name_col).yellow(),
            style(info).dim()
        ),
        Verdict::Block => eprintln!(
            "  {}  {}  {}",
            style("🚨").red(),
            style(name_col).red().bold(),
            style(info).red()
        ),
    }
}

fn format_tier_info(r: &PackageResult) -> String {
    let t1 = match &r.tier1 {
        TierResult::Pass => "T1 pass".into(),
        TierResult::Block { .. } => "T1 BLOCK".into(),
        TierResult::Warn { .. } => "T1 warn".into(),
        TierResult::Error { .. } => "T1 error".into(),
        TierResult::Skipped => "T1 skip".into(),
    };

    let mut parts = vec![t1];

    if !matches!(&r.tier2, TierResult::Skipped) {
        let label = if r.tier2_score >= 61 {
            format!("T2 flag (score {})", r.tier2_score)
        } else {
            format!("T2 {} (score {})", tier_label(&r.tier2), r.tier2_score)
        };
        parts.push(label);
    }

    if let Some(dl) = r.tier2_downloads {
        let dl_str = if dl >= 1_000_000 {
            format!("{}M weekly", dl / 1_000_000)
        } else if dl >= 1_000 {
            format!("{}K weekly", dl / 1_000)
        } else {
            format!("{} downloads", dl)
        };
        parts.push(dl_str);
    }

    parts.join(" · ")
}

fn tier_label(t: &TierResult) -> &'static str {
    match t {
        TierResult::Pass => "pass",
        TierResult::Warn { .. } => "warn",
        TierResult::Block { .. } => "BLOCK",
        TierResult::Error { .. } => "error",
        TierResult::Skipped => "skip",
    }
}

fn block_reason(r: &PackageResult) -> String {
    for t in [&r.tier3, &r.tier2, &r.tier1] {
        if let TierResult::Block { reason } = t {
            return reason.clone();
        }
    }
    "unknown reason".into()
}

// ── Compact (Claude Code / Cursor) ────────────────────────────────────────────

fn print_compact(results: &[PackageResult]) {
    let n = results.len();
    eprintln!();
    eprintln!(
        "  {}  {} package{} to scan",
        style("gard").green().bold(),
        n,
        if n == 1 { "" } else { "s" }
    );
    eprintln!();

    for r in results {
        let (icon, verdict_str) = match r.verdict {
            Verdict::Pass => (style("✓").green().to_string(), "PASS"),
            Verdict::Warn => (style("⚠").yellow().to_string(), "WARN"),
            Verdict::Block => (style("✗").red().to_string(), "BLOCK"),
        };
        eprintln!(
            "  {} {:<32}  {} (score {})",
            icon,
            r.package.to_string(),
            verdict_str,
            r.tier2_score
        );
    }

    eprintln!();
    let any_blocked = results.iter().any(|r| r.verdict == Verdict::Block);
    if any_blocked {
        eprintln!("  {} push rejected.", style("✗").red().bold());
    } else {
        eprintln!("  {} manifest updated → .gard/", style("✍").cyan());
    }
    eprintln!();
}

// ── CI (GitHub Actions annotations) ──────────────────────────────────────────

fn print_ci(results: &[PackageResult]) {
    for r in results {
        let lockfile = match r.package.ecosystem {
            gard_core::Ecosystem::Npm => "package.json",
            gard_core::Ecosystem::PyPI => "requirements.txt",
            gard_core::Ecosystem::Cargo => "Cargo.toml",
        };
        match &r.verdict {
            Verdict::Block => {
                let reason = block_reason(r);
                eprintln!(
                    "::error file={lockfile}::gard: {} BLOCKED — {reason}",
                    r.package
                );
            }
            Verdict::Warn => {
                eprintln!(
                    "::warning file={lockfile}::gard: {} WARN (score {})",
                    r.package, r.tier2_score
                );
            }
            Verdict::Pass => {
                eprintln!("::notice file={lockfile}::gard: {} PASS", r.package);
            }
        }
    }

    let blocked = results
        .iter()
        .filter(|r| r.verdict == Verdict::Block)
        .count();
    let warned = results
        .iter()
        .filter(|r| r.verdict == Verdict::Warn)
        .count();
    eprintln!(
        "[gard] {} scanned  {} blocked  {} warned",
        results.len(),
        blocked,
        warned
    );
}
