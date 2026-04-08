---
name: issueboss
description: Use when the user mentions issues, asks what to work on, references a backlog, asks about issue status or pipeline state, wants to find ready work, or references an issue slug (e.g. IB-42). Covers all IssueBoss MCP interactions.
---

## Step 1: Load Configuration

Read `.claude/issueboss.json` from the project root.

```json
{
  "project_slug": "ib",
  "server": "https://issueboss.example.com",
  "thoughts_dir": "thoughts/"
}
```

| Field          | Purpose                                                                 |
| -------------- | ----------------------------------------------------------------------- |
| `project_slug` | Default project for all MCP tool calls                                  |
| `server`       | IssueBoss server URL (informational — MCP connection is pre-configured) |
| `thoughts_dir` | Root directory for research, plans, and other workflow documents        |

If `.claude/issueboss.json` does not exist, stop and tell the user:

> No IssueBoss configuration found. Create `.claude/issueboss.json` with at minimum
> a `project_slug` field matching your IssueBoss project.

Do not proceed without a valid config.

## Step 2: Confirm MCP Availability

The IssueBoss MCP tools are available in this session. The `project_slug` from the
config is the default for all MCP tool calls unless the user specifies otherwise.

Current tool surface:

| Tool               | Purpose                                                |
| ------------------ | ------------------------------------------------------ |
| `list_issues`      | Query issues — filter by status, priority, size, limit |
| `create_issue`     | Create a new issue in the project                      |
| `update_issue`     | Update issue title, description, priority, or size     |
| `transition_issue` | Move an issue to a new pipeline status                 |

Resources (readable via MCP):

- `issueboss://projects` — all projects accessible to this API key
- `issueboss://issues/{slug}` — a single issue by slug (e.g. `issueboss://issues/IB-42`)

## Step 3: Answer or Act

With config loaded and MCP confirmed, handle the user's request directly.

If the user's intent is unclear — e.g. "check on issues" or "what should I work on" —
query `list_issues` with a sensible default filter and present the results before asking
follow-up questions.
