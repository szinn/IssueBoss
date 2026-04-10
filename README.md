# IssueBoss

An AI-native issue tracker built around a structured pipeline and an MCP server, so agents can read, update, and advance issues directly without leaving their tools.

## What it does

Issues move through a defined pipeline — triage, research, spec, plan, dev — with explicit gates that enforce process. Gates block transitions until required artifacts exist (e.g. a triage result before leaving triage, a spec before leaving spec review). Phases can be skipped when they aren't needed.

Artifacts attach structured documents to issues: triage results, specs, plans, research notes, handoff summaries, comments. File-backed artifacts track on-disk paths so `move_artifact` can keep references in sync when files move.

## MCP Interface

The primary interface is an MCP server that exposes the full issue lifecycle to AI agents:

- `list_issues` / `create_issue` / `update_issue`
- `transition_issue` — advance through the pipeline
- `add_artifact` / `update_artifact` / `remove_artifact` / `list_artifacts`
- `move_artifact` — update path references across all issues when a file moves
- Resources: `issueboss://projects`, `issueboss://issues/{slug}`

## Pipeline

```
TriageNeeded → TriageInProgress → TriageReview → ResearchNeeded → ResearchInProgress → ResearchReview
                                               → SpecNeeded → SpecInProgress → SpecReview
                                               → PlanNeeded → PlanInProgress → PlanReview
                                               → DevNeeded → DevInProgress → DevReview → Done
```

Phases are a DAG — after any Review state you can jump directly to any later phase or Done. Backlog and Canceled are reachable from most states.

## Tech Stack

- **Rust** — hexagonal (ports & adapters) architecture, dependencies point inward toward `ib-core`
- **Database** — SeaORM with Postgres and SQLite support
- **API** — MCP server (`rmcp`) + gRPC admin interface (`tonic`)
- **Frontend** — Dioxus

## Development

```sh
just build          # build all crates
just run            # run the server
just fmt            # format
just clippy         # lint
just component-tests    # unit + component tests (no Docker)
just integration-tests  # full integration tests (requires colima)
colima start        # start Docker for integration tests
```
