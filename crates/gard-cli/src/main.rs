use clap::{Parser, Subcommand};
use console::style;
use gard_core::{Config, Ecosystem, Manifest, Package, Verdict};
use std::path::{Path, PathBuf};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "gard",
    version,
    about   = "Sign what you commit. Guard what you ship.",
    long_about = None,
)]
struct Cli {
    /// Increase verbosity: -v info, -vv debug, -vvv trace
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize gard in the current repository
    Init {
        /// Preview changes without writing any files
        #[arg(long)]
        dry_run: bool,
    },

    /// Scan newly added packages (--format json/sarif for machine-readable output)
    Scan {
        #[arg(long, help = "Scan packages from lockfiles")]
        packages: bool,
        #[arg(long, default_value = "terminal", help = "Output format: terminal | json | sarif")]
        format: String,
        #[arg(long, help = "Suppress terminal output (exit code only)")]
        quiet: bool,
        /// Preview scan without updating manifest
        #[arg(long)]
        dry_run: bool,
    },

    /// Scan a specific package (e.g. lodash@4.17.21 or requests==2.28.0)
    Check {
        /// Package spec: name@version (npm/cargo) or name==version (pypi)
        package: String,
    },

    /// Show detailed scan results for a package
    Explain {
        /// Package name (must be in manifest)
        package: String,
    },

    /// Add a package to the allowlist (suppresses future warnings)
    Allow {
        /// Package name to allow
        package: String,
    },

    /// Show the current allowlist
    Allowlist,

    /// Verify manifest integrity
    Verify,

    /// Remove gard git hooks from this repository
    Uninstall,

    /// Run system diagnostics (git, hooks, network, ecosystem)
    Doctor,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Command::Init { dry_run }                           => cmd_init(dry_run).await,
        Command::Scan { packages, format, quiet, dry_run } => cmd_scan(packages, &format, quiet, dry_run).await,
        Command::Check { package }                         => cmd_check(&package).await,
        Command::Explain { package }                       => cmd_explain(&package).await,
        Command::Allow { package }                         => cmd_allow(&package).await,
        Command::Allowlist                                 => cmd_allowlist().await,
        Command::Verify                                    => cmd_verify().await,
        Command::Uninstall                                 => cmd_uninstall().await,
        Command::Doctor                                    => cmd_doctor().await,
    }
}

fn init_tracing(verbose: u8) {
    use tracing_subscriber::{fmt, EnvFilter};
    let default_level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("gard={default_level}")));
    fmt()
        .with_env_filter(filter)
        .with_target(verbose >= 2)
        .with_writer(std::io::stderr)
        .init();
}

// ── gard init ─────────────────────────────────────────────────────────────────

async fn cmd_init(dry_run: bool) -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    let gard_dir  = repo_root.join(".gard");

    tracing::info!("repo root → {}", repo_root.display());

    if dry_run {
        let ci = gard_git::ci::CiProvider::detect(&repo_root);
        eprintln!();
        if !gard_dir.join("config.toml").exists() {
            eprintln!("  {}  would create → .gard/config.toml", style("[dry-run]").dim());
        }
        if !gard_dir.join("manifest.json").exists() {
            eprintln!("  {}  would create → .gard/manifest.json", style("[dry-run]").dim());
        }
        eprintln!("  {}  would write  → .git/hooks/pre-push", style("[dry-run]").dim());
        if let Some(wf) = ci_workflow_path(&repo_root, &ci) {
            eprintln!("  {}  would create → {}", style("[dry-run]").dim(), wf.display());
        }
        eprintln!();
        eprintln!("  no files were written. remove --dry-run to apply.");
        eprintln!();
        return Ok(());
    }

    std::fs::create_dir_all(&gard_dir)?;

    let config_path = gard_dir.join("config.toml");
    if !config_path.exists() {
        tracing::info!("writing config → {}", config_path.display());
        std::fs::write(&config_path, Config::default().to_toml()?)?;
    }

    let manifest_path = gard_dir.join("manifest.json");
    if !manifest_path.exists() {
        tracing::info!("writing manifest → {}", manifest_path.display());
        Manifest::new(detect_repo_url(&repo_root)).save(&manifest_path)?;
    }

    tracing::info!("installing git hook");
    gard_git::hook::install(&repo_root)?;

    let ci          = gard_git::ci::CiProvider::detect(&repo_root);
    let ci_name     = ci.name();
    let ci_detected = ci_name.is_some();
    tracing::info!("CI detected: {:?}", ci_name);
    ci.generate_config(&repo_root)?;

    gard_report::print_init_success(ci_detected, ci_name);
    Ok(())
}

fn ci_workflow_path(repo_root: &Path, ci: &gard_git::ci::CiProvider) -> Option<PathBuf> {
    use gard_git::ci::CiProvider;
    match ci {
        CiProvider::GitHubActions => Some(repo_root.join(".github/workflows/gard.yml")),
        CiProvider::GitLabCI      => Some(repo_root.join(".gitlab-ci.yml")),
        CiProvider::Bitbucket     => Some(repo_root.join("bitbucket-pipelines.yml")),
        CiProvider::Jenkins       => Some(repo_root.join("Jenkinsfile.gard")),
        CiProvider::Unknown       => None,
    }
}

// ── gard scan ─────────────────────────────────────────────────────────────────

async fn cmd_scan(packages: bool, format: &str, quiet: bool, dry_run: bool) -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    let gard_dir  = repo_root.join(".gard");

    if !gard_dir.exists() {
        eprintln!("{} gard not initialized — run `gard init` first.",
            style("error:").red().bold());
        std::process::exit(1);
    }

    let cfg           = load_config(&gard_dir)?;
    let manifest_path = gard_dir.join("manifest.json");
    let mut manifest  = load_or_new_manifest(&manifest_path)?;

    let new_pkgs = if packages {
        gard_git::diff::detect_new_packages(&repo_root)?
    } else {
        vec![]
    };

    let new_pkgs: Vec<_> = new_pkgs.into_iter()
        .filter(|p| !cfg.allowlist.packages.contains(&p.name))
        .collect();

    tracing::debug!("manifest diff: {} new packages detected", new_pkgs.len());

    if new_pkgs.is_empty() {
        if !quiet {
            eprintln!("  {}  no new packages to scan.", style("gard").green().bold());
        }
        return Ok(());
    }

    let client   = reqwest::Client::new();
    let ctx      = gard_report::detect_runtime_context();
    let added_by = whoami();
    let mut results = Vec::new();

    for pkg in &new_pkgs {
        tracing::info!("scanning {}@{}", pkg.name, pkg.version);
        let pkg_path = resolve_pkg_path(&repo_root, pkg);
        let result = gard_pkg::scan_package(&client, pkg, pkg_path.as_deref(), &cfg).await;
        tracing::debug!("  verdict: {:?}", result.verdict);
        if !dry_run {
            manifest.upsert(&result, added_by.clone());
        }
        results.push(result);
    }

    if dry_run {
        eprintln!("  {} scan complete — manifest not updated", style("[dry-run]").dim());
    } else {
        manifest.save(&manifest_path)?;
    }

    if quiet {
        // exit code only
    } else {
        match format {
            "json"  => println!("{}", gard_report::json::render(&results)?),
            "sarif" => println!("{}", gard_report::sarif::render(&results)?),
            _       => gard_report::print_results(&results, &ctx),
        }
    }

    if results.iter().any(|r| r.verdict == Verdict::Block) {
        std::process::exit(1);
    }
    Ok(())
}

// ── gard doctor ───────────────────────────────────────────────────────────────

async fn cmd_doctor() -> anyhow::Result<()> {
    let repo_root  = std::env::current_dir()?;
    let gard_dir   = repo_root.join(".gard");
    let mut issues = 0usize;

    eprintln!();
    eprintln!("  {}  v{}  — system diagnostics",
        style("gard").green().bold(),
        env!("CARGO_PKG_VERSION"));
    eprintln!();

    // ── environment ──────────────────────────────────────────────────────────
    eprintln!("  environment");
    doctor_row(repo_root.join(".git").exists(),
        "git repository", &repo_root.display().to_string(), &mut issues);
    doctor_row(gard_dir.join("config.toml").exists(),
        "gard config", ".gard/config.toml", &mut issues);

    let manifest_path = gard_dir.join("manifest.json");
    let manifest_info = if manifest_path.exists() {
        let m = Manifest::load(&manifest_path).unwrap_or_else(|_| Manifest::new(None));
        format!(".gard/manifest.json  ({} packages)", m.packages.len())
    } else {
        ".gard/manifest.json  (not found)".to_string()
    };
    doctor_row(manifest_path.exists(), "gard manifest", &manifest_info, &mut issues);
    eprintln!();

    // ── git hooks ─────────────────────────────────────────────────────────────
    eprintln!("  git hooks");
    let hook_path   = repo_root.join(".git").join("hooks").join("pre-push");
    let hook_exists = hook_path.exists();
    doctor_row(hook_exists, "pre-push hook", "installed & executable", &mut issues);

    if hook_exists {
        let content  = std::fs::read_to_string(&hook_path).unwrap_or_default();
        let has_gard = content.contains("gard");
        if has_gard {
            eprintln!("  {}  hook content           gard block present", style("✓").green());
        } else {
            eprintln!("  {}  hook content           gard block NOT found → run: gard uninstall && gard init",
                style("✗").red());
            issues += 1;
        }
    }
    eprintln!();

    // ── network connectivity ──────────────────────────────────────────────────
    eprintln!("  network connectivity");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .user_agent("gard-doctor/0.1")
        .build()?;

    let endpoints: &[(&str, &str)] = &[
        ("OSV API",      "https://api.osv.dev/v1/query"),
        ("npm registry", "https://registry.npmjs.org/lodash"),
        ("PyPI",         "https://pypi.org/pypi/requests/json"),
        ("crates.io",    "https://crates.io/api/v1/crates/serde"),
    ];

    for (label, url) in endpoints {
        let t0 = std::time::Instant::now();
        match client.get(*url).send().await {
            Ok(resp) => {
                let ms  = t0.elapsed().as_millis();
                let ok  = resp.status().is_success() || resp.status().as_u16() == 404;
                if ok {
                    eprintln!("  {}  {:<20} {}  ({ms}ms)",
                        style("✓").green(), label, style("OK").dim());
                } else {
                    eprintln!("  {}  {:<20} {}",
                        style("✗").red(), label, style(resp.status().to_string()).red());
                    issues += 1;
                }
            }
            Err(e) => {
                let kind = if e.is_timeout() { "timeout" } else { "error" };
                eprintln!("  {}  {:<20} {}",
                    style("✗").red(), label, style(format!("{kind}: {e}")).red());
                issues += 1;
            }
        }
    }
    eprintln!();

    // ── ecosystem detection ────────────────────────────────────────────────────
    eprintln!("  ecosystem detection (project root)");
    for (name, file) in [("npm", "package.json"), ("Python", "requirements.txt"), ("Cargo", "Cargo.toml")] {
        if repo_root.join(file).exists() {
            eprintln!("  {}  {:<10}  {} found", style("✓").green(), name, file);
        } else {
            eprintln!("  {}  {:<10}  {} not found", style("–").dim(), name, file);
        }
    }

    // ── allowlist ─────────────────────────────────────────────────────────────
    if let Ok(cfg) = load_config(&gard_dir) {
        if !cfg.allowlist.packages.is_empty() {
            eprintln!();
            eprintln!("  allowlist");
            eprintln!("  {}  {} package{}: {}",
                style("ℹ").cyan(),
                cfg.allowlist.packages.len(),
                if cfg.allowlist.packages.len() == 1 { "" } else { "s" },
                cfg.allowlist.packages.join(", "));
        }
    }

    eprintln!();
    if issues == 0 {
        eprintln!("  {} all checks passed.", style("✓").green().bold());
    } else {
        eprintln!("  {} issue{} found. run `gard doctor -v` for details.",
            issues, if issues == 1 { "" } else { "s" });
    }
    eprintln!();
    Ok(())
}

fn doctor_row(ok: bool, label: &str, detail: &str, issues: &mut usize) {
    if ok {
        eprintln!("  {}  {:<24} {}", style("✓").green(), label, detail);
    } else {
        eprintln!("  {}  {:<24} {}  → run: gard init",
            style("✗").red(), label, style("not found").red());
        *issues += 1;
    }
}

// ── gard check ───────────────────────────────────────────────────────────────

async fn cmd_check(spec: &str) -> anyhow::Result<()> {
    let (name, version, ecosystem) = parse_package_spec(spec);
    tracing::info!("check {} {} ({})", name, version, ecosystem);
    let pkg    = Package::new(name, version, ecosystem);
    let cfg    = load_config_or_default(&std::env::current_dir()?.join(".gard"));
    let client = reqwest::Client::new();

    let result = gard_pkg::scan_package(&client, &pkg, None, &cfg).await;
    let ctx    = gard_report::detect_runtime_context();
    gard_report::print_results(&[result], &ctx);
    Ok(())
}

// ── gard explain ─────────────────────────────────────────────────────────────

async fn cmd_explain(package: &str) -> anyhow::Result<()> {
    let manifest = load_manifest_or_exit()?;

    match manifest.packages.iter().find(|e| e.name == package) {
        None => {
            eprintln!("  Package '{}' not found in manifest.", package);
            eprintln!("  Run `gard check {}@<version>` to scan it.", package);
            std::process::exit(1);
        }
        Some(e) => {
            eprintln!();
            eprintln!("  {}  {}", style("gard explain").green().bold(), e.name);
            eprintln!();
            eprintln!("  Package  : {}@{} ({})", e.name, e.version, e.ecosystem);
            eprintln!("  Checked  : {}", e.checked_at);
            eprintln!("  Verdict  : {}", style(&e.final_verdict).bold());
            eprintln!();
            eprintln!("  Tier 1   : {}{}",
                e.tier1.result,
                e.tier1.reason.as_deref().map(|r| format!("  — {r}")).unwrap_or_default());
            eprintln!("  Tier 2   : {} (score: {})",
                e.tier2.result, e.tier2.score);
            if let Some(age) = e.tier2.age_days {
                eprintln!("             age: {} days", age);
            }
            if let Some(dl) = e.tier2.weekly_downloads {
                eprintln!("             downloads: {}", dl);
            }
            eprintln!("  Tier 3   : {}{}",
                e.tier3.result,
                e.tier3.reason.as_deref().map(|r| format!("  — {r}")).unwrap_or_default());
            if let Some(tool) = &e.ai_tool {
                eprintln!();
                eprintln!("  AI tool  : {}", tool);
            }
            eprintln!();
        }
    }
    Ok(())
}

// ── gard allow ───────────────────────────────────────────────────────────────

async fn cmd_allow(package: &str) -> anyhow::Result<()> {
    let repo_root   = std::env::current_dir()?;
    let config_path = repo_root.join(".gard").join("config.toml");
    let mut cfg     = load_config_or_default(&repo_root.join(".gard"));

    if cfg.allowlist.packages.contains(&package.to_string()) {
        eprintln!("  '{}' is already in the allowlist.", package);
        return Ok(());
    }
    cfg.allowlist.packages.push(package.to_string());
    std::fs::create_dir_all(config_path.parent().unwrap())?;
    std::fs::write(&config_path, cfg.to_toml()?)?;
    eprintln!("  {} '{}' added to allowlist.", style("✓").green(), package);
    Ok(())
}

// ── gard allowlist ───────────────────────────────────────────────────────────

async fn cmd_allowlist() -> anyhow::Result<()> {
    let cfg = load_config_or_default(&std::env::current_dir()?.join(".gard"));

    if cfg.allowlist.packages.is_empty() {
        eprintln!("  allowlist is empty.");
    } else {
        eprintln!("  {} allowed package{}:",
            cfg.allowlist.packages.len(),
            if cfg.allowlist.packages.len() == 1 { "" } else { "s" });
        for pkg in &cfg.allowlist.packages {
            eprintln!("    - {}", pkg);
        }
    }
    Ok(())
}

// ── gard verify ──────────────────────────────────────────────────────────────

async fn cmd_verify() -> anyhow::Result<()> {
    let manifest = load_manifest_or_exit()?;
    let total   = manifest.packages.len();
    let blocked = manifest.packages.iter().filter(|e| e.final_verdict == "BLOCK").count();
    let warned  = manifest.packages.iter().filter(|e| e.final_verdict == "WARN").count();
    let passed  = manifest.packages.iter().filter(|e| e.final_verdict == "PASS").count();

    eprintln!();
    eprintln!("  {}  manifest integrity", style("gard").green().bold());
    eprintln!();
    eprintln!("  {} total packages in manifest", total);
    eprintln!("  {} passed",  style(passed.to_string()).green());
    if warned  > 0 { eprintln!("  {} warned",  style(warned.to_string()).yellow()); }
    if blocked > 0 { eprintln!("  {} blocked", style(blocked.to_string()).red()); }
    eprintln!();

    if blocked > 0 {
        eprintln!("  {} manifest contains blocked packages.", style("✗").red());
        std::process::exit(1);
    }
    eprintln!("  {} manifest verified.", style("✓").green());
    eprintln!();
    Ok(())
}

// ── gard uninstall ────────────────────────────────────────────────────────────

async fn cmd_uninstall() -> anyhow::Result<()> {
    let repo_root = std::env::current_dir()?;
    gard_git::hook::uninstall(&repo_root)?;
    eprintln!("  {} gard hooks removed.", style("✓").green());
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn load_config(gard_dir: &Path) -> anyhow::Result<Config> {
    let path = gard_dir.join("config.toml");
    if path.exists() {
        Ok(Config::from_toml(&std::fs::read_to_string(path)?)?)
    } else {
        Ok(Config::default())
    }
}

fn load_config_or_default(gard_dir: &Path) -> Config {
    load_config(gard_dir).unwrap_or_default()
}

fn load_or_new_manifest(path: &Path) -> anyhow::Result<Manifest> {
    if path.exists() { Manifest::load(path) } else { Ok(Manifest::new(None)) }
}

fn load_manifest_or_exit() -> anyhow::Result<Manifest> {
    let path = std::env::current_dir()?.join(".gard").join("manifest.json");
    if !path.exists() {
        eprintln!("{} no manifest found — run `gard init` first.",
            style("error:").red().bold());
        std::process::exit(1);
    }
    Ok(Manifest::load(&path)?)
}

fn resolve_pkg_path(repo_root: &Path, pkg: &Package) -> Option<PathBuf> {
    match &pkg.ecosystem {
        Ecosystem::Npm => {
            let p = repo_root.join("node_modules").join(&pkg.name);
            if p.exists() { Some(p) } else { None }
        }
        _ => None,
    }
}

fn detect_repo_url(repo_root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

fn whoami() -> Option<String> {
    std::env::var("USER").ok().or_else(|| std::env::var("USERNAME").ok())
}

fn parse_package_spec(spec: &str) -> (String, String, Ecosystem) {
    if let Some(pos) = spec[1..].find('@').map(|p| p + 1) {
        return (spec[..pos].to_string(), spec[pos + 1..].to_string(), Ecosystem::Npm);
    }
    if let Some(pos) = spec.find("==") {
        return (spec[..pos].to_string(), spec[pos + 2..].to_string(), Ecosystem::PyPI);
    }
    let eco = if Path::new("requirements.txt").exists() {
        Ecosystem::PyPI
    } else if Path::new("Cargo.toml").exists() {
        Ecosystem::Cargo
    } else {
        Ecosystem::Npm
    };
    (spec.to_string(), "latest".to_string(), eco)
}
