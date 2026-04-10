---
name: issueboss
description: Use when the user mentions issues, asks what to work on, references a backlog, asks about issue status or pipeline state, wants to find ready work, or references an issue slug (e.g. IB-42). Covers all IssueBoss MCP interactions including triage, artifact management, and pipeline advancement.
---

## Config

Read `.claude/issueboss.json` from project root. Required: `project_slug` (default for all MCP calls). Optional: `server` (informational), `insights_dir` (doc root). If missing, stop and ask user to create it.

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

File-backed (TriageResult, Spec, Plan, Research, Handoff): path immutable after creation; use move_artifact if file moves.
Singleton (TriageResult, Spec, Plan): auto-assigned slug, one per issue.
Caller-slug (Research, ResearchTopic, Comment, Handoff): caller provides slug — lowercase letters, digits, hyphens only.

TriageResult: singleton (slug "triage"), {path}, gates TriageInProgress→TriageReview, path={insights_dir}/issues/{issue-slug}-triage.md
Spec: singleton (slug "spec"), {path}, gates SpecInProgress→SpecReview, path={insights_dir}/shared/specs/{issue-slug}-spec.md
Plan: singleton (slug "plan"), {path}, gates PlanInProgress→PlanReview, path={insights_dir}/shared/plans/{issue-slug}-plan.md
Research: caller slug, {topic_token, status, path}, covers a ResearchTopic, path={insights_dir}/shared/research/{topic-slug}.md
ResearchTopic: caller slug, {description} or {path}, uncovered topics block ResearchReview
Comment: caller slug, {text}
Handoff: caller slug, {path}, file-backed, move_artifact applies, path={insights_dir}/issues/{issue-slug}-handoff.md
StatusTransition: system-generated only — do not create manually

## Workflow

### Triage

1. Read issue via issueboss://issues/{slug}
2. transition→TriageInProgress
3. Investigate scope (code, existing artifacts)
4. Determine size/risk/phases needed
5. add_artifact kind=TriageResult with path to triage doc
6. transition→TriageReview
7. Present summary; ask user which phase to advance to

> Phase-specific guidance (research, spec, plan, dev skills) to be added.

### Phase advancement

1. transition→{Phase}InProgress
2. Do the work
3. add_artifact with relevant kind+path
4. transition→{Phase}Review (gate clears once artifact exists)
5. transition→next phase or Done

### Walking a completed issue to Done

Read current status → walk forward → add required artifacts at gates → skip phases not needed → advance to Done.

### Displaying an issue

Show issue fields (slug, title, status, priority, size, description), then a time-ordered list of artifacts (oldest first). Exclude StatusTransition artifacts unless their `reason` is exceptional (non-routine context worth surfacing). For each artifact show kind, slug, datestamp, and relevant body fields.

### Listing issues

Show slug, status, priority, title. Lead with actionable states (TriageNeeded, in-progress) before blocked/low-priority.
