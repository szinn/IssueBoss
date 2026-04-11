use std::path::Path;

use anyhow::Result;

use crate::{
    config::Config,
    core::{git::run_git, searchable::rebuild_searchable, symlinks::ensure_symlink},
};

/// Pull latest from repo, re-create symlinks, rebuild searchable tree.
pub fn sync(config: &Config, verbose: bool) -> Result<()> {
    let repo = &config.repo;

    // 1. Pull
    run_git(repo, &["pull"], verbose)?;

    // 2. Re-create symlinks
    let insights_dir = Path::new(".insights");
    let project_lower = config.project_lower();

    ensure_symlink(&insights_dir.join("issues"), &repo.join(format!("projects/{project_lower}/issues")), verbose)?;
    ensure_symlink(&insights_dir.join("shared"), &repo.join("shared"), verbose)?;
    ensure_symlink(&insights_dir.join(&config.user), &repo.join(format!("users/{}", config.user)), verbose)?;

    // 3. Rebuild searchable tree
    rebuild_searchable(insights_dir, verbose)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Creates a bare "remote" repo with the expected directory structure, then
    /// clones it so the working clone has tracking info and `git pull`
    /// succeeds. Returns (clone_dir, _bare_dir) — caller must keep both
    /// alive.
    fn setup_repo() -> (tempfile::TempDir, tempfile::TempDir) {
        // 1. Create the bare upstream repo with the required content.
        let bare = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let out = Command::new("git").arg("-C").arg(bare.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup failed: {args:?}");
        }
        std::fs::create_dir_all(bare.path().join("projects/myproject/issues")).unwrap();
        std::fs::create_dir_all(bare.path().join("shared/research")).unwrap();
        std::fs::create_dir_all(bare.path().join("users/alice")).unwrap();
        std::fs::write(bare.path().join("shared/research/note.md"), "content").unwrap();
        for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
            let out = Command::new("git").arg("-C").arg(bare.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup failed: {args:?}");
        }

        // 2. Clone it so the working copy has a remote and tracking branch.
        let clone = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["clone", "--local"])
            .arg(bare.path())
            .arg(clone.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "git clone failed");

        (clone, bare)
    }

    #[test]
    fn sync_creates_symlinks_and_searchable() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare_dir) = setup_repo();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project_dir.path()).unwrap();

        // Create .insights dir (init would do this; sync assumes it exists)
        std::fs::create_dir_all(".insights").unwrap();

        let config = Config {
            repo: repo_dir.path().to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
        };

        sync(&config, false).unwrap();

        assert!(Path::new(".insights/issues").is_symlink(), "issues symlink missing");
        assert!(Path::new(".insights/shared").is_symlink(), "shared symlink missing");
        assert!(Path::new(".insights/alice").is_symlink(), "user symlink missing");

        assert!(
            Path::new(".insights/searchable/shared/research/note.md").exists(),
            "searchable hard link missing"
        );

        std::env::set_current_dir(original).unwrap();
    }
}
