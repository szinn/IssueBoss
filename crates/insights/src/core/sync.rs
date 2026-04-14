use anyhow::Result;

use crate::{
    config::Config,
    core::{git::git_pull, searchable::rebuild_searchable, symlinks::ensure_symlink},
};

/// Pull latest from repo, re-create symlinks, rebuild searchable tree.
pub fn sync(config: &Config, verbose: bool) -> Result<()> {
    let repo = &config.repo;

    // 1. Pull
    git_pull(repo, verbose)?;

    // 2. Re-create symlinks
    let insights_dir = Config::insights_dir();
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
    use std::path::Path;

    use super::*;
    use crate::core::test_support::setup_insights_repo;

    #[test]
    fn sync_creates_symlinks_and_searchable() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare_dir) = setup_insights_repo();

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
