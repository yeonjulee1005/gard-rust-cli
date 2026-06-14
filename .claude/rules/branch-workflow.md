# Branch Workflow

## Branch Structure

```
feature/*  →  develop  →  main
```

| Branch | Purpose |
|--------|---------|
| `feature/*` | Feature development (local work branch) |
| `develop` | Integration & staging |
| `main` | Production releases (tagged) |

## PR Policy

- **All local PRs: `feature/*` → `develop`**
- `develop` → `main` PRs are for releases only (tagged `vX.Y.Z`)
- Never target `main` directly from a feature branch

## Branch Naming

- Feature: `feature/<short-description>` (e.g. `feature/maven-ecosystem`)
- Fix: `fix/<short-description>` (e.g. `fix/tier2-cargo-timeout`)
- Docs: `docs/<short-description>`
- If currently on `develop` or `main`, stop and prompt the user to create a feature branch before push/PR

## diff base

- PR description, code review, and change analysis always diff against **`develop`**
- Never hardcode `main` as diff base (exception: `develop` → `main` release PRs)
