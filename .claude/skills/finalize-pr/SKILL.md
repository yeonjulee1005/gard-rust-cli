---
name: finalize-pr
description: >
  Automates commit → push → GitHub PR create/update after completing work on a feature branch.
  Uses gh CLI to communicate with GitHub.
  Follows commit-convention (.claude/rules/commit-convention.md) and generates an English PR description.
  Use when: (1) committing + pushing + PR in one step, (2) writing/updating PR content,
  (3) requests like "make a PR", "push and create PR", "commit and open PR", "ship this".
  Triggers: PR, pull request, commit and push, make a PR, create PR, open PR, finalize, ship,
  push and PR, commit PR, PR 만들어, PR 올려, PR 생성
---

# Finalize PR

Commit → push → GitHub PR create/update using `gh` CLI.

## Prerequisites

- `gh` CLI installed and authenticated (`gh auth status`)
- Working on a feature branch — never commit/push directly to `develop` or `main`

## Workflow

### Step 1: Collect State

```bash
git status
git branch --show-current
git remote get-url origin
gh auth status
```

**base branch: always `develop`** (see `.claude/rules/branch-workflow.md`)

```bash
git fetch origin develop --quiet
git log origin/develop..HEAD --oneline
git diff --stat origin/develop..HEAD
```

### Step 2: Commit

Skip to Step 3 if there are no uncommitted changes.

Run full preflight per `.claude/commands/commit.md`:
1. `cargo fmt --check` (auto-fix if needed)
2. `cargo clippy -- -D warnings` (fix changed-file errors only)
3. `cargo test` (fix changed-file failures only)

Compose commit message per `.claude/rules/commit-convention.md` and confirm with user before running.
Deliver via HEREDOC.

### Step 3: Push

```bash
git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null \
  && git push \
  || git push -u origin $(git branch --show-current)
```

### Step 4: Check / Create PR

```bash
gh pr list --head "$(git branch --show-current)" --base develop --state open
```

- **PR exists** → save number, update body in Step 5
- **No PR** → write description in Step 5, then:
  ```bash
  gh pr create --base develop --title "<title>" --body "<description>"
  ```

### Step 5: PR Description (English)

Diff base: `origin/develop`

```markdown
## 변경 유형
- [ ] bug fix
- [ ] new feature
- [ ] breaking change
- [ ] documentation
- [ ] chore / refactor
- [ ] performance

## 설명
<bullet list derived from commits and diff — factual, English>

## 테스트
- [x] 기존 테스트 통과 (`cargo test`)
- [ ] 새 테스트 추가 (해당 시)
- [x] `cargo clippy` 경고 없음
- [x] `cargo fmt --check` 통과

## 관련 이슈
closes #
```

Extract before-state when needed:
```bash
git show origin/develop:crates/gard-pkg/src/tier3_analyzer.rs
```

### Step 6: Report

- PR URL
- Change summary (files changed, lines +/-)
- Confirm base branch is `develop`

## Error Handling

- **gh not installed / not authenticated**: guide `brew install gh && gh auth login`
- **push rejected**: suggest `git pull --rebase origin develop`
- **on develop/main**: stop and prompt to create a feature branch
