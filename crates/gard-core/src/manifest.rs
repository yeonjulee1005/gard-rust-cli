use crate::types::{Ecosystem, PackageResult, TierResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub schema: String,
    pub repo: Option<String>,
    pub packages: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
    pub checked_at: String,
    pub tier1: TierSummary,
    pub tier2: Tier2Summary,
    pub tier3: TierSummary,
    #[serde(rename = "final")]
    pub final_verdict: String,
    pub ai_tool: Option<String>,
    pub added_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierSummary {
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tier2Summary {
    pub result: String,
    pub score: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_days: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly_downloads: Option<u64>,
}

impl Manifest {
    pub fn new(repo: Option<String>) -> Self {
        Self {
            version: "1".into(),
            schema: "https://gard.dev/schema/v1".into(),
            repo,
            packages: vec![],
        }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn upsert(&mut self, result: &PackageResult, added_by: Option<String>) {
        let entry = ManifestEntry::from_result(result, added_by);
        if let Some(pos) = self
            .packages
            .iter()
            .position(|e| e.name == result.package.name && e.ecosystem == result.package.ecosystem)
        {
            self.packages[pos] = entry;
        } else {
            self.packages.push(entry);
        }
    }
}

impl ManifestEntry {
    pub fn from_result(r: &PackageResult, added_by: Option<String>) -> Self {
        Self {
            name: r.package.name.clone(),
            version: r.package.version.clone(),
            ecosystem: r.package.ecosystem.clone(),
            checked_at: r.checked_at.to_rfc3339(),
            tier1: TierSummary::from_tier(&r.tier1),
            tier2: Tier2Summary {
                result: tier_label(&r.tier2),
                score: r.tier2_score,
                age_days: r.tier2_age_days,
                weekly_downloads: r.tier2_downloads,
            },
            tier3: TierSummary::from_tier(&r.tier3),
            final_verdict: r.verdict.to_string(),
            ai_tool: r.ai_tool.clone(),
            added_by,
        }
    }
}

impl TierSummary {
    fn from_tier(t: &TierResult) -> Self {
        match t {
            TierResult::Pass => Self {
                result: "PASS".into(),
                reason: None,
            },
            TierResult::Skipped => Self {
                result: "SKIPPED".into(),
                reason: None,
            },
            TierResult::Warn { reason } => Self {
                result: "WARN".into(),
                reason: Some(reason.clone()),
            },
            TierResult::Block { reason } => Self {
                result: "BLOCK".into(),
                reason: Some(reason.clone()),
            },
            TierResult::Error { message } => Self {
                result: "ERROR".into(),
                reason: Some(message.clone()),
            },
        }
    }
}

fn tier_label(t: &TierResult) -> String {
    match t {
        TierResult::Pass => "PASS".into(),
        TierResult::Skipped => "SKIPPED".into(),
        TierResult::Warn { .. } => "WARN".into(),
        TierResult::Block { .. } => "BLOCK".into(),
        TierResult::Error { .. } => "ERROR".into(),
    }
}
