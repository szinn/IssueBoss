# Database

The project uses SeaORM with Postgres, MySQL, and SQLite support. For Postgres and MySQL,
an existing instance is required for database-related commands.

## Environment Variables

- `PGUSER`, `PGPASSWORD`, `PGDATABASE` — used by `just create-database` and `just database`
- `PGADMINUSER`, `PGADMINPASSWORD` — admin credentials for database creation
- `ISSUEBOSS__DATABASE_URL` — SeaORM connection string for migrations and entity generation
  - Postgres: `postgres://user:password@host:port/database`
  - MySQL: `mysql://user:password@host:port/database`
  - SQLite: `sqlite::path`

## SeaORM Adapter Patterns

**Migrations:** Only `up()` migrations need to be implemented. The `down()` method can just
be empty.

**Enum storage:** All domain enums stored as plain `String` columns (no DB CHECK constraints).
Conversion functions are module-private.
`From<Model> for DomainType` is infallible and panics on unknown values — acceptable since all
writes go through adapters.

**`ActiveModelBehavior` / `before_save`:** The `issues`, `projects`, and `users` entities have a `before_save` hook that
auto-increments `version` and sets `updated_at`. When inserting, use `version: Set(0)` — the
hook bumps it to 1. Don't fight it.

**Optimistic locking pattern:**

```rust
let existing = Entity::find_by_id(id).one(db_tx).await?.ok_or(NotFound)?;
if existing.version != record.version { return Err(VersionConflict); }
// set all mutable fields, then .update()
```

**Adding a new repository to `RepositoryService`:**

1. Add field + accessor to `core/src/repository.rs` `RepositoryService`
2. Create `database/src/adapters/<name>.rs` with adapter impl + tests
3. Register in `database/src/adapters/mod.rs`
4. Import + wire into builder in `database/src/lib.rs`
5. Add `Mock<Name>Repository` to `default_repository_service_builder()` in the `testing`
   module of `core/src/repository.rs`
