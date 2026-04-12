---
name: issueboss
description: Use when the user mentions issues, asks what to work on, references a backlog, asks about issue status or pipeline state, wants to find ready work, or references an issue slug (e.g. IB-42). Covers all IssueBoss MCP interactions including triage, artifact management, and pipeline advancement.
---

## Config

**MANDATORY FIRST STEP — do this before any MCP call.** Read `.claude/issueboss.json` from the project root using the Read tool. MCP calls made without this will fail with misleading "project not found" errors because the wrong `project_slug` will be used.

Required field: `project_slug` — use this as the default for every MCP call that takes a project slug.
Optional fields: `server` (informational), `insights_dir` (doc root).

If the file is missing, stop and ask the user to create it.

## Tools

list_issues — filter by status/priority/size/limit
create_issue — new issue
update_issue — title/description/priority/size
transition_issue — advance pipeline status
add_artifact — attach artifact to issue
update_artifact — update artifact body by slug
remove_artifact — remove artifact by slug
list_artifacts — list artifacts, optional kind filter
move_artifact — update path field across all artifacts referencing a file

Resources: issueboss://projects, issueboss://issues/{slug}

## Pipeline

DAG — phases can be skipped when not needed. After any Review state, jump directly to any later phase or Done.

TriageNeeded→TriageInProgress→TriageReview→(any later phase or Done)
ResearchNeeded→ResearchInProgress→ResearchReview→(SpecNeeded or later)
SpecNeeded→SpecInProgress→SpecReview→(PlanNeeded or later)
PlanNeeded→PlanInProgress→PlanReview→(DevNeeded or Done)
DevNeeded→DevInProgress→DevReview→Done
Backlog and Canceled reachable from most states.

Gated transitions — artifact must exist before transition succeeds:
TriageInProgress→TriageReview requires TriageResult
SpecInProgress→SpecReview requires Spec
PlanInProgress→PlanReview requires Plan
ResearchInProgress→ResearchReview requires all ResearchTopics covered

## Artifacts

**NOTE:** The `.insights` directory is not under version control so no `git`/`jj` actions should be applied to files created/edited in `.insights`.

File-backed (TriageResult, Spec, Plan, Research, Handoff): path immutable after creation; use move_artifact if file moves.
Singleton (TriageResult, Spec, Plan): auto-assigned slug, one per issue.
Caller-slug (Research, ResearchTopic, Comment, Handoff): caller provides slug — lowercase letters, digits, hyphens only.

TriageResult: singleton (slug "triage"), {path}, gates TriageInProgress→TriageReview, path={insights_dir}/issues/{issue-slug}-triage-{kebab-summary}.md
Spec: singleton (slug "spec"), {path}, gates SpecInProgress→SpecReview, path={insights_dir}/shared/specs/{issue-slug}-spec-{kebab-summary}.md
Plan: singleton (slug "plan"), {path}, gates PlanInProgress→PlanReview, path={insights_dir}/shared/plans/{issue-slug}-plan-{kebab-summary}.md

**File naming:** `{issue-slug}` in paths must use the issue's slug exactly as returned by the API — preserve the project prefix casing (e.g. `IB-3-plan.md`, not `ib-3-plan.md`).
Research: caller slug, {topic_token, status, path}, covers a ResearchTopic, path={insights_dir}/shared/research/{kebab-summary}.md
**topic_token must be the artifact TOKEN of the ResearchTopic** (e.g. `A_8GWH1GYSR30P4`), not its slug.
Call list_artifacts to retrieve ResearchTopic tokens before adding Research artifacts.
ResearchTopic: caller slug, {description} or {path}, uncovered topics block ResearchReview
Comment: caller slug, {text}
Handoff: caller slug, {path}, file-backed, move_artifact applies, path={insights_dir}/issues/{issue-slug}-handoff.md
StatusTransition: system-generated only — do not create manually

## Workflow

### Triage

Delegate to the `issueboss:triage` agent via the Agent tool.

> **Permission requirement:** The triage agent writes files to `.insights/`. Before dispatching, ensure the current session has file write permissions enabled — otherwise the agent will fail when it attempts to write the triage document.

- **Single issue:** Dispatch one background agent — `subagent_type: issueboss:triage`, `run_in_background: true`, prompt containing "Triage issue {slug}." followed by any context the user provided. When it completes, display the returned summary and ask the user which phase to advance to.
- **Multiple issues:** Dispatch one background agent per issue in a single message (all Agent tool calls in the same response), each with prompt "Triage issue {slug}." followed by any relevant context. Display summaries grouped by issue when all complete; ask the user which phase to advance to for each.
- **No issues to triage:** If there are no TriageNeeded issues, inform the user and do nothing.
- **Foreground override:** If the user explicitly asks to wait (e.g., "wait for it", "don't background it", "triage IB-42 synchronously"), omit `run_in_background`.
- **Agent errors:** If the agent returns an error (e.g., `Error (Step 3): Issue {slug} is in {status}, not TriageNeeded`), display it to the user as-is and do not attempt retry.

### Spec phase

1. transition→SpecInProgress
2. Invoke `superpowers-extended-cc:brainstorming` to write the spec
3. The spec file MUST begin with YAML front-matter per `.insights/shared/schema.md`:

   <!-- prettier-ignore -->
   ```yaml
   ---
   type: spec
   issue: {slug}
   status: draft
   created: {YYYY-MM-DD}
   size: {XS|S|M|L|XL}
   risk: {low|medium|high}
   tags:
     - {relevant-tags}
   summary: >
     {one paragraph}
   ---
   ```

4. add_artifact kind=Spec with path
5. **Immediately** transition→SpecReview
6. When the user approves the spec and advances past SpecReview: update the spec file's
   front-matter `status` field from `draft` to `approved`, then transition the issue.

### Plan phase

1. transition→PlanInProgress
2. Invoke `superpowers-extended-cc:writing-plans` to write the plan
3. The plan file MUST begin with YAML front-matter per `.insights/shared/schema.md`:

   <!-- prettier-ignore -->
   ```yaml
   ---
   type: plan
   issue: {slug}
   status: draft
   created: {YYYY-MM-DD}
   size: {XS|S|M|L|XL}
   risk: {low|medium|high}
   summary: >
     {one paragraph}
   ---
   ```

4. add_artifact kind=Plan with path
5. **Immediately** transition→PlanReview
6. When the user approves the plan and advances past PlanReview: update the plan file's
   front-matter `status` field from `draft` to `approved`, then transition the issue.

### Phase advancement

1. transition→{Phase}InProgress
2. Do the work
3. add_artifact with relevant kind+path
4. **Immediately** transition→{Phase}Review — do NOT wait for user instruction; the gate clears as soon as the artifact exists
5. Present summary; ask user which phase to advance to next (or Done)

### Dev phase integration

When the dev phase uses `superpowers-extended-cc:finishing-a-development-branch` or `superpowers-extended-cc:executing-plans` to complete work, those skills drive the final steps. **You must still transition the issue to DevReview** as soon as the finishing skill presents its completion options to the user — do not wait for the user to ask. This is the same rule as step 4 above; it applies even when another skill is running the completion flow.

The "immediately transition to Review" rule is an invariant, not a step in a linear flow — it holds regardless of which skill is running at the moment work completes.

### Research phase

Delegate to the `issueboss:research` agent via the Agent tool.

> **Permission requirement:** The research agent writes files to `.insights/`. Before dispatching, ensure the current session has file write permissions enabled — otherwise the agent will fail when it attempts to write research documents.

- **Single issue:** Dispatch one background agent — `subagent_type: issueboss:research`, `run_in_background: true`, prompt containing "Research issue {slug}." followed by any context the user provided. When it completes, display the returned summary and ask the user which phase to advance to.
- **Multiple issues:** Dispatch one background agent per issue in a single message (all Agent tool calls in the same response), each with prompt "Research issue {slug}." followed by any relevant context. Display summaries grouped by issue when all complete; ask the user which phase to advance to for each.
- **Foreground override:** If the user explicitly asks to wait (e.g., "wait for it", "don't background it", "research IB-42 synchronously"), omit `run_in_background`.
- **Agent errors:** If the agent returns an error (e.g., `Error (Step 2): Issue {slug} is in {status}, not ResearchNeeded`), display it to the user as-is and do not attempt retry.
- **Ad-hoc research:** For free-form research queries not tied to an issue pipeline step, invoke the `research` skill directly — no IssueBoss MCP interaction is needed.

When covering ResearchTopics with Research artifacts:

1. Call list_artifacts (kinds=["ResearchTopic"]) to get each topic's `token` field
2. For each Research artifact, set `topic_token` to the ResearchTopic's `token` value (e.g. `A_8GWH1GYSR30P4`) — **not** the slug
3. All ResearchTopics must be covered before ResearchInProgress→ResearchReview gate passes
4. Research files MUST begin with YAML front-matter per `.insights/shared/schema.md`

### Walking a completed issue to Done

Read current status → walk forward → add required artifacts at gates → skip phases not needed → advance to Done.

### Displaying an issue

Show issue fields (slug, title, status, priority, size, description, submitter full_name, assigned full_name or "unassigned"), then a time-ordered list of artifacts (oldest first). Exclude StatusTransition artifacts unless their `reason` is exceptional (non-routine context worth surfacing). For each artifact show kind, slug, datestamp, creator, and relevant body fields.

**Artifact creator:** Each artifact has a `created_by` field containing a user token or `"system"`. Resolve the token to a name by matching against the issue's `submitter` and `assigned` fields. If no match is found, display the raw token. System-generated artifacts (StatusTransition) show "system" and are excluded from display per the rule above.

### Listing issues

Show slug, status, priority, title. Lead with actionable states (TriageNeeded, in-progress) before blocked/low-priority. **Do not include Backlog or Canceled issues** unless the user explicitly asks for them.

**Open issues** are those in any pipeline state except Done, Backlog, and Canceled. Backlog and Canceled are neither open nor done — Backlog is deprioritized, Canceled is abandoned. Never describe them as open or resolved when summarizing project status.
