use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{
    config::Config,
    core::{
        claude_md::{ensure_claude_md_frontmatter_hint, ensure_claude_md_insights_section},
        git::git_pull,
        gitignore::ensure_gitignore_entry,
        schema_md::ensure_schema_md,
        searchable::rebuild_searchable,
        symlinks::ensure_symlink,
    },
};

pub struct InitOptions {
    pub repo: PathBuf,
    pub user: String,
    pub project: String,
    /// Project root directory (the directory where .insights/ will be created)
    pub project_root: PathBuf,
}

/// Create the standard directory tree inside the insights git repo.
fn create_repo_dirs(repo: &Path, project_lower: &str, user: &str) -> Result<()> {
    let dirs = [
        repo.join("shared/research"),
        repo.join("shared/specs"),
        repo.join("shared/plans"),
        repo.join(format!("projects/{project_lower}/issues")),
        repo.join(format!("users/{user}")),
    ];
    for dir in &dirs {
        std::fs::create_dir_all(dir).with_context(|| format!("Failed to create repo directory '{}'", dir.display()))?;
    }
    Ok(())
}

/// Create the local `.insights/` directory if it does not exist.
fn create_insights_dir(insights_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(insights_dir).with_context(|| format!("Failed to create '{}'", insights_dir.display()))
}

/// Write symlinks from `.insights/` into the repo.
fn create_symlinks(insights_dir: &Path, repo: &Path, project_lower: &str, user: &str, verbose: bool) -> Result<()> {
    ensure_symlink(&insights_dir.join("issues"), &repo.join(format!("projects/{project_lower}/issues")), verbose)?;
    ensure_symlink(&insights_dir.join("shared"), &repo.join("shared"), verbose)?;
    ensure_symlink(&insights_dir.join(user), &repo.join(format!("users/{user}")), verbose)?;
    Ok(())
}

/// Write the insights config to `.insights/config.toml` relative to
/// `project_root`.
fn write_config(repo: &Path, user: &str, project: &str, project_root: &Path) -> Result<()> {
    let config = Config {
        repo: repo.to_owned(),
        user: user.to_owned(),
        project: project.to_owned(),
    };
    let original = std::env::current_dir().context("Failed to get current dir")?;
    std::env::set_current_dir(project_root).with_context(|| format!("Failed to chdir to '{}'", project_root.display()))?;
    let result = config.write();
    std::env::set_current_dir(original).context("Failed to restore working directory")?;
    result
}

#[allow(clippy::needless_pass_by_value)]
pub fn init(opts: InitOptions, verbose: bool) -> Result<()> {
    let repo = &opts.repo;
    let project_lower = opts.project.to_lowercase();
    let insights_dir = opts.project_root.join(".insights");

    // 1. Pull
    git_pull(repo, verbose)?;

    // 2. Create directories in repo
    create_repo_dirs(repo, &project_lower, &opts.user)?;

    // 3. Create .insights/
    create_insights_dir(&insights_dir)?;

    // 4. Ensure .gitignore contains .insights/
    ensure_gitignore_entry(&opts.project_root, ".insights/", verbose)?;

    // 5. Create symlinks
    create_symlinks(&insights_dir, repo, &project_lower, &opts.user, verbose)?;

    // 6. Build searchable tree
    rebuild_searchable(&insights_dir, verbose)?;

    // 7. Write schema.md to shared dir (idempotent)
    ensure_schema_md(&repo.join("shared"), verbose)?;

    // 8. Write config
    write_config(repo, &opts.user, &opts.project, &opts.project_root)?;

    // 9. Write CLAUDE.md insights section (idempotent)
    ensure_claude_md_insights_section(&opts.project_root, &opts.user)?;

    // 10. Add front-matter hint to CLAUDE.md (idempotent)
    ensure_claude_md_frontmatter_hint(&opts.project_root)?;

    Ok(())
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

    // --- Step-level unit tests (no git repo required) ---

    #[test]
    fn create_repo_dirs_creates_expected_structure() {
        let dir = tempfile::tempdir().unwrap();
        create_repo_dirs(dir.path(), "myproject", "alice").unwrap();

        assert!(dir.path().join("shared/research").is_dir());
        assert!(dir.path().join("shared/specs").is_dir());
        assert!(dir.path().join("shared/plans").is_dir());
        assert!(dir.path().join("projects/myproject/issues").is_dir());
        assert!(dir.path().join("users/alice").is_dir());
    }

    #[test]
    fn create_repo_dirs_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        create_repo_dirs(dir.path(), "myproject", "alice").unwrap();
        create_repo_dirs(dir.path(), "myproject", "alice").unwrap(); // must not error
    }

    #[test]
    fn create_insights_dir_creates_dir() {
        let dir = tempfile::tempdir().unwrap();
        let insights = dir.path().join(".insights");
        create_insights_dir(&insights).unwrap();
        assert!(insights.is_dir());
    }

    #[test]
    fn create_symlinks_creates_three_links() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let insights = dir.path().join(".insights");
        std::fs::create_dir_all(repo.join("projects/myproject/issues")).unwrap();
        std::fs::create_dir_all(repo.join("shared")).unwrap();
        std::fs::create_dir_all(repo.join("users/alice")).unwrap();
        std::fs::create_dir_all(&insights).unwrap();

        create_symlinks(&insights, &repo, "myproject", "alice", false).unwrap();

        assert!(insights.join("issues").is_symlink(), "issues symlink missing");
        assert!(insights.join("shared").is_symlink(), "shared symlink missing");
        assert!(insights.join("alice").is_symlink(), "user symlink missing");
    }

    #[test]
    fn write_config_creates_config_toml() {
        let repo_dir = tempfile::tempdir().unwrap();
        let project_dir = tempfile::tempdir().unwrap();

        write_config(repo_dir.path(), "alice", "MyProject", project_dir.path()).unwrap();

        let config_path = project_dir.path().join(".insights/config.toml");
        assert!(config_path.exists(), "config.toml not written");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("MyProject"), "project name missing from config");
        assert!(content.contains("alice"), "user name missing from config");
    }

    // --- Full integration tests (require git clone fixture) ---

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
    fn full_init_writes_claude_md() {
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

        let claude_md = project_dir.path().join("CLAUDE.md");
        assert!(claude_md.exists(), "CLAUDE.md should be created by init");
        let content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(content.contains("## Insights"), "CLAUDE.md missing ## Insights section");
        assert!(content.contains(".insights/alice/"), "CLAUDE.md missing user-specific path");
    }

    #[test]
    fn full_init_writes_schema_md() {
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

        assert!(repo_dir.path().join("shared/schema.md").exists(), "schema.md not written to shared dir");
    }

    #[test]
    fn full_init_adds_frontmatter_hint_to_claude_md() {
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

        let claude_md = project_dir.path().join("CLAUDE.md");
        let content = std::fs::read_to_string(&claude_md).unwrap();
        assert!(content.contains("schema.md"), "CLAUDE.md missing front-matter hint");
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
