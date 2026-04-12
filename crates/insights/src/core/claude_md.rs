use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const INSIGHTS_HEADING: &str = "## Insights";

/// Appends an `## Insights` section to the project's CLAUDE.md.
///
/// Search order: `.claude/CLAUDE.md` → `CLAUDE.md` in project root → create
/// root. Idempotent: if `## Insights` already appears in the resolved file,
/// this is a no-op. Prints which file was created or updated.
pub fn ensure_claude_md_insights_section(project_root: &Path, user: &str) -> Result<()> {
    let claude_md = resolve_claude_md(project_root);

    let existing = if claude_md.exists() {
        std::fs::read_to_string(&claude_md).with_context(|| format!("Failed to read '{}'", claude_md.display()))?
    } else {
        String::new()
    };

    if existing.contains(INSIGHTS_HEADING) {
        return Ok(());
    }

    let action = if claude_md.exists() { "updated" } else { "created" };

    let snippet = insights_snippet(user);
    let content = if existing.is_empty() {
        snippet
    } else {
        format!("{}\n\n{}", existing.trim_end(), snippet)
    };

    std::fs::write(&claude_md, content).with_context(|| format!("Failed to write '{}'", claude_md.display()))?;

    println!("CLAUDE.md {action}: {}", claude_md.display());

    Ok(())
}

/// Resolve which CLAUDE.md to use.
/// Prefers `.claude/CLAUDE.md` if it exists, otherwise falls back to
/// `CLAUDE.md` in root.
fn resolve_claude_md(project_root: &Path) -> PathBuf {
    let dot_claude = project_root.join(".claude/CLAUDE.md");
    if dot_claude.exists() { dot_claude } else { project_root.join("CLAUDE.md") }
}

const FRONTMATTER_HINT: &str = "See `.insights/shared/schema.md`";

/// Appends the front-matter hint to the project's CLAUDE.md if not already
/// present.
///
/// This handles projects that already have an `## Insights` section (written
/// before this feature was added). Idempotent — no-op if hint is already
/// present or if CLAUDE.md does not exist.
pub fn ensure_claude_md_frontmatter_hint(project_root: &Path) -> Result<()> {
    let claude_md = resolve_claude_md(project_root);
    if !claude_md.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&claude_md).with_context(|| format!("Failed to read '{}'", claude_md.display()))?;
    if content.contains(FRONTMATTER_HINT) {
        return Ok(());
    }
    let hint = "\nAll `.insights/` artifact files must include YAML front-matter.\nSee `.insights/shared/schema.md` for the full schema and vocabulary.\n";
    let updated = format!("{}{}", content.trim_end(), hint);
    std::fs::write(&claude_md, updated).with_context(|| format!("Failed to write '{}'", claude_md.display()))?;
    println!("CLAUDE.md updated with front-matter hint: {}", claude_md.display());
    Ok(())
}

fn insights_snippet(user: &str) -> String {
    let template = r#"## Insights

This project uses `.insights/` for research, triage docs, specs, plans, and personal notes
managed by the `insights` CLI.

**At the start of brainstorming, spec writing, or planning work**, dispatch the
`insights-locator` agent to check for prior context before proceeding. Use
`insights-analyzer` to read the most relevant documents. Use the `insights-research`
skill to orchestrate both and save a research document.

Directory layout:
- `.insights/issues/` — triage documents (IB-XX-triage-*.md)
- `.insights/shared/specs/` — specs (IB-XX-spec-*.md)
- `.insights/shared/plans/` — plans (IB-XX-plan-*.md)
- `.insights/shared/research/` — research documents
- `.insights/{USER}/` — personal notes
- `.insights/searchable/` — hardlink mirror for grep/search (read-only; strip "searchable/"
  from any path before reporting or editing)

All `.insights/` artifact files must include YAML front-matter.
See `.insights/shared/schema.md` for the full schema and vocabulary.
"#;
    template.replace("{USER}", user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_root_claude_md_when_neither_exists() {
        let dir = tempfile::tempdir().unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("## Insights"), "missing ## Insights heading");
        assert!(content.contains(".insights/alice/"), "missing user-specific path");
        assert!(!dir.path().join(".claude/CLAUDE.md").exists(), "should not create .claude/CLAUDE.md");
    }

    #[test]
    fn appends_to_dot_claude_claude_md_when_it_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::write(dir.path().join(".claude/CLAUDE.md"), "# My Project\n\nExisting content.\n").unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let dot_content = std::fs::read_to_string(dir.path().join(".claude/CLAUDE.md")).unwrap();
        assert!(dot_content.contains("# My Project"), "original content lost");
        assert!(dot_content.contains("## Insights"), "missing ## Insights heading");
        assert!(dot_content.contains(".insights/alice/"), "missing user-specific path");
        assert!(!dir.path().join("CLAUDE.md").exists(), "should not create root CLAUDE.md");
    }

    #[test]
    fn prefers_dot_claude_over_root_when_both_exist() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::write(dir.path().join(".claude/CLAUDE.md"), "# Dot Claude\n").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Root\n").unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let dot_content = std::fs::read_to_string(dir.path().join(".claude/CLAUDE.md")).unwrap();
        let root_content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(dot_content.contains("## Insights"), ".claude/CLAUDE.md should be updated");
        assert!(!root_content.contains("## Insights"), "root CLAUDE.md should not be touched");
    }

    #[test]
    fn appends_to_root_claude_md_when_only_root_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# My Project\n\nSome existing content.\n").unwrap();
        ensure_claude_md_insights_section(dir.path(), "bob").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("# My Project"), "original content lost");
        assert!(content.contains("## Insights"), "missing ## Insights heading");
        assert!(content.contains(".insights/bob/"), "missing user-specific path");
    }

    #[test]
    fn idempotent_when_run_twice() {
        let dir = tempfile::tempdir().unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        let count = content.matches("## Insights").count();
        assert_eq!(count, 1, "## Insights heading should appear exactly once, got {count}");
    }

    #[test]
    fn idempotent_for_dot_claude_when_section_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let original = "# My Project\n\n## Insights\n\nCustom insights section.\n";
        std::fs::write(dir.path().join(".claude/CLAUDE.md"), original).unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join(".claude/CLAUDE.md")).unwrap();
        assert_eq!(content, original, "file should be unchanged when ## Insights already present");
    }

    #[test]
    fn skips_when_heading_already_in_root_file() {
        let dir = tempfile::tempdir().unwrap();
        let original = "# My Project\n\n## Insights\n\nCustom insights section.\n";
        std::fs::write(dir.path().join("CLAUDE.md"), original).unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(content, original, "file should be unchanged when ## Insights already present");
    }

    #[test]
    fn adds_frontmatter_hint_when_insights_section_exists_without_hint() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# My Project\n\n## Insights\n\nSome insights content.\n").unwrap();
        ensure_claude_md_frontmatter_hint(dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("schema.md"), "front-matter hint not added");
        assert!(content.contains("## Insights"), "original content lost");
    }

    #[test]
    fn frontmatter_hint_idempotent_when_already_present() {
        let dir = tempfile::tempdir().unwrap();
        let original = "# My Project\n\n## Insights\n\nSee `.insights/shared/schema.md` for the full schema and vocabulary.\n";
        std::fs::write(dir.path().join("CLAUDE.md"), original).unwrap();
        ensure_claude_md_frontmatter_hint(dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(content, original, "file should be unchanged when hint already present");
    }

    #[test]
    fn frontmatter_hint_no_op_when_no_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        ensure_claude_md_frontmatter_hint(dir.path()).unwrap(); // must not error
        assert!(!dir.path().join("CLAUDE.md").exists(), "should not create CLAUDE.md");
    }

    #[test]
    fn adds_frontmatter_hint_to_dot_claude_claude_md() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::write(
            dir.path().join(".claude/CLAUDE.md"),
            "# My Project\n\n## Insights\n\nSome insights content without the hint.\n",
        )
        .unwrap();
        ensure_claude_md_frontmatter_hint(dir.path()).unwrap();
        let content = std::fs::read_to_string(dir.path().join(".claude/CLAUDE.md")).unwrap();
        assert!(content.contains("schema.md"), "front-matter hint not added to .claude/CLAUDE.md");
        assert!(content.contains("## Insights"), "original ## Insights heading lost");
        assert!(!dir.path().join("CLAUDE.md").exists(), "should not create root CLAUDE.md");
    }
}
