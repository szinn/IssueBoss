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
├── types.rs            # Shared newtypes (Email, Age) used across domains
├── repository.rs       # Shared infrastructure: Repository, Transaction traits,
│                       #   RepositoryService, and transaction macros
├── test_support.rs     # Mock implementations (behind "test-support" feature)
├── auth/               # Session auth: Session, AuthService, SessionRepository
└── user/               # Users and settings: User, UserService, UserSettingService
```

**Adding a new domain:** Create a new directory (e.g. `order/`) with `mod.rs`, `model.rs`,
`repository.rs`, and `service.rs`. Add re-exports in `mod.rs` and register the module in
`lib.rs`. Wire the new service into `CoreServices`.

**Import conventions:** Use flat re-exports from domain modules, not submodule paths:

- `use crate::user::{User, UserService, UserId}` (not `user::model::User`)
- `use crate::session::{Session, NewSession}` (not `session::model::Session`)
- `use crate::repository::{Repository, Transaction}` for shared infrastructure
- `use crate::types::{Email, Age}` for shared newtypes

### Subsystem Pattern (tokio-graceful-shutdown)

Each crate that owns background work exposes a `XxxSubsystem` struct + `create_xxx_subsystem()` factory
in its `lib.rs` — same pattern as `ApiSubsystem` in `bb-api`. The subsystem's `run()` starts its
child subsystems via `subsys.start(SubsystemBuilder::new(...))` then awaits `on_shutdown_requested()`.
`bookboss/main.rs` stays clean: call the factories,
pass results to `Toplevel`. Existing subsystems: `ApiSubsystem` (bb-api), `CoreSubsystem` (bb-core,
owns `JobWorker` and `BookdropScanner`).

## Frontend

The frontend is built using Dioxus. See @.claude/Dioxus.md for more info.

## Database

The project uses SeaORM with Postgres, MySQL, and SQLite support. See @.claude/Database.md
for environment variable setup and SeaORM adapter patterns.

## Workflows

**Multi-step implementations:** Each logical step **MUST** be its own jj changeset. Before
starting a step, ensure the working copy is empty (`jj new` if needed). At the end of each
step, run the end-of-task routine below.

**After completing each task (end-of-task routine):**

These **MUST** be run as separate Bash commands. Do **NOT** join them into a single one with `&&`.

1. `just fmt` — format code
2. `just clippy` — lint (run separately from fmt, not chained)
3. `just component-tests` — verify tests pass
4. `jj desc -m "type(scope): description\n\nbody"` — update working copy description
5. Update backlog if working on a task that came from the backlog.

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

- **Commits:** Valid scopes: `api`, `cli`, `core`, `database`, `frontend` (match crate names)
- **Error handling:** `thiserror` fo
- r `core`, `api`, `database`; `anyhow` for `bookboss` (binary)
