use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Npm,
    PyPI,
    Cargo,
}

impl Ecosystem {
    pub fn osv_name(&self) -> &str {
        match self {
            Self::Npm => "npm",
            Self::PyPI => "PyPI",
            Self::Cargo => "crates.io",
        }
    }
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Npm => write!(f, "npm"),
            Self::PyPI => write!(f, "PyPI"),
            Self::Cargo => write!(f, "cargo"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub ecosystem: Ecosystem,
}

impl Package {
    pub fn new(name: impl Into<String>, version: impl Into<String>, ecosystem: Ecosystem) -> Self {
        Self { name: name.into(), version: version.into(), ecosystem }
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// 각 Tier의 검사 결과
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "UPPERCASE")]
pub enum TierResult {
    Pass,
    Warn { reason: String },
    Block { reason: String },
    Skipped,
    Error { message: String },
}

impl TierResult {
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Block { .. })
    }

    pub fn is_flagged(&self) -> bool {
        matches!(self, Self::Warn { .. } | Self::Block { .. })
    }
}

/// 패키지 최종 판정
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Pass,
    Warn,
    Block,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass  => write!(f, "PASS"),
            Self::Warn  => write!(f, "WARN"),
            Self::Block => write!(f, "BLOCK"),
        }
    }
}

/// 패키지 검사 전체 결과 (manifest 저장 + 리포트 출력에 사용)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageResult {
    pub package: Package,
    pub tier1: TierResult,
    pub tier2: TierResult,
    pub tier2_score: u8,
    pub tier2_age_days: Option<u64>,
    pub tier2_downloads: Option<u64>,
    pub tier3: TierResult,
    pub verdict: Verdict,
    pub checked_at: DateTime<Utc>,
    pub ai_tool: Option<String>,
}
