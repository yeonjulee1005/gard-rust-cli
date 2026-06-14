use std::path::Path;

const MARKER_START: &str = "# --- gard (auto-injected) ---";
const MARKER_END: &str = "# --- end gard ---";

const GARD_SNIPPET: &str = "gard scan --packages --quiet\nif [ $? -ne 0 ]; then exit 1; fi";

/// Install the gard pre-push hook.
/// If a hook already exists it is merged: gard snippet is injected at the top,
/// and the original is backed up to `pre-push.bak`.
pub fn install(repo_root: &Path) -> anyhow::Result<()> {
    let hooks_dir = repo_root.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("pre-push");
    let block = format!("{MARKER_START}\n{GARD_SNIPPET}\n{MARKER_END}\n");

    if hook_path.exists() {
        let existing = std::fs::read_to_string(&hook_path)?;
        if existing.contains(MARKER_START) {
            return Ok(()); // already installed
        }
        // Back up and merge
        std::fs::copy(&hook_path, hooks_dir.join("pre-push.bak"))?;
        std::fs::write(&hook_path, inject_block(&existing, &block))?;
    } else {
        std::fs::write(&hook_path, format!("#!/bin/sh\n{block}"))?;
    }

    make_executable(&hook_path)?;
    Ok(())
}

/// Remove the gard snippet from the pre-push hook (or delete the file if it
/// was the only content). Restores `pre-push.bak` if the hook file is removed.
pub fn uninstall(repo_root: &Path) -> anyhow::Result<()> {
    let hooks_dir = repo_root.join(".git").join("hooks");
    let hook_path = hooks_dir.join("pre-push");

    if !hook_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&hook_path)?;
    let stripped = remove_block(&content);

    // Only the shebang line remains → nothing useful left
    if stripped.lines().filter(|l| !l.trim().is_empty()).count() <= 1 {
        std::fs::remove_file(&hook_path)?;
        let bak = hooks_dir.join("pre-push.bak");
        if bak.exists() {
            std::fs::rename(&bak, &hook_path)?;
            make_executable(&hook_path)?;
        }
    } else {
        std::fs::write(&hook_path, stripped)?;
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn inject_block(existing: &str, block: &str) -> String {
    // Preserve the shebang if present, inject gard right after it.
    let shebangs = ["#!/bin/sh\n", "#!/bin/bash\n", "#!/usr/bin/env sh\n"];
    for shebang in shebangs {
        if let Some(rest) = existing.strip_prefix(shebang) {
            return format!("{shebang}{block}{rest}");
        }
    }
    format!("#!/bin/sh\n{block}{existing}")
}

fn remove_block(content: &str) -> String {
    let mut in_block = false;
    let mut lines: Vec<&str> = Vec::new();

    for line in content.lines() {
        if line.trim_start().starts_with(MARKER_START) {
            in_block = true;
            continue;
        }
        if line.trim_start().starts_with(MARKER_END) {
            in_block = false;
            continue;
        }
        if !in_block {
            lines.push(line);
        }
    }

    let mut result = lines.join("\n");
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[cfg(unix)]
fn make_executable(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn hooks_dir(dir: &TempDir) -> std::path::PathBuf {
        let p = dir.path().join(".git").join("hooks");
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn install_creates_hook() {
        let dir = TempDir::new().unwrap();
        hooks_dir(&dir);
        install(dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".git/hooks/pre-push")).unwrap();
        assert!(content.contains(MARKER_START));
        assert!(content.contains("gard scan"));
    }

    #[test]
    fn install_is_idempotent() {
        let dir = TempDir::new().unwrap();
        hooks_dir(&dir);
        install(dir.path()).unwrap();
        install(dir.path()).unwrap(); // second call must not panic
        let content = std::fs::read_to_string(dir.path().join(".git/hooks/pre-push")).unwrap();
        assert_eq!(content.matches(MARKER_START).count(), 1);
    }

    #[test]
    fn install_merges_existing_hook() {
        let dir = TempDir::new().unwrap();
        let hook = hooks_dir(&dir).join("pre-push");
        std::fs::write(&hook, "#!/bin/sh\nnpm run pre-push-checks\n").unwrap();
        install(dir.path()).unwrap();
        let content = std::fs::read_to_string(&hook).unwrap();
        assert!(content.contains(MARKER_START));
        assert!(content.contains("npm run pre-push-checks"));
        assert!(std::fs::metadata(dir.path().join(".git/hooks/pre-push.bak")).is_ok());
    }

    #[test]
    fn uninstall_removes_hook() {
        let dir = TempDir::new().unwrap();
        hooks_dir(&dir);
        install(dir.path()).unwrap();
        uninstall(dir.path()).unwrap();
        assert!(!dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn uninstall_preserves_original_content() {
        let dir = TempDir::new().unwrap();
        let hook = hooks_dir(&dir).join("pre-push");
        std::fs::write(&hook, "#!/bin/sh\nnpm run checks\n").unwrap();
        install(dir.path()).unwrap();
        uninstall(dir.path()).unwrap();
        let content = std::fs::read_to_string(&hook).unwrap();
        assert!(!content.contains(MARKER_START));
        assert!(content.contains("npm run checks"));
    }
}
