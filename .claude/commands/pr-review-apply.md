---
description: "Analyze GitHub PR review comments and apply selected items to code"
allowed-tools: ["Read", "Edit", "MultiEdit", "Grep", "Glob", "Bash"]
---

Collect and analyze PR review comments, then apply only the selected ones to code.

## Usage

- `/pr-review-apply <PR#>` — analyze and list all actionable review comments
- `/pr-review-apply <PR#> <comment numbers>` — apply specific comments (e.g. `1` or `1,3,5`)

## Steps

### 1. Preflight

- Check `command -v gh` → guide install if missing
- Check `gh auth status` → guide login if needed
- First token must match `^[0-9]+$` — otherwise show usage and exit

### 2. Mode Decision

- **Phase 1** — PR number only → analyze and list
- **Phase 2** — additional tokens are comment numbers → plan + confirm + apply

### 3. Phase 1 — Analyze PR Review Comments

#### Collect

```bash
gh pr view <PR#> --json title,body,comments,reviews
gh pr diff <PR#>
gh api repos/{owner}/{repo}/pulls/<PR#>/comments
gh api repos/{owner}/{repo}/pulls/<PR#>/reviews
```

#### Filter

**Exclude**: LGTM, 👍, thank-you messages, resolved threads, questions with no change request

**Include**: code change requests, bug/security/performance concerns, convention violations

#### Output Format (English)

```markdown
## PR #<N> — Review Comment Analysis

Found N actionable comments.

### [1] <priority> — `crates/gard-pkg/src/tier3_analyzer.rs:84`

- **Reviewer**: @<username>
- **Comment**: <summary>
- **Recommend applying**: Yes/No — <reason>
```

End with: `Which comments would you like to apply? (e.g. "1" or "1,3,5")`

### 4. Phase 2 — Apply Comments

For each selected comment: show Before/After plan → wait for user confirmation → apply with `Edit`.

#### Verification

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

- If new errors appear in changed files → fix and re-run before reporting done

#### Final Report

Table of applied changes + "Run `/commit` separately to commit."

## Conventions

- All output in **English**
- Follow `.claude/rules/rust-conventions.md`
- No unwrap in non-test code; use `?` or return `TierResult::Error`

---

$ARGUMENTS
