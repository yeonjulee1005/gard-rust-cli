# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## [Unreleased]

## [0.1.0] - 2026-06-14

### Added

- `gard init` — one-command setup: git hooks + CI workflow generation
- `gard scan --packages` — 3-tier package scanning pipeline
- `gard check <pkg>` — on-demand single package check
- `gard explain <pkg>` — detailed manifest entry display
- `gard allow <pkg>` / `gard allowlist` — allowlist management
- `gard verify` — manifest integrity check
- `gard uninstall` — hook removal
- `gard doctor` — full system diagnostics (git, hooks, network, ecosystem)
- `-v / -vv / -vvv` global verbosity flags via `tracing`
- `--dry-run` flag for `init` and `scan`
- Tier 1: OSV vulnerability database lookup (Google OSV API)
- Tier 2: Package metadata risk scoring (age + download count)
- Tier 3: Source code pattern analysis (postinstall scripts, high-risk regex)
- SARIF 2.1.0 output for GitHub Security tab integration
- JSON output for CI pipeline consumption
- RuntimeContext detection: Standard / ClaudeCode / Cursor / GitHubActions
- CI auto-detection: GitHub Actions / GitLab CI / Bitbucket / Jenkins
- Allowlist: suppress warnings for trusted packages
- Manifest signing: `.gard/manifest.json` with AI tool attribution
- Ecosystems: npm / PyPI / crates.io

<!-- next-url -->
[Unreleased]: https://github.com/dewdew/gard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dewdew/gard/releases/tag/v0.1.0
