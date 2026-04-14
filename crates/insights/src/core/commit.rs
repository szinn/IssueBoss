use anyhow::Result;

use crate::{
    config::Config,
    core::{
        git::{git_add_all, git_commit_with_message, git_is_dirty, git_push},
        sync::sync,
    },
};

/// Sync the repo, then commit and push if there are pending changes.
/// Skips commit+push when the working tree is clean after sync.
pub fn commit(config: &Config, verbose: bool) -> Result<()> {
    // 1. Sync first (pull + symlinks + searchable)
    sync(config, verbose)?;

    let repo = &config.repo;

    // 2. Check if there is anything to commit
    if git_is_dirty(repo, verbose)?.is_none() {
        println!("Nothing to commit — insights repo is up to date");
        return Ok(());
    }

    let timestamp = chrono::Utc::now().to_rfc3339();

    // 3. Stage all changes
    git_add_all(repo, verbose)?;

    // 4. Commit (no --allow-empty: we already checked there's something to commit)
    git_commit_with_message(repo, &format!("insights update {timestamp}"), verbose)?;

    // 5. Push
    git_push(repo, verbose)?;

    println!("Committed and pushed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    /// Creates a local repo with initial commit, then creates a bare clone as
    /// "remote", then re-clones the bare remote into a fresh local working
    /// copy. Returns (remote_bare_dir, local_clone_dir).
    fn setup_repo_with_remote() -> (tempfile::TempDir, tempfile::TempDir) {
        // 1. Bootstrap a local repo with the required structure and an initial commit.
        let bootstrap = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let out = Command::new("git").arg("-C").arg(bootstrap.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "bootstrap git {args:?} failed");
        }
        std::fs::create_dir_all(bootstrap.path().join("projects/myproject/issues")).unwrap();
        std::fs::create_dir_all(bootstrap.path().join("shared/research")).unwrap();
        std::fs::create_dir_all(bootstrap.path().join("users/alice")).unwrap();
        std::fs::write(bootstrap.path().join("shared/research/.keep"), "").unwrap();
        for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
            let out = Command::new("git").arg("-C").arg(bootstrap.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "bootstrap git {args:?} failed");
        }

        // 2. Create a bare clone to act as the remote.
        let remote = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["clone", "--bare", bootstrap.path().to_str().unwrap(), remote.path().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(out.status.success(), "bare clone failed");

        // 3. Clone the bare remote into the local working copy.
        let local = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["clone", remote.path().to_str().unwrap(), local.path().to_str().unwrap()])
            .output()
            .unwrap();
        assert!(out.status.success(), "local clone failed");
        for args in [vec!["config", "user.email", "t@t.com"], vec!["config", "user.name", "T"]] {
            let out = Command::new("git").arg("-C").arg(local.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git config failed: {args:?}");
        }

        (remote, local)
    }

    #[test]
    fn commit_stages_and_pushes() {
        let project_dir = tempfile::tempdir().unwrap();
        let (remote_dir, repo_dir) = setup_repo_with_remote();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project_dir.path()).unwrap();

        // Create .insights/ dir and the 3 symlinks manually
        std::fs::create_dir_all(".insights").unwrap();
        let repo_path = repo_dir.path();
        std::os::unix::fs::symlink(repo_path.join("projects/myproject/issues"), ".insights/issues").unwrap();
        std::os::unix::fs::symlink(repo_path.join("shared"), ".insights/shared").unwrap();
        std::os::unix::fs::symlink(repo_path.join("users/alice"), ".insights/alice").unwrap();

        // Write a file via the symlink
        std::fs::write(".insights/shared/research/new-note.md", "hello from commit test").unwrap();

        let config = Config {
            repo: repo_path.to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
        };

        commit(&config, false).unwrap();

        // Assert local git log contains "insights update"
        let local_log = Command::new("git").arg("-C").arg(repo_path).args(["log", "--oneline"]).output().unwrap();
        let local_log_str = String::from_utf8_lossy(&local_log.stdout);
        assert!(
            local_log_str.contains("insights update"),
            "local git log missing 'insights update': {local_log_str}"
        );

        // Assert remote git log also contains "insights update"
        let remote_log = Command::new("git")
            .arg("-C")
            .arg(remote_dir.path())
            .args(["log", "--oneline"])
            .output()
            .unwrap();
        let remote_log_str = String::from_utf8_lossy(&remote_log.stdout);
        assert!(
            remote_log_str.contains("insights update"),
            "remote git log missing 'insights update': {remote_log_str}"
        );

        // Also confirm the new file is visible in the searchable tree
        assert!(
            Path::new(".insights/searchable/shared/research/new-note.md").exists(),
            "searchable hard link for new-note.md missing"
        );

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn commit_skips_when_nothing_to_commit() {
        let project_dir = tempfile::tempdir().unwrap();
        let (remote_dir, repo_dir) = setup_repo_with_remote();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project_dir.path()).unwrap();

        std::fs::create_dir_all(".insights").unwrap();
        let repo_path = repo_dir.path();
        std::os::unix::fs::symlink(repo_path.join("projects/myproject/issues"), ".insights/issues").unwrap();
        std::os::unix::fs::symlink(repo_path.join("shared"), ".insights/shared").unwrap();
        std::os::unix::fs::symlink(repo_path.join("users/alice"), ".insights/alice").unwrap();

        // Do NOT write any new files — repo should be clean after sync

        let config = Config {
            repo: repo_path.to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
        };

        commit(&config, false).unwrap();

        // Git log should only have the "init" commit — no "insights update"
        let local_log = Command::new("git").arg("-C").arg(repo_path).args(["log", "--oneline"]).output().unwrap();
        let local_log_str = String::from_utf8_lossy(&local_log.stdout);
        assert!(
            !local_log_str.contains("insights update"),
            "expected no commit when nothing to commit, but got: {local_log_str}"
        );

        // Remote should also be unchanged
        let remote_log = Command::new("git")
            .arg("-C")
            .arg(remote_dir.path())
            .args(["log", "--oneline"])
            .output()
            .unwrap();
        let remote_log_str = String::from_utf8_lossy(&remote_log.stdout);
        assert!(
            !remote_log_str.contains("insights update"),
            "expected no push when nothing to commit, but got: {remote_log_str}"
        );

        std::env::set_current_dir(original).unwrap();
    }
}
