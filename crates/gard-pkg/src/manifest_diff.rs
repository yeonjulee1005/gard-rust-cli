use gard_core::{Ecosystem, Manifest, Package};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ── Public API ────────────────────────────────────────────────────────────────

/// Return packages present in the current lockfiles but not yet in the manifest.
pub fn find_new_packages(repo_root: &Path, manifest: &Manifest) -> anyhow::Result<Vec<Package>> {
    let mut current = Vec::new();
    if let Ok(pkgs) = parse_npm_packages(repo_root) {
        current.extend(pkgs);
    }
    if let Ok(pkgs) = parse_pypi_packages(repo_root) {
        current.extend(pkgs);
    }
    if let Ok(pkgs) = parse_cargo_packages(repo_root) {
        current.extend(pkgs);
    }

    let checked: HashSet<(String, String)> = manifest
        .packages
        .iter()
        .map(|e| (e.name.clone(), e.version.clone()))
        .collect();

    Ok(current
        .into_iter()
        .filter(|p| !checked.contains(&(p.name.clone(), p.version.clone())))
        .collect())
}

/// Simple slice diff: packages in `after` not present in `before`.
pub fn diff_packages(before: &[Package], after: &[Package]) -> Vec<Package> {
    let before_set: HashSet<(&str, &str)> = before
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str()))
        .collect();
    after
        .iter()
        .filter(|p| !before_set.contains(&(p.name.as_str(), p.version.as_str())))
        .cloned()
        .collect()
}

// ── npm (package.json) ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PackageJson {
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

pub fn parse_npm_packages(repo_root: &Path) -> anyhow::Result<Vec<Package>> {
    let path = repo_root.join("package.json");
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let pj: PackageJson = serde_json::from_str(&content)?;

    let mut packages = Vec::new();
    for deps in [pj.dependencies, pj.dev_dependencies].into_iter().flatten() {
        for (name, spec) in deps {
            packages.push(Package::new(name, normalize_version(&spec), Ecosystem::Npm));
        }
    }
    Ok(packages)
}

// ── PyPI (requirements.txt) ───────────────────────────────────────────────────

pub fn parse_pypi_packages(repo_root: &Path) -> anyhow::Result<Vec<Package>> {
    let path = repo_root.join("requirements.txt");
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter_map(parse_requirement_line)
        .collect())
}

fn parse_requirement_line(line: &str) -> Option<Package> {
    let line = line.trim();
    if line.starts_with('-') {
        return None; // skip -r, --index-url, etc.
    }
    // Split on first operator (==, ~=, >=, !=, >)
    let (name, version) = if let Some(p) = line.find("==") {
        (&line[..p], line[p + 2..].to_string())
    } else if let Some(p) = line.find("~=") {
        (&line[..p], line[p + 2..].to_string())
    } else if let Some(p) = line.find(">=") {
        (&line[..p], line[p + 2..].to_string())
    } else if let Some(p) = line.find('>') {
        (&line[..p], line[p + 1..].to_string())
    } else {
        (line, "latest".to_string())
    };
    let name = name.trim();
    let version = version
        .split(',')
        .next()
        .unwrap_or("latest")
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    Some(Package::new(name, version, Ecosystem::PyPI))
}

// ── Cargo (Cargo.lock preferred, Cargo.toml fallback) ────────────────────────

#[derive(Deserialize)]
struct CargoToml {
    dependencies: Option<HashMap<String, toml::Value>>,
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: Option<HashMap<String, toml::Value>>,
}

#[derive(Deserialize)]
struct CargoLock {
    package: Option<Vec<CargoLockPackage>>,
}

#[derive(Deserialize)]
struct CargoLockPackage {
    name: String,
    version: String,
}

pub fn parse_cargo_packages(repo_root: &Path) -> anyhow::Result<Vec<Package>> {
    let lock_path = repo_root.join("Cargo.lock");
    if lock_path.exists() {
        let content = std::fs::read_to_string(&lock_path)?;
        let lock: CargoLock = toml::from_str(&content)?;
        return Ok(lock
            .package
            .unwrap_or_default()
            .into_iter()
            .map(|p| Package::new(p.name, p.version, Ecosystem::Cargo))
            .collect());
    }

    let toml_path = repo_root.join("Cargo.toml");
    if !toml_path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&toml_path)?;
    let cargo: CargoToml = toml::from_str(&content)?;

    let mut packages = Vec::new();
    for deps in [cargo.dependencies, cargo.dev_dependencies]
        .into_iter()
        .flatten()
    {
        for (name, val) in deps {
            packages.push(Package::new(
                name,
                extract_cargo_version(&val),
                Ecosystem::Cargo,
            ));
        }
    }
    Ok(packages)
}

fn extract_cargo_version(val: &toml::Value) -> String {
    match val {
        toml::Value::String(s) => normalize_version(s),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .map(normalize_version)
            .unwrap_or_else(|| "0.0.0".into()),
        _ => "0.0.0".into(),
    }
}

/// Strip semver operators (^, ~, >=, etc.) and return the base version.
fn normalize_version(spec: &str) -> String {
    let stripped = spec
        .trim()
        .trim_start_matches(['^', '~', '>', '<', '=', '!']);
    let version = stripped.split([',', ' ']).next().unwrap_or("").trim();
    if version.is_empty() || version == "*" {
        "latest".to_string()
    } else {
        version.to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_npm_package_json() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
            "dependencies": { "lodash": "^4.17.21", "axios": "~1.7.0" },
            "devDependencies": { "jest": "29.0.0" }
        }"#,
        )
        .unwrap();
        let pkgs = parse_npm_packages(dir.path()).unwrap();
        assert_eq!(pkgs.len(), 3);
        assert!(pkgs
            .iter()
            .any(|p| p.name == "lodash" && p.version == "4.17.21"));
        assert!(pkgs
            .iter()
            .any(|p| p.name == "axios" && p.version == "1.7.0"));
        assert!(pkgs
            .iter()
            .any(|p| p.name == "jest" && p.version == "29.0.0"));
    }

    #[test]
    fn parse_requirements_txt() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("requirements.txt"),
            "requests==2.28.0\ndjango>=4.0\nflask~=2.0.0\n# comment\n\n",
        )
        .unwrap();
        let pkgs = parse_pypi_packages(dir.path()).unwrap();
        assert_eq!(pkgs.len(), 3);
        assert!(pkgs
            .iter()
            .any(|p| p.name == "requests" && p.version == "2.28.0"));
        assert!(pkgs
            .iter()
            .any(|p| p.name == "django" && p.version == "4.0"));
        assert!(pkgs
            .iter()
            .any(|p| p.name == "flask" && p.version == "2.0.0"));
    }

    #[test]
    fn diff_finds_added_packages() {
        let before = vec![Package::new("lodash", "4.17.21", Ecosystem::Npm)];
        let after = vec![
            Package::new("lodash", "4.17.21", Ecosystem::Npm),
            Package::new("axios", "1.7.0", Ecosystem::Npm),
        ];
        let diff = diff_packages(&before, &after);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].name, "axios");
    }

    #[test]
    fn diff_version_bump_is_new() {
        let before = vec![Package::new("express", "4.17.0", Ecosystem::Npm)];
        let after = vec![Package::new("express", "4.18.0", Ecosystem::Npm)];
        let diff = diff_packages(&before, &after);
        assert_eq!(diff.len(), 1);
        assert_eq!(diff[0].version, "4.18.0");
    }

    #[test]
    fn normalize_strips_operators() {
        assert_eq!(normalize_version("^4.17.21"), "4.17.21");
        assert_eq!(normalize_version("~1.7.0"), "1.7.0");
        assert_eq!(normalize_version(">=2.0.0"), "2.0.0");
        assert_eq!(normalize_version("1.0.0"), "1.0.0");
        assert_eq!(normalize_version("*"), "latest");
    }
}
