---
description: "Analyze GitHub PR changes and auto-generate/register the PR description"
allowed-tools: ["Read", "Bash"]
---

Automatically generate and register a PR description based on change analysis.

## Usage

- `/pr-description` — find the open PR on the current branch (`develop` base) and auto-update description
- `/pr-description <PR number>` — update a specific PR description
- `/pr-description --dry-run` — output draft to chat only, do not register

## Steps

### 1. Preflight

- Check `command -v gh` → if missing, guide `brew install gh && gh auth login` then exit
- Check `gh auth status` → if not authenticated, guide `gh auth login` then exit

### 2. Determine Target PR

- If argument is a number (`^[0-9]+$`) → use as PR number
- If empty or only `--dry-run`:
  ```bash
  gh pr list --head "$(git branch --show-current)" --base develop --state open --json number,url
  ```
  - If no PR found: "No open PR targeting `develop` on this branch. Use `/push` to create one, or pass a PR number."

### 3. Analyze Changes

diff base = **PR's actual base branch** (`develop` expected). Never hardcode `main`.

```bash
TARGET=$(gh pr view <PR#> --json baseRefName -q .baseRefName)
git fetch origin "$TARGET" --quiet
git log "origin/$TARGET..HEAD" --oneline
git diff --stat "origin/$TARGET...HEAD"
git diff "origin/$TARGET...HEAD"
```

File type classification:
- `crates/gard-cli/` — CLI commands, argument parsing
- `crates/gard-core/` — shared types, config, manifest
- `crates/gard-pkg/` — T1/T2/T3 detection engine
- `crates/gard-git/` — hook install, CI generation
- `crates/gard-report/` — output rendering
- `Cargo.toml`, `Cargo.lock` — dependency changes
- `.github/workflows/` — CI/CD
- `*.md` — documentation

### 4. Generate Description (English)

Follow the structure from `.github/PULL_REQUEST_TEMPLATE.md`:

```markdown
## 변경 유형
- [ ] bug fix
- [x] <check the relevant types>

## 설명
<What and why — derived from diff analysis. 3-5 bullet points.>

## 테스트
- [x] 기존 테스트 통과 (`cargo test`)
- [ ] 새 테스트 추가 (해당 시)
- [x] `cargo clippy` 경고 없음
- [x] `cargo fmt --check` 통과

## 관련 이슈
closes #
```

Write the "설명" section in **English**. Keep it factual and concise.

### 5. Apply

- If `--dry-run`: output markdown to chat only, then exit
- Otherwise:
  ```bash
  gh pr edit <PR#> --body-file -
  ```
  (pass body via HEREDOC)

### 6. Quality Check

- All changed crates are mentioned
- Test checklist accurately reflects what was run
- base branch is `develop`

---

$ARGUMENTS
