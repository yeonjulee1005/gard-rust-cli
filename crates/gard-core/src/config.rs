use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub protection: ProtectionConfig,
    pub tier2: Tier2Config,
    pub tier3: Tier3Config,
    pub allowlist: AllowlistConfig,
    pub report: ReportConfig,
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub version: String,
    pub author: String,
    pub sign_commits: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            version: "1".into(),
            author: String::new(),
            sign_commits: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProtectionConfig {
    pub pre_push_scan: bool,
    pub block_on_critical: bool,
    pub block_unsigned_push: bool,
}

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            pre_push_scan: true,
            block_on_critical: true,
            block_unsigned_push: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Tier2Config {
    pub npm_warn_threshold: u64,
    pub npm_flag_threshold: u64,
    pub pypi_warn_threshold: u64,
    pub pypi_flag_threshold: u64,
    pub cargo_warn_threshold: u64,
    pub cargo_flag_threshold: u64,
    pub flag_if_newer_than_days: u32,
    pub warn_if_newer_than_days: u32,
}

impl Default for Tier2Config {
    fn default() -> Self {
        Self {
            npm_warn_threshold: 1_000,
            npm_flag_threshold: 100,
            pypi_warn_threshold: 2_000,
            pypi_flag_threshold: 500,
            cargo_warn_threshold: 500,
            cargo_flag_threshold: 100,
            flag_if_newer_than_days: 7,
            warn_if_newer_than_days: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Tier3Config {
    pub max_file_size_kb: u64,
    pub skip_minified: bool,
    pub obfuscation_block_score: u8,
}

impl Default for Tier3Config {
    fn default() -> Self {
        Self {
            max_file_size_kb: 100,
            skip_minified: true,
            obfuscation_block_score: 80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AllowlistConfig {
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReportConfig {
    pub format: ReportFormat,
    pub show_suggestions: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            format: ReportFormat::Terminal,
            show_suggestions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Terminal,
    Json,
    Sarif,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HooksConfig {
    pub pre_commit: bool,
    pub post_commit: bool,
    pub pre_push: bool,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_commit: false,
            post_commit: true,
            pre_push: true,
        }
    }
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
