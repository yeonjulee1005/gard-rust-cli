# Commit Convention

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).

**Format**: `<type>(<scope>): <message>`

Scope is optional but recommended for this workspace.

## Types

| Type | When to use |
|------|-------------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `chore` | Build, dependencies, tooling |
| `test` | Adding or fixing tests |
| `refactor` | Code improvement without behavior change |
| `perf` | Performance improvement |
| `security` | Security-related fix or hardening |
| `ci` | CI/CD workflow changes |

## Scopes

| Scope | Crate / Area |
|-------|-------------|
| `cli` | `gard-cli` |
| `core` | `gard-core` |
| `pkg` | `gard-pkg` |
| `git` | `gard-git` |
| `report` | `gard-report` |
| `t1` | Tier 1 OSV logic |
| `t2` | Tier 2 metadata logic |
| `t3` | Tier 3 source analysis logic |
| `deps` | Cargo.toml dependency changes |

## Rules

- Message in **English**, imperative mood ("add", "fix", "update" — not "added", "fixes")
- No period at the end
- Keep subject line under 72 characters
- Breaking changes: append `!` after scope — `feat(core)!: rename Verdict variants`

## Examples

```
feat(pkg): add parallel T1+T2 execution with tokio::join
fix(t3): handle missing package.json without panic
docs: add gard doctor usage to README
chore(deps): bump reqwest to 0.13
test(t3): add fixture for obfuscated hex-escape pattern
security(t1): block packages with CRITICAL OSV severity
ci: add stale issue bot workflow
refactor(cli): extract doctor_row into shared helper
```
