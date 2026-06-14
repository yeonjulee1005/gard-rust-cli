use gard_core::{Package, TierResult};
use serde::Deserialize;

const OSV_API: &str = "https://api.osv.dev/v1/query";

#[derive(Deserialize)]
struct OsvResponse {
    #[serde(default)]
    vulns: Vec<OsvVuln>,
}

#[derive(Deserialize)]
struct OsvVuln {
    id: String,
}

/// Tier 1: query the OSV vulnerability database.
///
/// - Vulnerabilities found → `Block`
/// - Clean              → `Pass`
/// - Network/parse error → `Error` (caller proceeds to Tier 2 with a warning)
pub async fn check(client: &reqwest::Client, pkg: &Package) -> TierResult {
    let body = serde_json::json!({
        "package": {
            "name": pkg.name,
            "ecosystem": pkg.ecosystem.osv_name(),
        },
        "version": pkg.version,
    });

    let response = match client.post(OSV_API).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            return TierResult::Error {
                message: format!("OSV API request failed: {e}"),
            }
        }
    };

    let data: OsvResponse = match response.json().await {
        Ok(d) => d,
        Err(e) => {
            return TierResult::Error {
                message: format!("OSV API response parse failed: {e}"),
            }
        }
    };

    if data.vulns.is_empty() {
        TierResult::Pass
    } else {
        let shown: Vec<&str> = data.vulns.iter().take(3).map(|v| v.id.as_str()).collect();
        let trailer = if data.vulns.len() > 3 { ", …" } else { "" };
        TierResult::Block {
            reason: format!(
                "{} known vulnerability(-ies) [{}{}]",
                data.vulns.len(),
                shown.join(", "),
                trailer,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gard_core::Ecosystem;

    fn make_pkg(name: &str, version: &str, eco: Ecosystem) -> Package {
        Package::new(name, version, eco)
    }

    /// Run manually with: cargo test -p gard-pkg -- --ignored
    #[tokio::test]
    #[ignore]
    async fn lodash_should_pass() {
        let client = reqwest::Client::new();
        let pkg = make_pkg("lodash", "4.17.21", Ecosystem::Npm);
        let result = check(&client, &pkg).await;
        assert_eq!(result, TierResult::Pass);
    }

    #[tokio::test]
    #[ignore]
    async fn event_stream_should_block() {
        // event-stream@3.3.6 is known malicious (CVE-2018-16484)
        let client = reqwest::Client::new();
        let pkg = make_pkg("event-stream", "3.3.6", Ecosystem::Npm);
        let result = check(&client, &pkg).await;
        assert!(result.is_blocking(), "expected Block, got {result:?}");
    }
}
