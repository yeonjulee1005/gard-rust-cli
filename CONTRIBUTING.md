# Contributing to gard

Thank you for your interest in contributing!

## Development Setup

**Prerequisites**: Rust 1.75+ ([rustup](https://rustup.rs))

```bash
git clone https://github.com/dewdew/gard
cd gard
cargo build          # debug build
cargo test           # run all tests
cargo clippy         # lint
cargo fmt --check    # format check
```

## Running Tests

```bash
# All tests (no network required)
cargo test

# With network (OSV + registry APIs)
cargo test -- --ignored

# Single crate
cargo test -p gard-pkg

# Specific verbosity
cargo run --bin gard -- init -vv
RUST_LOG=gard=trace cargo run --bin gard -- check lodash@4.17.21
```

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add Maven ecosystem support
fix: handle missing package.json gracefully
docs: update README installation steps
chore: bump reqwest to 0.13
test: add T3 fixture for obfuscated packages
```

Types: `feat` / `fix` / `docs` / `chore` / `test` / `refactor` / `perf`

## Pull Request Process

1. Open an issue first for non-trivial changes
2. Fork → branch (`feat/my-feature`) → PR against `main`
3. All CI checks must pass (fmt → clippy → test on ubuntu/macos/windows)
4. Add tests for new behaviour
5. Update `CHANGELOG.md` under `[Unreleased]`

## Project Structure

```
crates/
  gard-cli/       # CLI entrypoint (clap)
  gard-core/      # shared types, config, manifest
  gard-pkg/       # 3-tier detection engine (T1/T2/T3)
  gard-git/       # hook install + CI generation
  gard-report/    # terminal / JSON / SARIF output
```

## Adding Detection Patterns

Tier 3 patterns live in `crates/gard-pkg/src/tier3_analyzer.rs` — the
`CRITICAL_COMBOS` and `DANGEROUS_SCRIPTS` constants. Add a fixture test in
`crates/gard-pkg/tests/integration.rs` for every new pattern.

## License

By contributing you agree that your contributions will be licensed under the MIT License.
