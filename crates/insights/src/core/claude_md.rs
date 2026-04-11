use std::path::Path;

use anyhow::{Context, Result};

const INSIGHTS_HEADING: &str = "## Insights";

/// Appends an `## Insights` section to the project's CLAUDE.md.
///
/// Idempotent: if `## Insights` already appears in the file the function
/// is a no-op. Safe to call from `insights init` on every run.
pub fn ensure_claude_md_insights_section(project_root: &Path, user: &str) -> Result<()> {
    let claude_md = project_root.join("CLAUDE.md");

    let existing = if claude_md.exists() {
        std::fs::read_to_string(&claude_md).with_context(|| format!("Failed to read '{}'", claude_md.display()))?
    } else {
        String::new()
    };

    if existing.contains(INSIGHTS_HEADING) {
        return Ok(());
    }

    let snippet = insights_snippet(user);

    let content = if existing.is_empty() {
        snippet
    } else {
        format!("{}\n\n{}", existing.trim_end(), snippet)
    };

    std::fs::write(&claude_md, content).with_context(|| format!("Failed to write '{}'", claude_md.display()))?;

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
"#;
    template.replace("{USER}", user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_claude_md_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("## Insights"), "missing ## Insights heading");
        assert!(content.contains(".insights/alice/"), "missing user-specific path");
    }

    #[test]
    fn appends_to_existing_claude_md() {
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
    fn skips_when_heading_already_in_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let original = "# My Project\n\n## Insights\n\nCustom insights section.\n";
        std::fs::write(dir.path().join("CLAUDE.md"), original).unwrap();
        ensure_claude_md_insights_section(dir.path(), "alice").unwrap();
        let content = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert_eq!(content, original, "file should be unchanged when ## Insights already present");
    }
}
