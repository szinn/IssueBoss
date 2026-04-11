use std::path::Path;

use anyhow::{Context, Result};

/// Ensure `entry` appears as its own line in `<project_root>/.gitignore`.
/// Creates the file if it does not exist. No-op if already present (full-line
/// match).
pub fn ensure_gitignore_entry(project_root: &Path, entry: &str, verbose: bool) -> Result<()> {
    let gitignore = project_root.join(".gitignore");
    if verbose {
        tracing::trace!("[gitignore:check] {} entry={}", gitignore.display(), entry);
    }
    let content = if gitignore.exists() {
        std::fs::read_to_string(&gitignore).with_context(|| format!("Failed to read '{}'", gitignore.display()))?
    } else {
        String::new()
    };

    // Full-line match only (not substring)
    if content.lines().any(|line| line.trim() == entry.trim()) {
        return Ok(());
    }

    if verbose {
        tracing::trace!("[gitignore:add] {} entry={}", gitignore.display(), entry);
    }
    // Ensure file ends with newline before appending
    let suffix = if content.is_empty() || content.ends_with('\n') {
        format!("{entry}\n")
    } else {
        format!("\n{entry}\n")
    };
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&gitignore)
        .with_context(|| format!("Failed to open '{}' for append", gitignore.display()))?;
    file.write_all(suffix.as_bytes())
        .with_context(|| format!("Failed to write to '{}'", gitignore.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_gitignore_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        ensure_gitignore_entry(dir.path(), ".insights/", false).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains(".insights/"));
    }

    #[test]
    fn appends_to_existing_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "target/\n").unwrap();
        ensure_gitignore_entry(dir.path(), ".insights/", false).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(content.contains("target/"));
        assert!(content.contains(".insights/"));
    }

    #[test]
    fn no_duplicate_if_already_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "target/\n.insights/\n").unwrap();
        ensure_gitignore_entry(dir.path(), ".insights/", false).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        let count = content.lines().filter(|l| *l == ".insights/").count();
        assert_eq!(count, 1, "expected exactly one .insights/ entry, got {count}");
    }

    #[test]
    fn substring_does_not_count_as_match() {
        let dir = tempfile::tempdir().unwrap();
        // ".insights/shared" contains ".insights" but is not the entry ".insights/"
        std::fs::write(dir.path().join(".gitignore"), ".insights/shared\n").unwrap();
        ensure_gitignore_entry(dir.path(), ".insights/", false).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).unwrap();
        assert!(
            content.contains(".insights/\n") || content.ends_with(".insights/"),
            "entry should have been added: {content}"
        );
    }
}
