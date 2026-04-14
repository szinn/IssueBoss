use std::{path::Path, process::Command};

use anyhow::{Context, Result};

/// Run a git command in `repo` directory. Returns stdout as a String.
/// Emits a TRACE log with the full command when verbose.
pub fn run_git(repo: &Path, args: &[&str], verbose: bool) -> Result<String> {
    if verbose {
        tracing::trace!("[cmd] git -C {} {}", repo.display(), args.join(" "));
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .context("Failed to spawn git — is git installed?")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git {} failed: {}", args.join(" "), stderr.trim())
    }
}

/// Returns `Some(output)` when the repo has staged or unstaged changes,
/// `None` when the working tree is clean.
pub fn git_is_dirty(repo: &Path, verbose: bool) -> Result<Option<String>> {
    let out = run_git(repo, &["status", "--short"], verbose)?;
    if out.trim().is_empty() { Ok(None) } else { Ok(Some(out)) }
}

/// Stage all changes in `repo` (equivalent to `git add -A`).
pub fn git_add_all(repo: &Path, verbose: bool) -> Result<()> {
    run_git(repo, &["add", "-A"], verbose)?;
    Ok(())
}

/// Commit staged changes with `message`. Does not allow empty commits.
pub fn git_commit_with_message(repo: &Path, message: &str, verbose: bool) -> Result<()> {
    run_git(repo, &["commit", "-m", message], verbose)?;
    Ok(())
}

/// Push to the tracked remote.
pub fn git_push(repo: &Path, verbose: bool) -> Result<()> {
    run_git(repo, &["push"], verbose)?;
    Ok(())
}

/// Pull from the tracked remote.
pub fn git_pull(repo: &Path, verbose: bool) -> Result<()> {
    run_git(repo, &["pull"], verbose)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn init_git_repo() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "test@test.com"],
            vec!["config", "user.name", "Test"],
        ] {
            let out = Command::new("git").arg("-C").arg(dir.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup step failed: {args:?}");
        }
        dir
    }

    #[test]
    fn version_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_git(dir.path(), &["--version"], false).unwrap();
        assert!(out.contains("git version"), "expected version string, got: {out}");
    }

    #[test]
    fn invalid_command_returns_err() {
        let dir = init_git_repo();
        let err = run_git(dir.path(), &["no-such-subcommand-xyz"], false).unwrap_err();
        assert!(err.to_string().contains("failed"), "expected failure message");
    }

    #[test]
    fn is_dirty_returns_none_on_clean_repo() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["add", "."]).output().unwrap();
        assert!(out.status.success());
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["commit", "-m", "init"]).output().unwrap();
        assert!(out.status.success());

        let result = git_is_dirty(dir.path(), false).unwrap();
        assert!(result.is_none(), "expected None for clean repo, got: {result:?}");
    }

    #[test]
    fn is_dirty_returns_some_when_untracked_file() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["add", "."]).output().unwrap();
        assert!(out.status.success());
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["commit", "-m", "init"]).output().unwrap();
        assert!(out.status.success());

        std::fs::write(dir.path().join("new.md"), "untracked content").unwrap();
        let result = git_is_dirty(dir.path(), false).unwrap();
        assert!(result.is_some(), "expected Some for untracked file");
        assert!(result.unwrap().contains("new.md"));
    }

    #[test]
    fn is_dirty_returns_some_when_modified_file() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["add", "."]).output().unwrap();
        assert!(out.status.success());
        let out = Command::new("git").arg("-C").arg(dir.path()).args(["commit", "-m", "init"]).output().unwrap();
        assert!(out.status.success());

        std::fs::write(dir.path().join("README.md"), "modified content").unwrap();
        let result = git_is_dirty(dir.path(), false).unwrap();
        assert!(result.is_some(), "expected Some for modified file");
    }

    #[test]
    fn status_in_clean_repo() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        let add = Command::new("git").arg("-C").arg(dir.path()).args(["add", "."]).output().unwrap();
        assert!(add.status.success(), "git add failed");
        let commit = Command::new("git").arg("-C").arg(dir.path()).args(["commit", "-m", "init"]).output().unwrap();
        assert!(commit.status.success(), "git commit failed");
        let out = run_git(dir.path(), &["status", "--short"], false).unwrap();
        assert!(out.is_empty(), "expected clean repo, got: {out}");
    }

    fn setup_bare_and_clone() -> (TempDir, TempDir) {
        let bare = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "--bare", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let out = Command::new("git").arg("-C").arg(bare.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup step failed: {args:?}");
        }
        // Bootstrap a staging repo, commit, push to bare
        let staging = tempfile::tempdir().unwrap();
        for args in [
            vec!["init", "-b", "main"],
            vec!["config", "user.email", "t@t.com"],
            vec!["config", "user.name", "T"],
        ] {
            let out = Command::new("git").arg("-C").arg(staging.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup step failed: {args:?}");
        }
        std::fs::write(staging.path().join("README.md"), "init").unwrap();
        for args in [vec!["add", "."], vec!["commit", "-m", "init"]] {
            let out = Command::new("git").arg("-C").arg(staging.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup step failed: {args:?}");
        }
        let out = Command::new("git")
            .arg("-C")
            .arg(staging.path())
            .args(["push", bare.path().to_str().unwrap(), "main:main"])
            .output()
            .unwrap();
        assert!(out.status.success(), "git push to bare failed");

        let clone = tempfile::tempdir().unwrap();
        let out = Command::new("git")
            .args(["clone", "--local", bare.path().to_str().unwrap()])
            .arg(clone.path())
            .output()
            .unwrap();
        assert!(out.status.success(), "git clone failed");
        for args in [vec!["config", "user.email", "t@t.com"], vec!["config", "user.name", "T"]] {
            let out = Command::new("git").arg("-C").arg(clone.path()).args(&args).output().unwrap();
            assert!(out.status.success(), "git setup step failed: {args:?}");
        }
        (clone, bare)
    }

    #[test]
    fn git_add_all_stages_files() {
        let dir = init_git_repo();
        // Create initial commit so repo has a HEAD
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        run_git(dir.path(), &["add", "."], false).unwrap();
        run_git(dir.path(), &["commit", "-m", "init"], false).unwrap();

        std::fs::write(dir.path().join("new.md"), "content").unwrap();
        assert!(git_is_dirty(dir.path(), false).unwrap().is_some());

        git_add_all(dir.path(), false).unwrap();

        let out = run_git(dir.path(), &["status", "--short"], false).unwrap();
        assert!(out.contains('A'), "expected staged file after add_all: {out}");
    }

    #[test]
    fn git_commit_with_message_creates_commit() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        git_add_all(dir.path(), false).unwrap();
        git_commit_with_message(dir.path(), "test commit", false).unwrap();

        let log = run_git(dir.path(), &["log", "--oneline"], false).unwrap();
        assert!(log.contains("test commit"), "commit not found in log: {log}");
    }

    #[test]
    fn git_pull_succeeds_on_up_to_date_clone() {
        let (clone_dir, _bare) = setup_bare_and_clone();
        git_pull(clone_dir.path(), false).unwrap();
    }

    #[test]
    fn git_push_sends_commit_to_remote() {
        let (clone_dir, bare_dir) = setup_bare_and_clone();
        std::fs::write(clone_dir.path().join("new.md"), "pushed").unwrap();
        git_add_all(clone_dir.path(), false).unwrap();
        git_commit_with_message(clone_dir.path(), "push test", false).unwrap();
        git_push(clone_dir.path(), false).unwrap();

        let log = run_git(bare_dir.path(), &["log", "--oneline"], false).unwrap();
        assert!(log.contains("push test"), "push did not reach bare remote: {log}");
    }
}
