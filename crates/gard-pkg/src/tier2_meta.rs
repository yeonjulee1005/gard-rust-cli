use chrono::{DateTime, Utc};
use gard_core::{Config, Ecosystem, Package, TierResult};
use serde::Deserialize;

// ── Tier 2 output ─────────────────────────────────────────────────────────────

pub struct Tier2Output {
    pub result: TierResult,
    pub score: u8,
    pub age_days: Option<u64>,
    pub downloads: Option<u64>,
}

// ── Public entry point ────────────────────────────────────────────────────────

/// Tier 2: fetch package age + download count, compute risk score.
///
/// Score 0–40  → Pass
/// Score 41–60 → Warn
/// Score 61+   → Warn (flagged; caller triggers Tier 3 if pkg_path available)
pub async fn check(client: &reqwest::Client, pkg: &Package, cfg: &Config) -> Tier2Output {
    let (age_days, downloads) = fetch_metadata(client, pkg).await;
    let score = compute_score(age_days, downloads, &pkg.ecosystem, cfg);
    let result = score_to_result(score);
    Tier2Output {
        result,
        score,
        age_days,
        downloads,
    }
}

// ── Score computation (pure — unit-testable) ──────────────────────────────────

/// Compute a 0–100 risk score from age and download data.
pub fn compute_score(
    age_days: Option<u64>,
    downloads: Option<u64>,
    ecosystem: &Ecosystem,
    cfg: &Config,
) -> u8 {
    let t2 = &cfg.tier2;

    // Age scoring
    let age_score: u8 = match age_days {
        None => 20, // unavailable → suspicious
        Some(d) => {
            let flag = t2.flag_if_newer_than_days as u64;
            let warn = t2.warn_if_newer_than_days as u64;
            if d < flag {
                30
            } else if d < warn {
                15
            } else if d < warn * 3 {
                5
            } else {
                0
            }
        }
    };

    // Download scoring (thresholds differ per ecosystem)
    let (flag_dl, warn_dl) = match ecosystem {
        Ecosystem::Npm => (t2.npm_flag_threshold, t2.npm_warn_threshold),
        Ecosystem::PyPI => (t2.pypi_flag_threshold, t2.pypi_warn_threshold),
        Ecosystem::Cargo => (t2.cargo_flag_threshold, t2.cargo_warn_threshold),
    };

    let dl_score: u8 = match downloads {
        None => 20, // unavailable → suspicious
        Some(d) => {
            if d < flag_dl {
                30
            } else if d < warn_dl {
                15
            } else if d < warn_dl * 10 {
                5
            } else {
                0
            }
        }
    };

    (age_score + dl_score).min(100)
}

fn score_to_result(score: u8) -> TierResult {
    match score {
        0..=40 => TierResult::Pass,
        41..=60 => TierResult::Warn {
            reason: format!("risk score {score}/100 — low download count or recent package"),
        },
        _ => TierResult::Warn {
            reason: format!("risk score {score}/100 — flagged for source analysis"),
        },
    }
}

// ── Per-ecosystem metadata fetching ──────────────────────────────────────────

async fn fetch_metadata(client: &reqwest::Client, pkg: &Package) -> (Option<u64>, Option<u64>) {
    match &pkg.ecosystem {
        Ecosystem::Npm => fetch_npm(client, &pkg.name).await,
        Ecosystem::PyPI => fetch_pypi(client, &pkg.name).await,
        Ecosystem::Cargo => fetch_cargo(client, &pkg.name).await,
    }
}

// ── npm ───────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct NpmDownloads {
    downloads: Option<u64>,
}

#[derive(Deserialize)]
struct NpmRegistry {
    time: Option<NpmTime>,
}

#[derive(Deserialize)]
struct NpmTime {
    created: Option<String>,
}

async fn fetch_npm(client: &reqwest::Client, name: &str) -> (Option<u64>, Option<u64>) {
    let downloads = async {
        let url = format!("https://api.npmjs.org/downloads/point/last-week/{name}");
        let resp: NpmDownloads = client.get(&url).send().await.ok()?.json().await.ok()?;
        resp.downloads
    };

    let age = async {
        let url = format!("https://registry.npmjs.org/{name}");
        let resp: NpmRegistry = client.get(&url).send().await.ok()?.json().await.ok()?;
        let created_str = resp.time?.created?;
        let dt = DateTime::parse_from_rfc3339(&created_str).ok()?;
        let age = (Utc::now() - dt.with_timezone(&Utc)).num_days();
        Some(age.max(0) as u64)
    };

    tokio::join!(downloads, age)
}

// ── PyPI ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PypiRecent {
    data: Option<PypiRecentData>,
}

#[derive(Deserialize)]
struct PypiRecentData {
    last_week: Option<u64>,
}

#[derive(Deserialize)]
struct PypiInfo {
    urls: Option<Vec<PypiUrl>>,
}

#[derive(Deserialize)]
struct PypiUrl {
    upload_time: Option<String>,
}

async fn fetch_pypi(client: &reqwest::Client, name: &str) -> (Option<u64>, Option<u64>) {
    let downloads = async {
        let url = format!("https://pypistats.org/api/packages/{name}/recent");
        let resp: PypiRecent = client.get(&url).send().await.ok()?.json().await.ok()?;
        resp.data?.last_week
    };

    let age = async {
        let url = format!("https://pypi.org/pypi/{name}/json");
        let resp: PypiInfo = client.get(&url).send().await.ok()?.json().await.ok()?;
        // `urls` contains the latest release files; use first upload_time as proxy
        let upload = resp.urls?.into_iter().find_map(|u| u.upload_time)?;
        // PyPI format: "2020-01-01T00:00:00"  (no timezone — assume UTC)
        let dt = chrono::NaiveDateTime::parse_from_str(&upload, "%Y-%m-%dT%H:%M:%S").ok()?;
        let dt_utc = dt.and_utc();
        let age = (Utc::now() - dt_utc).num_days();
        Some(age.max(0) as u64)
    };

    tokio::join!(downloads, age)
}

// ── crates.io ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CratesResp {
    #[serde(rename = "crate")]
    krate: Option<CrateData>,
}

#[derive(Deserialize)]
struct CrateData {
    created_at: Option<String>,
    recent_downloads: Option<u64>,
}

async fn fetch_cargo(client: &reqwest::Client, name: &str) -> (Option<u64>, Option<u64>) {
    let url = format!("https://crates.io/api/v1/crates/{name}");
    // crates.io requires a User-Agent header identifying the application
    let resp: CratesResp = match client
        .get(&url)
        .header("User-Agent", "gard/0.1.0 (github.com/dewdew/gard)")
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(d) => d,
            Err(_) => return (None, None),
        },
        Err(_) => return (None, None),
    };

    let krate = match resp.krate {
        Some(k) => k,
        None => return (None, None),
    };

    let downloads = krate.recent_downloads;

    let age = krate.created_at.and_then(|s| {
        let dt = DateTime::parse_from_rfc3339(&s).ok()?;
        let age = (Utc::now() - dt.with_timezone(&Utc)).num_days();
        Some(age.max(0) as u64)
    });

    (age, downloads)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gard_core::{Config, Ecosystem};

    fn default_cfg() -> Config {
        Config::default()
    }

    #[test]
    fn old_popular_package_scores_zero() {
        let cfg = default_cfg();
        // 2000 days old, 5M weekly downloads (npm)
        let score = compute_score(Some(2000), Some(5_000_000), &Ecosystem::Npm, &cfg);
        assert_eq!(score, 0);
    }

    #[test]
    fn brand_new_no_downloads_scores_max() {
        let cfg = default_cfg();
        // 1 day old, 3 downloads
        let score = compute_score(Some(1), Some(3), &Ecosystem::Npm, &cfg);
        assert_eq!(score, 60); // 30 (age) + 30 (downloads)
    }

    #[test]
    fn unavailable_meta_adds_penalty() {
        let cfg = default_cfg();
        let score = compute_score(None, None, &Ecosystem::Npm, &cfg);
        assert_eq!(score, 40); // 20 + 20
    }

    #[test]
    fn warn_threshold_triggers_warn() {
        let cfg = default_cfg();
        // Score 61 → flagged
        let score = compute_score(Some(1), Some(3), &Ecosystem::PyPI, &cfg);
        assert!(score >= 41);
        let result = score_to_result(score);
        assert!(result.is_flagged());
    }

    /// Run manually: cargo test -p gard-pkg -- --ignored
    #[tokio::test]
    #[ignore]
    async fn lodash_passes_tier2() {
        let client = reqwest::Client::new();
        let cfg = Config::default();
        let pkg = Package::new("lodash", "4.17.21", Ecosystem::Npm);
        let out = check(&client, &pkg, &cfg).await;
        assert_eq!(out.result, TierResult::Pass, "score={}", out.score);
    }
}
