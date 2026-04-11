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
}
