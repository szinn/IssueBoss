# IssueBoss

An AI-native issue tracker built around a structured pipeline and an MCP server, so agents can read, update, and advance issues directly without leaving their tools.

## What it does

Issues move through a defined pipeline — triage, research, spec, plan, dev — with explicit gates that enforce process. Gates block transitions until required artifacts exist (e.g. a triage result before leaving triage, a spec before leaving spec review). Phases can be skipped when they aren't needed.

Artifacts attach structured documents to issues: triage results, specs, plans, research notes, handoff summaries, comments. File-backed artifacts track on-disk paths so `move_artifact` can keep references in sync when files move.

## MCP Interface

The primary interface is an MCP server that exposes the full issue lifecycle to AI agents.

### Tools

**Issues**

- `list_issues` — list issues with optional filters: `status`, `priority`, `size`, `limit`, `exclude_blocked`, `submitted_by`, `assigned_to`
- `create_issue` — create a new issue; priority defaults to Medium
- `update_issue` — update title, description, priority, or size
- `transition_issue` — advance through the pipeline with an optional `reason`

**Artifacts**

- `add_artifact` — attach an artifact to an issue; kinds: `TriageResult`, `Spec`, `Plan`, `Research`, `ResearchTopic`, `Comment`, `Handoff`
- `update_artifact` — update an artifact's body (StatusTransition artifacts and file path fields are immutable)
- `remove_artifact` — remove an artifact by slug
- `list_artifacts` — list artifacts, with optional `kinds` filter and `uncovered_only` flag for uncovered ResearchTopics
- `move_artifact` — update path references across all issues when a file moves on disk (admin only)

**Relationships**

- `add_relationship` — link two issues with `DependsOn` (with cycle detection) or `RelatedTo` (symmetric)
- `remove_relationship` — remove a relationship by kind
- `list_relationships` — list `depends_on`, `blocks`, and `related_to` for an issue

### Resources

- `issueboss://projects` — list all accessible projects
- `issueboss://issues/{slug}` — read a single issue with its relationships (e.g. `issueboss://issues/IB-5`)

## Pipeline

```
TriageNeeded → TriageInProgress → TriageReview → ResearchNeeded → ResearchInProgress → ResearchReview
                                               → SpecNeeded → SpecInProgress → SpecReview
                                               → PlanNeeded → PlanInProgress → PlanReview
                                               → DevNeeded → DevInProgress → DevReview → Done
```

Phases are a DAG — after any Review state you can jump directly to any later phase or Done. Backlog and Canceled are reachable from most states.

**Gated transitions** require artifacts to exist before they pass:

| Transition                          | Required artifact          |
| ----------------------------------- | -------------------------- |
| TriageInProgress → TriageReview     | TriageResult               |
| SpecInProgress → SpecReview         | Spec                       |
| PlanInProgress → PlanReview         | Plan                       |
| ResearchInProgress → ResearchReview | all ResearchTopics covered |

## Tech Stack

- **Rust** — hexagonal (ports & adapters) architecture, dependencies point inward toward `ib-core`
- **Database** — SeaORM with Postgres, MySQL, MariaDB, and SQLite support
- **API** — MCP server (`rmcp`) + gRPC admin interface (`tonic`)
- **Frontend** — Dioxus

## Development

```sh
just build          # build all crates
just run            # run the server
just bundle         # bundle web + server components
just run-bundle     # run the bundled binary
just fmt            # format code and documentation
just lint           # run all lint checks
just clippy         # run Clippy directly
```

**Testing**

```sh
just component-tests            # unit + component tests (no Docker)
just quick-test                 # component + postgres tests
just test                       # all tests
just integration-tests          # all integration tests (requires colima)
just postgres-integration-tests # Postgres only
just sqlite-integration-tests   # SQLite only
just mysql-integration-tests    # MySQL only
just mariadb-integration-tests  # MariaDB only
just insta                      # run snapshot tests
just insta-review               # review snapshot deltas
colima start                    # start Docker for integration tests
colima stop                     # stop Docker
```

**Database**

```sh
just create-database  # create the Postgres database
just database         # database admin
just config           # edit encrypted configuration
```

**Documentation & releases**

```sh
just docs-build       # build documentation as a static site
just docs-serve       # serve docs locally with live reload
just changelog        # update CHANGELOG.md
just release VERSION  # create a release
```

**Tooling**

```sh
just install-tools    # install contributing toolchain
just deps             # update Rust crate dependencies
just buf              # run proto lint
just tailwindcss      # update Tailwind CSS
just clean            # clean the workspace
```

## Insights

IssueBoss integrates with a shared [Insights](https://github.com/szinn/Insights) git repository — a central knowledge store for triage documents, specs, plans, and research notes. The `insights` CLI manages the connection between a project and the repo.

```sh
insights init --repo <path> --user <username> --project <name>
```

Sets up `.insights/` in the project root with symlinks into the shared repo:

- `.insights/issues/` → per-project triage docs and handoffs
- `.insights/shared/` → specs, plans, and research shared across projects
- `.insights/<username>/` → personal notes
- `.insights/searchable/` → hard-link mirror, safe for `rg`/`grep` (read-only by convention)

`.insights/` is not version-controlled — it is added to `.gitignore` automatically on init.

```sh
insights sync    # pull latest from the Insights repo and rebuild local state
insights status  # sync and show pending changes
insights commit  # sync and commit all pending changes to the Insights repo
```

All commands accept `-v` / `--verbose` for detailed output.
