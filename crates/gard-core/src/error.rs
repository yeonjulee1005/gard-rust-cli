use thiserror::Error;

#[derive(Debug, Error)]
pub enum GardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("package blocked: {reason}")]
    Blocked { reason: String },

    #[error(".gard/ directory not found — run `gard init` first")]
    NotInitialized,

    #[error("{0}")]
    Other(String),
}
