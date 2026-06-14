# Rust Conventions — gard

## Workspace Structure

```
crates/
  gard-cli/     — CLI entrypoint (clap), no business logic
  gard-core/    — shared types, Config, Manifest (no network deps)
  gard-pkg/     — T1/T2/T3 detection engine
  gard-git/     — hook install + CI workflow generation
  gard-report/  — terminal / JSON / SARIF output
```

Dependency direction: `cli → {pkg, git, report} → core`. No cycles.

## Error Handling

- Use `anyhow::Result` at crate boundaries and in `gard-cli`
- Use `thiserror` for typed errors inside library crates
- Never use `.unwrap()` or `.expect()` in non-test code — propagate with `?`
- Network errors in T1/T2 → return `TierResult::Error { message }` (caller handles as WARN)

## Code Style

- `cargo fmt` enforced (rustfmt.toml: max_width = 100)
- Zero `clippy -D warnings` — all warnings are errors in CI
- No unused imports or variables — remove, don't comment out
- Comments only when WHY is non-obvious — no "what" comments
- No emoji in code or comments (only in terminal output strings)

## Testing

- Unit tests in the same file (`#[cfg(test)] mod tests { ... }`)
- Integration tests in `crates/*/tests/`
- Network-dependent tests must be `#[ignore]` with a `/// Run with: cargo test -- --ignored` doc comment
- Fixture-based tests preferred over mocks — use `tempfile::TempDir`
- Pattern: arrange tempdir → write fixture files → call function → assert

## Key Types (gard-core)

- `TierResult`: `Pass | Warn { reason } | Block { reason } | Skipped | Error { message }`
- `Verdict`: `Pass | Warn | Block` (final per-package decision)
- `PackageResult`: full scan output (all tier results + metadata)
- `Config`: loaded from `.gard/config.toml`, has `Default` impl
- `Manifest`: loaded from `.gard/manifest.json`

## T3 Detection Rules

- `src_crits >= 2` → `Block`
- `src_crits == 1` → `Warn`
- `script_findings` with Critical → `Block` (single is enough)
- `obfuscation_score >= cfg.tier3.obfuscation_block_score` → `Block`

## Async

- `tokio` runtime on `gard-cli` only (via `#[tokio::main]`)
- T1 + T2 run in parallel: `tokio::join!(tier1_osv::check(...), tier2_meta::check(...))`
- T3 is synchronous (CPU-bound file analysis)

## Output

- All user-facing output → `eprintln!` (stderr), keeps stdout clean for `--format json/sarif`
- `tracing::info!` / `debug!` / `trace!` for verbosity levels — never `println!` for diagnostics
- RuntimeContext determines output style — don't print ANSI in CI (`GitHubActions` context)
