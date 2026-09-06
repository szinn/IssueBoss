use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// Rebuild the hard-link mirror at `<insights_dir>/searchable/`.
/// - Walks all real files reachable through symlinks under `insights_dir`
///   (excluding `searchable/` itself and `config.toml`)
/// - Creates hard links in `searchable/<relative_path>` for files that don't
///   have one, or whose inode has changed (source was atomically replaced)
/// - Removes hard links whose source file no longer exists
/// - Cleans up empty directories left behind after stale removal
pub fn rebuild_searchable(insights_dir: &Path, verbose: bool) -> Result<()> {
    if !insights_dir.exists() {
        anyhow::bail!("insights directory '{}' does not exist. Run 'insights init' first.", insights_dir.display());
    }

    let searchable = insights_dir.join("searchable");
    std::fs::create_dir_all(&searchable).with_context(|| format!("Failed to create '{}'", searchable.display()))?;

    // Collect all real file paths reachable through insights_dir (following
    // symlinks). Relative paths are stripped from insights_dir (e.g.
    // "shared/research/foo.md").
    let files = collect_insight_files(insights_dir)?;

    // Build hard links for each file
    for (rel_path, abs_src) in &files {
        // dest relative path mirrors the source relative path under
        // searchable/. Both strips use the same base, so rel_path
        // comparisons in remove_stale_links are valid: collect strips
        // insights_dir, remove_stale_links strips searchable.
        let dest = searchable.join(rel_path);
        if verbose {
            tracing::trace!("[symlink:check] hard: {}", dest.display());
        }
        if dest.exists() {
            // Verify the hard link still points to the same inode. If the
            // source was atomically replaced (editor safe-write),
            // the inode changes while the path remains the same —
            // we must re-link in that case.
            if let (Ok(src_meta), Ok(dest_meta)) = (std::fs::metadata(abs_src), std::fs::metadata(&dest)) {
                if src_meta.ino() == dest_meta.ino() {
                    continue; // already correctly linked
                }
            } else {
                continue; // can't stat — leave as-is
            }
            // Inode mismatch: remove stale hard link before re-creating
            if verbose {
                tracing::trace!("[symlink:remove] hard: {}", dest.display());
            }
            std::fs::remove_file(&dest).with_context(|| format!("Failed to remove stale hard link '{}'", dest.display()))?;
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

    // Remove stale hard links, then clean up empty directories
    remove_stale_links(&searchable, &files, verbose)?;
    remove_empty_dirs(&searchable);

    Ok(())
}

/// Walk `insights_dir` (following symlinks, excluding `searchable/` and
/// `config.toml`) and return (relative_path, absolute_real_path) pairs for all
/// regular files. Relative paths are stripped from `insights_dir`.
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

/// Remove hard links in `searchable/` whose relative path (stripped from
/// `searchable`) is not in `live_files` (stripped from `insights_dir`).
/// Both use the same relative path convention, so the comparison is valid.
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

/// Remove empty directories under `searchable/` left behind after stale link
/// removal. Walks contents-first (deepest first) so parent dirs are cleaned
/// after their children.
fn remove_empty_dirs(searchable: &Path) {
    for entry in walkdir::WalkDir::new(searchable)
        .contents_first(true)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path != searchable && path.is_dir() {
            let _ = std::fs::remove_dir(path); // no-op if non-empty
        }
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn relinks_when_inode_changes() {
        let (dir, insights) = setup();
        rebuild_searchable(&insights, false).unwrap();

        let link = insights.join("searchable/shared/research/foo.md");
        let original_ino = std::fs::metadata(&link).unwrap().ino();

        // Atomically replace the source file (new inode)
        let repo_file = dir.path().join("repo/shared/research/foo.md");
        std::fs::write(&repo_file, "updated content").unwrap();
        // On most editors this is in-place; force a new inode via remove+write
        std::fs::remove_file(&repo_file).unwrap();
        std::fs::write(&repo_file, "updated content").unwrap();

        rebuild_searchable(&insights, false).unwrap();

        let new_ino = std::fs::metadata(&link).unwrap().ino();
        assert_ne!(original_ino, new_ino, "hard link should have been re-created with new inode");
    }

    #[test]
    fn cleans_up_empty_dirs_after_stale_removal() {
        let (dir, insights) = setup();
        rebuild_searchable(&insights, false).unwrap();

        // Remove the source file — leaves searchable/shared/research/
        // potentially empty
        let repo_file = dir.path().join("repo/shared/research/foo.md");
        std::fs::remove_file(&repo_file).unwrap();

        rebuild_searchable(&insights, false).unwrap();

        // Empty subdirectory should be cleaned up
        let empty_dir = insights.join("searchable/shared/research");
        assert!(!empty_dir.exists(), "empty dir should have been removed: {}", empty_dir.display());
    }

    #[test]
    fn fails_if_insights_dir_missing() {
        let dir = tempfile::tempdir().unwrap();
        let insights = dir.path().join(".insights");
        let result = rebuild_searchable(&insights, false);
        assert!(result.is_err(), "should fail when insights_dir does not exist");
        assert!(result.unwrap_err().to_string().contains("insights init"));
    }
}
