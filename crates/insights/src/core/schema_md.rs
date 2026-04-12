use std::path::Path;

use anyhow::{Context, Result};

const SCHEMA_FILENAME: &str = "schema.md";

/// Writes the front-matter schema reference to `shared_dir/schema.md`.
/// Idempotent — skips if the file already exists.
pub fn ensure_schema_md(shared_dir: &Path, verbose: bool) -> Result<()> {
    let path = shared_dir.join(SCHEMA_FILENAME);
    if path.exists() {
        if verbose {
            println!("schema.md already exists — skipping");
        }
        return Ok(());
    }
    std::fs::write(&path, SCHEMA_CONTENT).with_context(|| format!("Failed to write '{}'", path.display()))?;
    println!("schema.md written: {}", path.display());
    Ok(())
}

#[allow(clippy::needless_raw_string_hashes)]
const SCHEMA_CONTENT: &str = r#"# Insights Artifact Front-matter Schema

All `.insights/` artifact files must begin with a YAML front-matter block.
This schema is shared across all projects using the `insights` CLI.

## Common Fields (all kinds)

```yaml
type: <kind>           # required — see Type Vocabulary below
issue: IB-26           # required for issue-linked docs; omit for standalone research
status: <value>        # required — doc lifecycle state (see Status Vocabulary)
created: YYYY-MM-DD    # required — date the file was first written
updated: YYYY-MM-DD    # optional — date last substantively revised; omit on first write
summary: >             # required — one concise paragraph describing the document
  ...
```

Rules:
- No `id` field — the filename is the canonical identifier
- No internal tokens or database IDs — this file is human- and agent-editable
- `issue` is omitted on standalone research/design docs not tied to a specific issue
- `updated` is omitted on first write; added when the doc is substantively revised

## Kind-specific Fields

| Kind | Additional fields |
|---|---|
| `triage` | `size`, `risk` |
| `spec` | `size`, `risk`, `tags` |
| `plan` | `size`, `risk` |
| `research` / `research-session` / `design` | `tags` |
| `handoff` | _(none)_ |

`size` — issue size estimate: `XS`, `S`, `M`, `L`, `XL`
`risk` — implementation risk: `low`, `medium`, `high`
`tags` — list of lowercase kebab strings for topic categorization

## Type Vocabulary

| Value | Used for |
|---|---|
| `triage` | Triage result documents |
| `spec` | Specification documents |
| `plan` | Implementation plan documents |
| `research` | Directed autonomous research outputs |
| `research-session` | Summaries of interactive research discussions |
| `design` | Design documents produced during the research phase |
| `handoff` | Handoff documents for context transfer |

## Status Vocabulary

| Kind | Values | Notes |
|---|---|---|
| `triage` | `complete` | Always complete when written — point-in-time, not revised |
| `spec` | `draft` → `approved` → `superseded` | `draft` on write; `approved` after human review |
| `plan` | `draft` → `approved` → `superseded` | `draft` on write; `approved` after human review |
| `research` / `research-session` / `design` | `draft` → `complete` | `complete` when finished |
| `handoff` | `complete` | Always complete when written |

## Naming Convention

| Kind | Path |
|---|---|
| Triage | `.insights/issues/{issue-slug}-triage-{kebab}.md` |
| Spec | `.insights/shared/specs/{issue-slug}-spec-{kebab}.md` |
| Plan | `.insights/shared/plans/{issue-slug}-plan-{kebab}.md` |
| Research / Design | `.insights/shared/research/{kebab}.md` |
| Handoff | `.insights/issues/{issue-slug}-handoff.md` |

`{issue-slug}` uses exact API casing (e.g. `IB-26`, not `ib-26`).
`{kebab}` is a 3–5 word kebab-case summary derived from the document title.

## Examples

**Triage:**
```yaml
---
type: triage
issue: IB-26
status: complete
created: 2026-04-12
size: S
risk: low
summary: >
  Define YAML front-matter for .insights/ artifact files so agents can grep
  metadata without reading full content. Low risk, no server changes needed.
---
```

**Spec:**
```yaml
---
type: spec
issue: IB-26
status: draft
created: 2026-04-12
size: S
risk: low
tags:
  - insights
  - schema
summary: >
  Canonical front-matter schema for triage, spec, plan, research, and handoff
  artifact kinds. Written by agents; validated by grep on .insights/searchable/.
---
```

**Plan:**
```yaml
---
type: plan
issue: IB-26
status: approved
created: 2026-04-12
size: S
risk: low
summary: >
  Backfill front-matter and rename non-conforming files across .insights/;
  update insights init to write schema.md and CLAUDE.md front-matter reference.
---
```

**Research / Design:**
```yaml
---
type: design
status: complete
created: 2026-04-05
tags:
  - architecture
  - mcp
summary: >
  IssueBoss overall architecture: single binary on three ports, hexagonal core,
  MCP over Streamable HTTP, gRPC admin, Dioxus frontend.
---
```
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_schema_md_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        ensure_schema_md(dir.path(), false).unwrap();
        let path = dir.path().join("schema.md");
        assert!(path.exists(), "schema.md not written");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("## Type Vocabulary"), "schema missing Type Vocabulary section");
        assert!(content.contains("## Status Vocabulary"), "schema missing Status Vocabulary section");
        assert!(content.contains("## Naming Convention"), "schema missing Naming Convention section");
    }

    #[test]
    fn idempotent_when_schema_md_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("schema.md"), "# Custom\n").unwrap();
        ensure_schema_md(dir.path(), false).unwrap();
        let content = std::fs::read_to_string(dir.path().join("schema.md")).unwrap();
        assert_eq!(content, "# Custom\n", "existing schema.md should not be overwritten");
    }
}
