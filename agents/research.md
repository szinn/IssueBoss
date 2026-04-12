---
name: research
description: |
  IssueBoss research orchestrator agent. Dispatched by the issueboss skill (background by default).
  Receives an issue slug, iterates through uncovered ResearchTopic artifacts sequentially via the
  research-topic-processor agent, writes Research docs, attaches Research artifacts, then transitions
  to ResearchReview. Returns a brief summary distinguishing newly-researched vs already-covered topics.
model: inherit
---

## Prerequisites

The parent session must have **file write permissions enabled** before dispatching this agent. The research agent writes research docs to `.insights/` — if the parent session has not allowed edits/writes, the agent will fail when writing research documents.

## Config

Read `.claude/issueboss.json` from the project root. Required fields:

| Field          | Purpose                                |
| -------------- | -------------------------------------- |
| `project_slug` | Default project for all MCP tool calls |
| `insights_dir` | Root directory for research documents  |

If the file or either field is missing, halt immediately and return:

> Error (Step 1): `.claude/issueboss.json` not found or missing required fields (`project_slug`, `insights_dir`).

## Tools

transition_issue — advance pipeline status
add_artifact — attach artifact to issue
list_artifacts — list existing artifacts on an issue
Read, Write — file system access (built-in)
Bash — VCS context detection
Agent — dispatch research-topic-processor sub-agent

Resources: issueboss://issues/{slug}

## Pipeline

ResearchNeeded → ResearchInProgress → ResearchReview

Gated transition: ResearchInProgress → ResearchReview requires all ResearchTopic artifacts to be covered by a matching Research artifact (linked via `topic_token`).

## Artifacts

**NOTE:** `.insights` is not under version control — do not run `jj`/`git` on files in `.insights`.

Research: caller slug (`{kebab-summary}`), body `{"topic_token": "<token>", "status": "complete", "path": "<path>"}`, path=`{insights_dir}/shared/research/{kebab-summary}.md`

**`topic_token` must be the artifact TOKEN of the ResearchTopic** (e.g. `A_8GWH1GYSR30P4`), not its slug. The token is the `token` field returned by `list_artifacts`.

## Research Workflow

The issue slug is provided in the incoming prompt as the first argument (e.g. `Research issue IB-19.`). Extract it from the prompt before beginning.

1. Read `.claude/issueboss.json` — extract `project_slug` and `insights_dir`

2. Read issue via `issueboss://issues/{slug}` — verify status is `ResearchNeeded`. If not, halt and return:

   > Error (Step 2): Issue `{slug}` is in `{status}`, not `ResearchNeeded`. Research aborted.

3. `list_artifacts(kind=ResearchTopic)` — get all ResearchTopic artifacts with their `token`, `slug`, and body fields (`description`, `path`, `tags`)

   If there are no ResearchTopic artifacts, halt and return **before transitioning**:

   > Error (Step 3): Issue `{slug}` has no ResearchTopic artifacts. Add at least one ResearchTopic before starting research.

4. `transition_issue` → `ResearchInProgress`

5. `list_artifacts(kind=Research)` — get all existing Research artifacts; parse each artifact's body as JSON and read the `topic_token` key to get the covered token value

6. Compute **uncovered topics**: ResearchTopic artifacts whose `token` does not appear in any Research artifact's `topic_token` field

7. If all topics are already covered (uncovered list is empty), skip to Step 11 — do not re-research already-covered topics

8. Resolve VCS context ONCE before processing any topics:
   - Check if `.jj/` exists in the project root
   - **If jj repo:** Run `jj log -r @ --no-graph -T 'change_id ++ " " ++ commit_id'` — split the output on the first space: first token = `change_id`, second token = `commit`
   - **If git repo:** Run `git rev-parse HEAD` → `commit`; set `change_id = ""`
   - **If neither:** Set both `change_id = ""` and `commit = ""`

9. For each uncovered topic (process sequentially, one at a time):

   a. Read the topic description:
   - If the topic body has a `description` field, use it directly
   - If the topic body has a `path` field, read the file at that path to get the description

   b. Extract tags from the topic body `tags` field (default to `[]` if absent)

   c. Dispatch `issueboss:research-topic-processor` via the Agent tool with the following prompt:

   ```
   Research the following topic:
   topic_description: {description}
   topic_tags: {tags as JSON array}
   change_id: {change_id}
   commit: {commit}
   project_root: {absolute path to project root}
   issue_slug: {slug}
   ```

   d. Parse the processor's return output:
   - Find `---FRONTMATTER---` marker — content between it and `---FINDINGS---` is the frontmatter block
   - Find `---FINDINGS---` marker — content between it and `---END---` is the findings body
   - If either delimiter is missing, record: "Warning: topic `{topic.slug}` returned malformed output — skipping artifact creation"

   e. Derive `kebab-summary` from the frontmatter `topic` field:
   - Take the first 6 words, lowercase, replace spaces/non-alphanum with hyphens, strip leading/trailing hyphens
   - Example: "SeaORM migration patterns in PostgreSQL" → `seaorm-migration-patterns-in-postgresql`

   f. Write the research doc using the Write tool:
   Path: `{insights_dir}/shared/research/{kebab-summary}.md`

   Content format — YAML front-matter block followed by findings body:

   ```
   ---
   {frontmatter fields as parsed, e.g.:}
   topic: {topic value from frontmatter}
   date: {date value from frontmatter}
   status: complete
   change_id: {change_id value from frontmatter}
   commit: {commit value from frontmatter}
   dimensions_active: {dimensions_active value from frontmatter}
   ---
   {findings body exactly as returned by the processor}
   ```

   **Do not run mkdir** — use the Write tool directly.

   g. `add_artifact` with:
   - kind: `Research`
   - slug: `{kebab-summary}`
   - body: `{"topic_token": "{topic.token}", "status": "complete", "path": "{insights_dir}/shared/research/{kebab-summary}.md"}`

   **Critical:** `topic_token` must be the ResearchTopic's TOKEN field value (e.g. `A_8GWH1GYSR30P4`), not its slug.

10. Repeat Step 9 for each remaining uncovered topic

11. `transition_issue` → `ResearchReview` — do this immediately, do NOT wait for user instruction

12. Return the summary below

If any step fails, return the error and the step number that failed.

## Return Format

```
{slug} "{title}"
Outcome: <one sentence — what was researched and what was found>
Newly researched: <comma-separated topic slugs/summaries, or "none">
Already covered (skipped): <comma-separated topic slugs/summaries, or "none">
Research docs:
- {insights_dir}/shared/research/{kebab}.md — {topic summary}
- (one line per newly-written doc)
```
