use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Rebuild the hard-link mirror at `<insights_dir>/searchable/`.
/// - Walks all real files reachable through symlinks under `insights_dir`
///   (excluding `searchable/` itself and `config.toml`)
/// - Creates hard links in `searchable/<relative_path>` for files that don't
///   have one
/// - Removes hard links whose source file no longer exists
pub fn rebuild_searchable(insights_dir: &Path, verbose: bool) -> Result<()> {
    let searchable = insights_dir.join("searchable");
    std::fs::create_dir_all(&searchable).with_context(|| format!("Failed to create '{}'", searchable.display()))?;

    // Collect all real file paths reachable through insights_dir (following
    // symlinks)
    let files = collect_insight_files(insights_dir)?;

    // Build hard links for each file
    for (rel_path, abs_src) in &files {
        let dest = searchable.join(rel_path);
        if verbose {
            tracing::trace!("[symlink:check] hard: {}", dest.display());
        }
        if dest.exists() {
            continue; // already linked
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("Failed to create dir '{}'", parent.display()))?;
        }
        if verbose {
            tracing::trace!("[symlink:create] hard: {} → {}", dest.display(), abs_src.display());
        }
        std::fs::hard_link(abs_src, &dest).with_context(|| {
            format!(
                "Failed to hard-link '{}' → '{}'. Are the project and Insights repo on the same filesystem?",
                dest.display(),
                abs_src.display()
            )
        })?;
    }

    // Remove stale hard links
    remove_stale_links(&searchable, &files, verbose)?;

    Ok(())
}

/// Walk `insights_dir` (following symlinks, excluding `searchable/` and
/// `config.toml`) and return (relative_path, absolute_real_path) pairs for all
/// regular files.
fn collect_insight_files(insights_dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let searchable = insights_dir.join("searchable");
    let config = insights_dir.join("config.toml");
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(insights_dir).follow_links(true).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        // Skip the searchable dir itself and config.toml
        if path.starts_with(&searchable) || path == config {
            continue;
        }
        if path.is_file() {
            let rel = path
                .strip_prefix(insights_dir)
                .with_context(|| format!("Failed to strip prefix from '{}'", path.display()))?;
            let canonical = path.canonicalize().with_context(|| format!("Failed to canonicalize '{}'", path.display()))?;
            files.push((rel.to_owned(), canonical));
        }
    }
    Ok(files)
}

fn remove_stale_links(searchable: &Path, live_files: &[(PathBuf, PathBuf)], verbose: bool) -> Result<()> {
    if !searchable.exists() {
        return Ok(());
    }
    let live_rel: std::collections::HashSet<&PathBuf> = live_files.iter().map(|(r, _)| r).collect();
    for entry in walkdir::WalkDir::new(searchable).follow_links(false).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_file() {
            let rel = path
                .strip_prefix(searchable)
                .with_context(|| format!("Failed to strip prefix from '{}'", path.display()))?;
            if !live_rel.contains(&rel.to_owned()) {
                if verbose {
                    tracing::trace!("[symlink:remove] hard: {}", path.display());
                }
                std::fs::remove_file(path).with_context(|| format!("Failed to remove stale link '{}'", path.display()))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::MetadataExt;

    use super::*;

    /// Create a fake insights dir with a file reachable via a symlink
    /// subdirectory.
    fn setup() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let insights = dir.path().join(".insights");
        std::fs::create_dir_all(&insights).unwrap();

        // Simulate a repo directory with files
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("shared/research")).unwrap();
        std::fs::write(repo.join("shared/research/foo.md"), "hello").unwrap();

        // Create symlink: .insights/shared → repo/shared
        std::os::unix::fs::symlink(repo.join("shared"), insights.join("shared")).unwrap();

        (dir, insights)
    }

    #[test]
    fn creates_hard_links() {
        let (_dir, insights) = setup();
        rebuild_searchable(&insights, false).unwrap();

        let link = insights.join("searchable/shared/research/foo.md");
        assert!(link.exists(), "hard link should exist at {}", link.display());

        // Verify same inode
        let src_meta = std::fs::metadata(insights.join("shared/research/foo.md")).unwrap();
        let link_meta = std::fs::metadata(&link).unwrap();
        assert_eq!(src_meta.ino(), link_meta.ino(), "hard link should share inode");
    }

    #[test]
    fn idempotent() {
        let (_dir, insights) = setup();
        rebuild_searchable(&insights, false).unwrap();
        rebuild_searchable(&insights, false).unwrap(); // should not error
        let link = insights.join("searchable/shared/research/foo.md");
        assert!(link.exists());
    }

    #[test]
    fn removes_stale_links() {
        let (dir, insights) = setup();
        rebuild_searchable(&insights, false).unwrap();

        // Delete the source file from the repo
        let repo_file = dir.path().join("repo/shared/research/foo.md");
        std::fs::remove_file(&repo_file).unwrap();

        rebuild_searchable(&insights, false).unwrap();

        let link = insights.join("searchable/shared/research/foo.md");
        assert!(!link.exists(), "stale hard link should have been removed");
    }
}
