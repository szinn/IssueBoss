//! Shared test fixtures for `core` module tests.
//! Only compiled when `cfg(test)`.

use std::process::Command;

use tempfile::TempDir;

/// Creates a bare upstream repo containing the standard insights directory
/// structure, then clones it. Returns `(clone_dir, bare_dir)`.
/// The clone has `user.email` and `user.name` configured.
///
/// Directory structure in the bare/clone:
/// ```
/// projects/myproject/issues/
/// shared/research/note.md
/// users/alice/
/// ```
pub fn setup_insights_repo() -> (TempDir, TempDir) {
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
        assert!(out.status.success(), "git commit failed: {args:?}");
    }
    let clone = tempfile::tempdir().unwrap();
    let out = Command::new("git")
        .args(["clone", "--local"])
        .arg(bare.path())
        .arg(clone.path())
        .output()
        .unwrap();
    assert!(out.status.success(), "git clone failed");
    for args in [vec!["config", "user.email", "t@t.com"], vec!["config", "user.name", "T"]] {
        let out = Command::new("git").arg("-C").arg(clone.path()).args(&args).output().unwrap();
        assert!(out.status.success(), "git config on clone failed: {args:?}");
    }
    (clone, bare)
}
