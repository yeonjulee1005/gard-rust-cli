---
description: "GitHub PR / branch / file code review (optionally post as PR comment)"
allowed-tools: ["Read", "Grep", "Bash"]
---

Perform a code review. Output to chat by default; optionally post to GitHub PR as a comment.

## Usage

- `/review` — review current branch vs `develop` (works without a PR)
- `/review <PR#>` — review that PR's changes
- `/review <PR#> --comment` — review and post to GitHub PR as comment
- `/review <file-path>` — review a specific file
- `/review --dry-run` — with `--comment`, output to chat only (no post)

## Steps

### 1. Parse Arguments

Priority:
1. Strip `--comment` and `--dry-run` flags
2. Remaining token matches `^[0-9]+$` → **PR mode**
3. Remaining token is an existing file/directory → **file mode**
4. Empty → **local branch mode**

If `--comment` is set but no PR number, stop:
> "`--comment` requires a PR number. Example: `/review 123 --comment`"

### 2. Preflight (PR mode or `--comment`)

- Check `command -v gh` → guide install if missing
- Check `gh auth status` → guide login if needed

### 3. Collect Changes

#### PR mode
```bash
gh pr view <#> --json title,body,baseRefName,headRefName,author
gh pr diff <#>
```
Warn if base is not `develop`.

#### Local branch mode
```bash
git fetch origin develop --quiet
git log origin/develop..HEAD --oneline
git diff --stat origin/develop...HEAD
git diff origin/develop...HEAD
```
If no diff: "No changes vs `develop`." then exit.

#### File mode
Read the file with `Read` tool.

### 4. Review Perspective

Analyze on these dimensions (skip if not applicable):

1. **Correctness** — logic bugs, off-by-one, incorrect tier decision thresholds
2. **Security** — this is a security tool; false negatives in T1/T2/T3 detection are regressions. Check regex patterns, OSV query construction, path traversal in T3 file walker
3. **Rust best practices** — `.claude/rules/rust-conventions.md`: no unwrap in non-test code, anyhow at boundaries, thiserror for library errors
4. **Performance** — T1+T2 must use `tokio::join!` (parallel), T3 skips files >100KB and minified files
5. **Test coverage** — new detection patterns must have fixture tests in `crates/gard-pkg/tests/integration.rs`
6. **Clippy / fmt** — flag anything that would fail `cargo clippy -D warnings`
7. **API contracts** — `TierResult`, `Verdict`, `PackageResult` must match across crates
8. **Output consistency** — stderr for UX, stdout for json/sarif; no ANSI in CI context

### 5. Write Review (English)

```markdown
## 📋 Summary
(1-3 lines: what changed, from a user/security perspective)

## ✅ Good Parts
(what was done well)

## 🔍 Suggestions
- `crate/src/file.rs:42` — specific suggestion (include code snippet if helpful)

## 🚨 Issues
(potential bugs, security regressions, missing test coverage)

## 🎯 Verdict
Approve / Request changes / Needs discussion — one-line reason
```

### 6. Output / Post

- If `--dry-run` or no `--comment` → print to chat only
- If `--comment` and PR number available:
  ```bash
  gh pr comment <#> --body "<review markdown>"
  ```

---

$ARGUMENTS
