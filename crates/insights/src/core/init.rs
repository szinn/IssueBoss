use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{
    config::Config,
    core::{git::run_git, gitignore::ensure_gitignore_entry, searchable::rebuild_searchable, symlinks::ensure_symlink},
};

pub struct InitOptions {
    pub repo: PathBuf,
    pub user: String,
    pub project: String,
    /// Project root directory (the directory where .insights/ will be created)
    pub project_root: PathBuf,
}

#[allow(clippy::needless_pass_by_value)]
pub fn init(opts: InitOptions, verbose: bool) -> Result<()> {
    let repo = &opts.repo;
    let project_lower = opts.project.to_lowercase();
    let insights_dir = opts.project_root.join(".insights");

    // 1. Pull
    run_git(repo, &["pull"], verbose)?;

    // 2. Create directories in repo
    let dirs = [
        repo.join("shared/research"),
        repo.join("shared/specs"),
        repo.join("shared/plans"),
        repo.join(format!("projects/{project_lower}/issues")),
        repo.join(format!("users/{}", opts.user)),
    ];
    for dir in &dirs {
        std::fs::create_dir_all(dir).with_context(|| format!("Failed to create repo directory '{}'", dir.display()))?;
    }

    // 3. Create .insights/
    std::fs::create_dir_all(&insights_dir).with_context(|| format!("Failed to create '{}'", insights_dir.display()))?;

    // 4. Ensure .gitignore contains .insights/
    ensure_gitignore_entry(&opts.project_root, ".insights/", verbose)?;

    // 5. Create symlinks
    ensure_symlink(&insights_dir.join("issues"), &repo.join(format!("projects/{project_lower}/issues")), verbose)?;
    ensure_symlink(&insights_dir.join("shared"), &repo.join("shared"), verbose)?;
    ensure_symlink(&insights_dir.join(&opts.user), &repo.join(format!("users/{}", opts.user)), verbose)?;

    // 6. Build searchable tree
    rebuild_searchable(&insights_dir, verbose)?;

    // 7. Write config
    // Config::write() uses CWD-relative path (.insights/config.toml)
    let config = Config {
        repo: repo.clone(),
        user: opts.user.clone(),
        project: opts.project.clone(),
    };
    let original = std::env::current_dir().context("Failed to get current dir")?;
    std::env::set_current_dir(&opts.project_root).with_context(|| format!("Failed to chdir to '{}'", opts.project_root.display()))?;
    let result = config.write();
    std::env::set_current_dir(original).context("Failed to restore working directory")?;
    result
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// Creates a bare upstream and a local clone with tracking so `git pull`
    /// works.
    fn setup_repo() -> (tempfile::TempDir, tempfile::TempDir) {
        let bare = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .arg("-C")
            .arg(bare.path())
            .args(["init", "--bare", "-b", "main"])
            .output()
            .unwrap();
        assert!(out.status.success(), "git bare init failed");

        // Initialize a temp repo, commit to it, and push to bare
        let staging = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let out = Command::new("git").arg("-C").arg(staging.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup failed: {args:?}");
        }
        std::fs::write(staging.path().join("README.md"), "insights repo").unwrap();
        for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
            let out = Command::new("git").arg("-C").arg(staging.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git commit failed: {args:?}");
        }
        let out = Command::new("git")
            .arg("-C")
            .arg(staging.path())
            .args(["push", bare.path().to_str().unwrap(), "main:main"])
            .output()
            .unwrap();
        assert!(out.status.success(), "git push to bare failed");

        // Clone bare so we get tracking info
        let clone = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["clone", "--local", bare.path().to_str().unwrap()])
            .arg(clone.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "git clone failed");
        for args in [vec!["config", "user.email", "t@t.com"], vec!["config", "user.name", "T"]] {
            Command::new("git").arg("-C").arg(clone.path()).args(&args).output().unwrap();
        }

        (clone, bare)
    }

    #[test]
    fn full_init() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare) = setup_repo();

        init(
            InitOptions {
                repo: repo_dir.path().to_owned(),
                user: "alice".into(),
                project: "MyProject".into(),
                project_root: project_dir.path().to_owned(),
            },
            false,
        )
        .unwrap();

        let insights = project_dir.path().join(".insights");
        assert!(insights.join("config.toml").exists(), "config.toml missing");
        assert!(insights.join("issues").is_symlink(), "issues symlink missing");
        assert!(insights.join("shared").is_symlink(), "shared symlink missing");
        assert!(insights.join("alice").is_symlink(), "user symlink missing");
        assert!(insights.join("searchable").is_dir(), "searchable dir missing");
        assert!(project_dir.path().join(".gitignore").exists(), ".gitignore missing");
        let gitignore = std::fs::read_to_string(project_dir.path().join(".gitignore")).unwrap();
        assert!(gitignore.contains(".insights/"), ".gitignore missing .insights/ entry");
    }

    #[test]
    fn idempotent() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare) = setup_repo();

        let make_opts = || InitOptions {
            repo: repo_dir.path().to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
            project_root: project_dir.path().to_owned(),
        };

        init(make_opts(), false).unwrap();
        init(make_opts(), false).unwrap(); // should not error
    }
}
