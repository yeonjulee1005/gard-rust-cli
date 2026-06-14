use gard_core::{PackageResult, TierResult, Verdict};
use serde::Serialize;

// ── SARIF 2.1.0 structures ────────────────────────────────────────────────────

#[derive(Serialize)]
struct Sarif {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
    information_uri: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: &'static str,
    name: &'static str,
    short_description: SarifMessage,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: &'static str,
    level: &'static str,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical: SarifPhysical,
}

#[derive(Serialize)]
struct SarifPhysical {
    #[serde(rename = "artifactLocation")]
    artifact: SarifArtifact,
}

#[derive(Serialize)]
struct SarifArtifact {
    uri: String,
}

// ── Rules ─────────────────────────────────────────────────────────────────────

fn make_rules() -> Vec<SarifRule> {
    vec![
        SarifRule {
            id: "GARD001",
            name: "KnownVulnerability",
            short_description: SarifMessage {
                text: "Package has known CVE vulnerabilities (OSV database)".into(),
            },
        },
        SarifRule {
            id: "GARD002",
            name: "SuspiciousMetadata",
            short_description: SarifMessage {
                text: "Package has suspicious metadata (very new or low downloads)".into(),
            },
        },
        SarifRule {
            id: "GARD003",
            name: "MaliciousCode",
            short_description: SarifMessage {
                text: "Package contains malicious code patterns".into(),
            },
        },
    ]
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn render(results: &[PackageResult]) -> anyhow::Result<String> {
    let sarif_results: Vec<SarifResult> = results
        .iter()
        .filter(|r| r.verdict != Verdict::Pass)
        .map(to_sarif_result)
        .collect();

    let sarif = Sarif {
        schema:  "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        version: "2.1.0",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name:            "gard",
                    version:         env!("CARGO_PKG_VERSION"),
                    information_uri: "https://github.com/dewdew/gard",
                    rules:           make_rules(),
                },
            },
            results: sarif_results,
        }],
    };

    Ok(serde_json::to_string_pretty(&sarif)?)
}

fn to_sarif_result(r: &PackageResult) -> SarifResult {
    let (rule_id, message_text) = pick_rule(r);
    let level = match r.verdict {
        Verdict::Block => "error",
        Verdict::Warn => "warning",
        Verdict::Pass => "note",
    };
    let lockfile = match r.package.ecosystem {
        gard_core::Ecosystem::Npm => "package.json",
        gard_core::Ecosystem::PyPI => "requirements.txt",
        gard_core::Ecosystem::Cargo => "Cargo.toml",
    };

    SarifResult {
        rule_id,
        level,
        message: SarifMessage { text: message_text },
        locations: vec![SarifLocation {
            physical: SarifPhysical {
                artifact: SarifArtifact {
                    uri: lockfile.into(),
                },
            },
        }],
    }
}

fn pick_rule(r: &PackageResult) -> (&'static str, String) {
    if let TierResult::Block { reason } | TierResult::Warn { reason } = &r.tier3 {
        return ("GARD003", format!("{} — {}", r.package, reason));
    }
    if let TierResult::Block { reason } | TierResult::Warn { reason } = &r.tier2 {
        return ("GARD002", format!("{} — {}", r.package, reason));
    }
    if let TierResult::Block { reason } | TierResult::Warn { reason } = &r.tier1 {
        return ("GARD001", format!("{} — {}", r.package, reason));
    }
    (
        "GARD002",
        format!("{} — score {}", r.package, r.tier2_score),
    )
}
