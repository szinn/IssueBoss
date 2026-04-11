use anyhow::Result;

use crate::{
    config::Config,
    core::{git::git_is_dirty, sync::sync},
};

pub fn status(config: &Config, verbose: bool) -> Result<()> {
    sync(config, verbose)?;
    match git_is_dirty(&config.repo, verbose)? {
        None => println!("Insights repo is up to date — nothing to sync"),
        Some(output) => {
            let count = output.lines().count();
            println!("{count} file{} ready to sync:", if count == 1 { "" } else { "s" });
            print!("{output}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    /// Bare upstream + local clone, same pattern as sync tests.
    fn setup_repo() -> (tempfile::TempDir, tempfile::TempDir) {
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
    fn status_clean_repo_succeeds() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare_dir) = setup_repo();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project_dir.path()).unwrap();
        std::fs::create_dir_all(".insights").unwrap();

        let config = Config {
            repo: repo_dir.path().to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
        };

        status(&config, false).unwrap();

        std::env::set_current_dir(original).unwrap();
    }

    #[test]
    fn status_dirty_repo_succeeds() {
        let project_dir = tempfile::tempdir().unwrap();
        let (repo_dir, _bare_dir) = setup_repo();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(project_dir.path()).unwrap();
        std::fs::create_dir_all(".insights").unwrap();

        // Write a file into the repo to make it dirty
        std::fs::write(repo_dir.path().join("shared/research/new.md"), "pending").unwrap();

        let config = Config {
            repo: repo_dir.path().to_owned(),
            user: "alice".into(),
            project: "MyProject".into(),
        };

        status(&config, false).unwrap();

        // Verify searchable tree was built (sync ran)
        assert!(
            Path::new(".insights/searchable/shared/research/note.md").exists(),
            "searchable hard link missing — sync did not run"
        );

        std::env::set_current_dir(original).unwrap();
    }
}
