use std::path::Path;

use anyhow::{Context, Result};

/// Create or re-create a soft symlink at `link` pointing to `target`.
/// Idempotent: if `link` already points to `target` **by exact path bytes**,
/// does nothing. Pass the same form (absolute or relative) on every call.
/// If `link` points elsewhere (stale), removes and re-creates.
pub fn ensure_symlink(link: &Path, target: &Path, verbose: bool) -> Result<()> {
    if verbose {
        tracing::trace!("[symlink:check] soft: {}", link.display());
    }
    if link.is_symlink() {
        let current = std::fs::read_link(link).with_context(|| format!("Failed to read symlink '{}'", link.display()))?;
        if current == target {
            return Ok(()); // already correct
        }
        // Stale — remove and re-create
        if verbose {
            tracing::trace!("[symlink:remove] {}", link.display());
        }
        std::fs::remove_file(link).with_context(|| format!("Failed to remove stale symlink '{}'", link.display()))?;
    }
    if verbose {
        tracing::trace!("[symlink:create] soft: {} → {}", link.display(), target.display());
    }
    std::os::unix::fs::symlink(target, link).with_context(|| format!("Failed to create symlink '{}' → '{}'", link.display(), target.display()))
}

/// Remove a soft symlink at `link`. No-op if it does not exist.
pub fn remove_symlink(link: &Path, verbose: bool) -> Result<()> {
    if verbose {
        tracing::trace!("[symlink:check] {}", link.display());
    }
    if link.is_symlink() {
        if verbose {
            tracing::trace!("[symlink:remove] {}", link.display());
        }
        std::fs::remove_file(link).with_context(|| format!("Failed to remove symlink '{}'", link.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn setup() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target_dir");
        std::fs::create_dir_all(&target).unwrap();
        let link = dir.path().join("link");
        (dir, target, link)
    }

    #[test]
    fn creates_symlink() {
        let (_dir, target, link) = setup();
        ensure_symlink(&link, &target, false).unwrap();
        assert!(link.is_symlink());
        assert_eq!(std::fs::read_link(&link).unwrap(), target);
    }

    #[test]
    fn idempotent() {
        let (_dir, target, link) = setup();
        ensure_symlink(&link, &target, false).unwrap();
        ensure_symlink(&link, &target, false).unwrap(); // should not error
        assert!(link.is_symlink());
    }

    #[test]
    fn replaces_stale_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target_a = dir.path().join("a");
        let target_b = dir.path().join("b");
        std::fs::create_dir_all(&target_a).unwrap();
        std::fs::create_dir_all(&target_b).unwrap();
        let link = dir.path().join("link");

        ensure_symlink(&link, &target_a, false).unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), target_a);

        ensure_symlink(&link, &target_b, false).unwrap();
        assert_eq!(std::fs::read_link(&link).unwrap(), target_b);
    }

    #[test]
    fn remove_existing() {
        let (_dir, target, link) = setup();
        ensure_symlink(&link, &target, false).unwrap();
        remove_symlink(&link, false).unwrap();
        assert!(!link.exists() && !link.is_symlink());
    }

    #[test]
    fn remove_nonexistent_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("no_such_link");
        remove_symlink(&link, false).unwrap(); // should not error
    }

    #[test]
    fn remove_dangling_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let gone_target = dir.path().join("was_here");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&gone_target, &link).unwrap();
        // target never existed — dangling symlink; is_symlink() must be true
        assert!(link.is_symlink());
        remove_symlink(&link, false).unwrap();
        assert!(!link.is_symlink());
    }

    #[test]
    fn fails_if_parent_missing() {
        let dir = tempfile::tempdir().unwrap();
        let link = dir.path().join("nonexistent_dir").join("link");
        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        let result = ensure_symlink(&link, &target, false);
        assert!(result.is_err(), "should fail when parent dir is missing");
    }
}
