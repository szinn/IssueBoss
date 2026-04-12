---
name: research
description: Ad-hoc research skill — investigate any topic (no IssueBoss issue required) by dispatching research-topic-processor, presenting findings inline, and optionally saving a document to .insights/shared/research/.
argument-hint: [topic or query]
---

# Research

You are using the `research` skill to investigate a free-form topic and present findings inline.
No IssueBoss issue is required — this skill works for any ad-hoc query.

## Trigger Phrases

This skill applies when:

- User says "research [topic]", "look into [topic]", "investigate [topic]", or "find out about [topic]"
- User invokes `/research` with a query
- Another skill needs free-form research outside the context of a specific issue

## Inputs (from caller)

- **topic** (required) — the research question or subject; use the argument provided, or ask: "What would you like me to research?"
- **tags** (optional) — hashtag strings to steer dimension selection, e.g. `["#codebase", "#web"]`. Valid tags: `#codebase`, `#implementation`, `#architecture`, `#patterns`, `#conventions`, `#examples`, `#web`, `#external`, `#library`, `#practices`, `#web-deep`. Pass `[]` if none are provided — the processor infers dimensions from the topic description.

## VCS Context Resolution

Before dispatching the processor, resolve the current VCS context from the project root.

1. Check whether `.jj/` exists in the project root.

2. **If jj repo:** Run:

   ```
   jj log -r @ --no-graph -T 'change_id ++ " " ++ commit_id'
   ```

   Split on the first space: first token = `change_id`, second token = `commit`.

3. **If git repo (no `.jj/`):** Run `git rev-parse HEAD` → `commit`; set `change_id = ""`.

4. **If neither:** Set `change_id = ""` and `commit = ""`.

## Research Flow

1. **Resolve VCS context** — follow the VCS Context Resolution section above.

2. **Dispatch `issueboss:research-topic-processor`** via the Agent tool with this exact prompt format:

   ```
   Research the following topic:
   topic_description: {topic}
   topic_tags: {tags as JSON array, or [] if none}
   change_id: {change_id}
   commit: {commit}
   project_root: {absolute path to project root}
   ```

   Note: do NOT include an `issue_slug` line — this is an ad-hoc invocation.

3. **Wait for the processor** to return its structured output (delimited by `---FRONTMATTER---`, `---FINDINGS---`, `---END---`).

4. **Parse the output:**
   - Extract the YAML block between `---FRONTMATTER---` and `---FINDINGS---` as frontmatter fields.
   - Extract everything between `---FINDINGS---` and `---END---` as the findings body.
   - If either delimiter is missing, inform the user: "The research processor returned unexpected output — findings cannot be displayed." Do not attempt to save or present any document.

5. **Present findings inline** — display the full findings body to the user as-is.

6. **Offer to save** — after presenting, ask:

   > Want me to save this as a research document in `.insights/shared/research/`?

   Do **not** save automatically. Wait for an explicit yes. If the user declines or gives no clear answer, do not write any file.

## Saving the Document (if user says yes)

1. **Derive the filename** from the frontmatter `topic` field:
   - Split `topic` into space-delimited tokens; take the first 6 (or all tokens if fewer than 6)
   - Lowercase everything
   - Replace any non-alphanumeric characters (including spaces) with hyphens
   - Collapse consecutive hyphens to a single hyphen
   - Strip leading and trailing hyphens
   - Result: `{kebab-summary}`

2. **Write the file** to `.insights/shared/research/{kebab-summary}.md` using the Write tool.
   Do NOT run `mkdir` — the Write tool creates parent directories automatically.

   File content format:

   ```
   ---
   topic: {topic value from frontmatter}
   date: {date value from frontmatter}
   status: {status value from frontmatter}
   change_id: {change_id value from frontmatter}
   commit: {commit value from frontmatter}
   dimensions_active: {dimensions_active value from frontmatter}
   ---
   {findings body exactly as returned by the processor}
   ```

3. **Confirm** the path to the user:

   > Saved to `.insights/shared/research/{kebab-summary}.md`

## Constraints

- **No issueboss MCP dependency** — do not call any `mcp__issueboss__*` tools.
- **No `.claude/issueboss.json`** — do not read project config; derive all context from the query and VCS.
- **Never save automatically** — always ask before writing any document.
- **No `issue_slug`** — never include an `issue_slug` line in the processor prompt for this skill.
