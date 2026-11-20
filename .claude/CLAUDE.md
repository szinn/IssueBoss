# IssueBoss: Take Control Of Your Project Issues

## Version Control

This is a **jj (jujutsu) repo**. Never use git commands (including `git worktree`).
Use only `jj` commands for all version control operations.

## Commands

- Build: `just build`
- Run: `just run`
- Format: `just fmt`
- Lint: `just clippy`
- Quick tests (component + postgres): `just quick-test`
- All tests: `just test`
- Component tests: `just component-tests`
- Integration tests: `just integration-tests`
- Postgres integration tests: `just postgres-integration-tests`
- SQLite integration tests: `just sqlite-integration-tests`
- Insta tests: `just insta`
- Start colima (for integration/all tests): `colima start`
- Stop colima: `colima stop`

## Architecture

This project follows hexagonal (ports & adapters) architecture. Dependencies point inward
toward the core domain. Never introduce dependencies from `core` to outer crates.

The architecture is the same structure as BookBoss (<https://github.com/szinn/BookBoss>).
If you are going to do something new, checking out BookBoss (@../BookBoss) to see if the
pattern (or code) already exists there and suggest that to the user as an option.

### Core Crate Organization

The core crate uses **domain-based modules** — each domain concept groups its model,
repository trait (port), and service together:

```
crates/core/src/
├── lib.rs              # CoreServices composition root, create_services()
├── error.rs            # Error, ErrorKind, RepositoryError
├── types.rs            # Shared types used across domains (e.g. Capabilities)
├── token.rs            # Generic Token type used for entity tokens
├── repository.rs       # Shared infrastructure: Repository, Transaction traits,
│                       #   RepositoryService, transaction macros, and `mod testing`
│                       #   (MockTransaction, pre-wired MockRepository)
└── api_key/ artifact/ issue/ project/ relationship/ user/
                        # one directory per domain: model, repository, service
```

**Adding a new domain:** Create a new directory (e.g. `order/`) with `mod.rs`, `model.rs`,
`repository.rs`, and `service.rs`. Add re-exports in `mod.rs` and register the module in
`lib.rs`. Wire the new service into `CoreServices`.

**Import conventions:** Use flat re-exports from domain modules, not submodule paths:

- `use crate::user::{User, UserService, UserId}` (not `user::model::User`)
- `use crate::repository::{Repository, Transaction}` for shared infrastructure
- `use crate::types::Capabilities` for shared types

### Subsystem Pattern (tokio-graceful-shutdown)

Each crate that owns background work exposes a `XxxSubsystem` struct + `create_xxx_subsystem()` factory
in its `lib.rs` — see `ApiSubsystem` in `ib-api`, whose `run()` starts its child subsystems
(`GrpcSubsystem`, `McpSubsystem`) via `subsys.start(SubsystemBuilder::new(...))` then awaits
`on_shutdown_requested()`. `crates/issueboss/src/commands/server.rs` stays clean: call the factories,
pass results to `Toplevel`. Existing subsystems: `ApiSubsystem` (ib-api), `FrontendSubsystem` (ib-frontend).

## Frontend

The frontend (`ib-frontend`) is currently an axum stub page; Dioxus is the planned UI framework.
See @.claude/Dioxus.md for Dioxus 0.7 guidance.

## Database

The project uses SeaORM with Postgres, MySQL, and SQLite support. See @.claude/Database.md
for environment variable setup and SeaORM adapter patterns.

## Workflows

**Multi-step implementations:** Make each logical step its own jj changeset. Before
starting a step, ensure the working copy is empty (`jj new` if needed). At the end of each
step, run the end-of-task routine below.

**After completing each task (end-of-task routine):**

Run these as three separate Bash commands, not one `&&` chain, so each step's result is visible on its own.

1. `just fmt-lint` — format and lint code
2. `just component-tests` — verify tests pass
3. `jj desc -m "type(scope): description\n\nbody"` — update working copy description

### Workspaces

When creating a new workspace, run

```bash
direnv allow
mise trust
# just tailwindcss
```

To verify the baseline state, run `just component-tests`.

## Testing

- Tests live alongside source code in `#[cfg(test)]` modules
- Colima manages docker containers required for integration testing

## Conventions

- **Commits:** Valid scopes: `api`, `core`, `database`, `frontend`, `insights` (match crate names)
- **Error handling:** `thiserror` for `core`, `api`, `database`; `anyhow` for the `issueboss` and `insights` binaries

## Insights

This project uses `.insights/` for research, triage docs, specs, plans, and personal notes
managed by the `insights` CLI.

**At the start of brainstorming, spec writing, or planning work**, dispatch the
`insights-locator` agent to check for prior context before proceeding. Use
`insights-analyzer` to read the most relevant documents. The `insights-research`
skill orchestrates both and presents the findings inline.

Directory layout:

- `.insights/issues/` — triage documents (IB-XX-triage-\*.md)
- `.insights/shared/specs/` — specs (IB-XX-spec-\*.md)
- `.insights/shared/plans/` — plans (IB-XX-plan-\*.md)
- `.insights/shared/research/` — research documents
- `.insights/scotte/` — personal notes
- `.insights/searchable/` — hardlink mirror for grep/search (read-only; strip "searchable/"
  from any path before reporting or editing)
  All `.insights/` artifact files must include YAML front-matter.
  See `.insights/shared/schema.md` for the full schema and vocabulary.
