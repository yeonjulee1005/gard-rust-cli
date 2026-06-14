use gard_core::{Manifest, Package};
use std::path::Path;

/// Return packages present in the current lockfiles but not yet in the gard manifest.
/// If `.gard/manifest.json` does not exist yet, all lockfile packages are returned.
pub fn detect_new_packages(repo_root: &Path) -> anyhow::Result<Vec<Package>> {
    let manifest_path = repo_root.join(".gard").join("manifest.json");
    let manifest = if manifest_path.exists() {
        Manifest::load(&manifest_path)?
    } else {
        Manifest::new(None)
    };

    gard_pkg::manifest_diff::find_new_packages(repo_root, &manifest)
}
