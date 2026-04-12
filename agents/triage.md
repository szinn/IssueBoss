---
name: triage
description: |
  IssueBoss triage agent. Dispatched by the issueboss skill (background by default).
  Receives an issue slug and optional context, performs full triage autonomously —
  reads config, investigates scope, writes triage doc, registers TriageResult artifact,
  transitions to TriageReview — then returns a brief summary to the caller.
model: inherit
---

## Prerequisites

The parent session must have **file write permissions enabled** before dispatching this agent. The triage agent writes a triage doc to `.insights/` — if the parent session has not allowed edits/writes, the agent will fail at Step 8.

## Config

Read `.claude/issueboss.json` from the project root. Required fields:

| Field          | Purpose                                |
| -------------- | -------------------------------------- |
| `project_slug` | Default project for all MCP tool calls |
| `insights_dir` | Root directory for triage documents    |

If the file or either field is missing, halt immediately and return:

> Error (Step 1): `.claude/issueboss.json` not found or missing required fields (`project_slug`, `insights_dir`).

## Tools

transition_issue — advance pipeline status
add_artifact — attach artifact to issue
list_artifacts — list existing artifacts on an issue
Read, Write — file system access (built-in)

Resources: issueboss://issues/{slug}

## Pipeline

TriageNeeded → TriageInProgress → TriageReview

Gated transition: TriageInProgress → TriageReview requires a TriageResult artifact.

## Artifacts

**NOTE:** `.insights` is not under version control — do not run `jj`/`git` on files in `.insights`.

TriageResult: singleton (auto-slug "triage"), requires `{path}`, gates TriageInProgress→TriageReview.
Path format: `{insights_dir}/issues/{issue-slug}-triage-{kebab-summary}.md`
({kebab-summary} = short 3–5 word kebab-case summary derived from the issue title)

**File naming:** Preserve exact API casing of the issue slug (e.g. `IB-19`, not `ib-19`).

## Triage Workflow

The issue slug is provided in the incoming prompt as the first argument (e.g. `Triage issue IB-19.`). Extract it from the prompt before beginning.

1. Read `.claude/issueboss.json` — extract `project_slug` and `insights_dir`
2. Verify `{insights_dir}/issues/` exists on disk — if not, halt and return:
   > Error (Step 2): `{insights_dir}/issues/` does not exist. Create it before triaging.
3. Read issue via `issueboss://issues/{slug}` — verify status is `TriageNeeded`. If not, halt and return:
   > Error (Step 3): Issue `{slug}` is in `{status}`, not `TriageNeeded`. Triage aborted.
4. `transition_issue` → `TriageInProgress`
5. Read complete issue including all attached artifacts: `issueboss://issues/{slug}` and `list_artifacts` (re-read after transition to capture any state changes and full artifact list)
6. Investigate scope: read relevant source files, look for related issues. If the issue describes a bug or unexpected behavior, invoke the `superpowers-extended-cc:systematic-debugging` skill (if available) to guide the investigation.
7. Identify open questions, size (XS/S/M/L), risk (low/medium/high), and phases needed
8. Write triage doc using the Write tool directly — do NOT run mkdir.
   Path: `{insights_dir}/issues/{slug}-triage-{kebab-summary}.md`

   The file MUST begin with this YAML front-matter block (before any prose):

   <!-- prettier-ignore -->
   ```yaml
   ---
   type: triage
   issue: {slug}
   status: complete
   created: {YYYY-MM-DD}
   size: {XS|S|M|L|XL}
   risk: {low|medium|high}
   summary: >
     {one paragraph summary of the issue and recommended phases}
   ---
   ```

   See `.insights/shared/schema.md` for field definitions.

9. `add_artifact` kind=`TriageResult` with `path` pointing to the triage doc
10. `transition_issue` → `TriageReview` — do this immediately, do NOT wait for user instruction
11. Return the summary below

If any step fails, return the error and the step number that failed.

## Return Format

```
{slug} "{title}"
Outcome: <one sentence — what the issue is, what phases are needed>
Size: XS/S/M/L  Risk: low/medium/high
Open questions:
- <question, or "none">
```
