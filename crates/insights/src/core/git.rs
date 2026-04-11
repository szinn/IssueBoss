use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Run a git command in `repo` directory. Returns stdout as a String.
/// Emits a TRACE log with the full command when verbose.
pub fn run_git(repo: &Path, args: &[&str], verbose: bool) -> Result<String> {
    if verbose {
        tracing::trace!(cmd = format!("git {}", args.join(" ")), cwd = %repo.display(), "cmd");
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_git_repo() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["init", "-b", "main"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["config", "user.email", "test@test.com"])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["config", "user.name", "Test"])
            .output()
            .unwrap();
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
    fn status_in_clean_repo() {
        let dir = init_git_repo();
        std::fs::write(dir.path().join("README.md"), "hello").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["add", "."])
            .output()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(["commit", "-m", "init"])
            .output()
            .unwrap();
        let out = run_git(dir.path(), &["status", "--short"], false).unwrap();
        assert!(out.is_empty(), "expected clean repo, got: {out}");
    }
}
