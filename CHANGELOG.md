# IssueBoss - Take Control Of Your Project Issues

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1](https://github.com/szinn/IssueBoss/compare/v0.3.0..v0.3.1) - 2026-05-18

### Bug Fixes

- _(core)_ Coerce stringified JSON artifact bodies in add/update - ([a7decbc](https://github.com/szinn/IssueBoss/commit/a7decbc8103071a2b6a6047ccd4de71d07c6ea3d))

### Refactor

- _(crates)_ Rename paths to crates to remove ib- prefix - ([bb19dbb](https://github.com/szinn/IssueBoss/commit/bb19dbb27574d970dcfccfe9c68b5c546a40dc4d))
- _(tests)_ Refactored main to have setup function - ([b171912](https://github.com/szinn/IssueBoss/commit/b171912b5cadbeab9b1519f4f8d6b8b3d48f199e))

## [0.3.0](https://github.com/szinn/IssueBoss/compare/v0.2.11..v0.3.0) - 2026-04-14

### Features

- _(skills)_ Add instructions for implementing an issue - ([38bdf3f](https://github.com/szinn/IssueBoss/commit/38bdf3f69cecdab0b060919f93d2abce2bb5e7c0))

### Refactor

- _(cli)_ Extract output formatting into display module - ([c96b7fa](https://github.com/szinn/IssueBoss/commit/c96b7fa14ea8afe5662c686dfce1080b24bcbf41))
- _(core)_ Extract encode_into helper in token.rs - ([5e317e5](https://github.com/szinn/IssueBoss/commit/5e317e5f934d2aedd0ecdee4c7575db544b4b5a9))
- _(database)_ Ib-database refactoring audit and test coverage (IB-31) - ([e80958c](https://github.com/szinn/IssueBoss/commit/e80958c085f15b744e4572798cc914262b1ec166))

### Documentation

- _(readme)_ Update to include environment variables and bootstrap - ([5e6679e](https://github.com/szinn/IssueBoss/commit/5e6679e6b2b5d1246b16fba9c26c2396d126e5ef))

### Testing

- _(api)_ Consolidate test helpers and add handler coverage - ([485f1d4](https://github.com/szinn/IssueBoss/commit/485f1d488fd74dc1f832f74728c1e7800863f9eb))
- _(core)_ Add issue service check_transition_gate tests - ([f1dc5de](https://github.com/szinn/IssueBoss/commit/f1dc5dec9b3d481d66dda1912c310951a3f73c5b))
- _(frontend)_ Add unknown_route_returns_404 test - ([d3b6af5](https://github.com/szinn/IssueBoss/commit/d3b6af5114d99d71d74c6693eb3cf52aa8b8a48b))
- _(integration-tests)_ GRPC transport foundation - ([7900ed2](https://github.com/szinn/IssueBoss/commit/7900ed226171e7e237adbe78f5a924b988ffac8c))
- _(integration-tests)_ Fix cross-database parity - ([6efbcef](https://github.com/szinn/IssueBoss/commit/6efbcef4f212d26286d685a7439e77c940f5dc94))
- _(integration-tests)_ Spec/plan gate pipeline tests - ([d2c17b3](https://github.com/szinn/IssueBoss/commit/d2c17b3d7b45fc6696f60b71eeff928cb60043e1))
- _(integration-tests)_ Fill issue/project/user service-layer gaps - ([7746a43](https://github.com/szinn/IssueBoss/commit/7746a43051d17c8acf8b698673127a30d6caa66a))

### Miscellaneous Tasks

- _(cli)_ Refactor init.rs into composable named steps - ([38f847a](https://github.com/szinn/IssueBoss/commit/38f847ab26338049dbff56cf170e07e2486a4637))
- _(cli)_ Extract shared test fixture to test_support.rs - ([0d18115](https://github.com/szinn/IssueBoss/commit/0d181154816015355d12e7601240063a25cd72f9))
- _(cli)_ Extract git operation helpers to git.rs - ([b3b5fe1](https://github.com/szinn/IssueBoss/commit/b3b5fe1b2d0bfaa2ede53274a684dde0e1de80c9))
- _(core)_ Remove unused workspace dependencies - ([de646f8](https://github.com/szinn/IssueBoss/commit/de646f85d81e7e30b430f6ac3210b2749e3bb1fe))
- _(release)_ Strip binaries - ([77ee634](https://github.com/szinn/IssueBoss/commit/77ee634013076256defefddc6c60be1ebea972b1))

## [0.2.8] - 2026-04-12

### Features

- _(api)_ Resolve list_artifacts created_by to user object - ([6b12e31](https://github.com/szinn/IssueBoss/commit/6b12e316588a460aef15ce1ca672f13274242295))
- _(api)_ Add submitter/assigned to gRPC IssueResponse - ([8502489](https://github.com/szinn/IssueBoss/commit/85024894b71f639525ed7eb79afc89fa0fdec4ea))
- _(api)_ Add submitter/assigned fields to MCP server layer - ([16a8aac](https://github.com/szinn/IssueBoss/commit/16a8aac3fa171e1e4077e29f7568033e055870b8))
- _(api)_ Wire exclude_blocked filter through MCP and gRPC list_issues - ([bda6c1f](https://github.com/szinn/IssueBoss/commit/bda6c1f9bb7376a0d087ac1cb877d62b6c27eb89))
- _(api)_ Add pub mod api client functions to relationship.rs - ([00b3bc1](https://github.com/szinn/IssueBoss/commit/00b3bc111174b35606f18bdb69e3eb233d837dbe))
- _(api)_ Add add_relationship, remove_relationship, list_relationships MCP tools - ([59b8564](https://github.com/szinn/IssueBoss/commit/59b8564950eddfa14f5d4ff4d7faa1693bcffdbb))
- _(api)_ Artifact gRPC handler and proto messages - ([a145e9c](https://github.com/szinn/IssueBoss/commit/a145e9cc5f01582991f5afb19c307136db4f3f89))
- _(api)_ GRPC per-project capability checks on issue handlers - ([20758c7](https://github.com/szinn/IssueBoss/commit/20758c7962e278e64930a1438aa9d4b0410d5b36))
- _(api)_ MCP move_artifact admin check + issues resource auth - ([292a1d4](https://github.com/szinn/IssueBoss/commit/292a1d4203eeff0553394d6f2aab1b243ce1c2d3))
- _(api)_ MCP capability checks on issue-slug handlers - ([b18879e](https://github.com/szinn/IssueBoss/commit/b18879e674859a1afb92d781fb8d54ddd778b113))
- _(api)_ MCP require_capability helper + project-slug handler auth - ([4642df1](https://github.com/szinn/IssueBoss/commit/4642df1178ce654ae71e72fb346cb8d6c96dbd84))
- _(api)_ Add Handoff artifact kind to MCP description and integration test - ([c34c9f4](https://github.com/szinn/IssueBoss/commit/c34c9f4e7f04488e1622542ca7b224697f95f2ab))
- _(api)_ Update API layer for new IssueStatus state machine - ([cf43ea9](https://github.com/szinn/IssueBoss/commit/cf43ea9a9f355bbbf99b5ce5b7e120e289aead13))
- _(api)_ Add move_artifact MCP tool - ([a28f5d2](https://github.com/szinn/IssueBoss/commit/a28f5d2a2c38a5e47942f83d83eb61c2509f3eed))
- _(api)_ Add move_artifact MCP tool - ([a943988](https://github.com/szinn/IssueBoss/commit/a943988dadbdc341d35830da5e9cba17733ae061))
- _(api)_ Add slug to ArtifactMcp output, remove token - ([48b2803](https://github.com/szinn/IssueBoss/commit/48b2803836111e844c1bc5de064456d32103dfc6))
- _(api)_ Delete legacy MCP handler and finalize cleanup - ([d4db692](https://github.com/szinn/IssueBoss/commit/d4db6921b1406ae4ea4167412e424e5de764f053))
- _(api)_ Replace MCP REST router with rmcp StreamableHttpService - ([cc48202](https://github.com/szinn/IssueBoss/commit/cc48202484f9c9e2bad56a5d6a0926660dd7e7b5))
- _(api)_ Implement IssueBossServer MCP handler with tools and resources - ([248f090](https://github.com/szinn/IssueBoss/commit/248f090ffd7d83a1685f68c18efad2169a5c4cff))
- _(api)_ Add rmcp macros and streamable-http features - ([920fb26](https://github.com/szinn/IssueBoss/commit/920fb260737a67c44d244250355e889eda9754ce))
- _(api)_ Add gRPC TransitionIssue; remove status field from UpdateIssue - ([0d0e5f6](https://github.com/szinn/IssueBoss/commit/0d0e5f63cb68749f0a160a2d2a8d898221bdc4bd))
- _(api)_ Add MCP issue handlers - ([0abe4a2](https://github.com/szinn/IssueBoss/commit/0abe4a220e6f758abc4cdf4da862acbac09c96cd))
- _(api)_ Add gRPC issue handlers and proto messages - ([9a73e1d](https://github.com/szinn/IssueBoss/commit/9a73e1d31a2fe16e00e9faba18a5b1df70e23b03))
- _(api)_ Expose description in CreateProjectRequest and CLI - ([8463772](https://github.com/szinn/IssueBoss/commit/84637725dd2420b056d82d1c0ff5be4a18982faf))
- _(api)_ Implement real MCP list_projects using membership filter - ([919e34a](https://github.com/szinn/IssueBoss/commit/919e34a9c19b55bb868acb0966b3ec7603b12531))
- _(api)_ Add project/member gRPC handlers and proto messages - ([ac9e35f](https://github.com/szinn/IssueBoss/commit/ac9e35f11c74543bd24968dfd426c4c71fddd1c9))
- _(api)_ Add API key auth middleware for MCP and gRPC ports - ([f97c4d3](https://github.com/szinn/IssueBoss/commit/f97c4d33e8bf2a7d53abe668607cfbb241390860))
- _(api)_ Implement User CRUD gRPC handlers and client API functions - ([386fcf0](https://github.com/szinn/IssueBoss/commit/386fcf0976f35f594383cf4a2f09280b4a113554))
- _(api)_ Wire CoreServices into GrpcAdminService - ([d261cbc](https://github.com/szinn/IssueBoss/commit/d261cbc223e74c1b95c10b7727daf46e49db738a))
- _(api)_ Expand admin.proto — rename Seed→SuperAdmin, add User CRUD stubs - ([9c42a9f](https://github.com/szinn/IssueBoss/commit/9c42a9ff5b2abad0e932d7465ee4eb72f5fd7a52))
- _(api)_ Stub gRPC AdminService (Seed → AlreadyExists) and MCP list_projects (dummy data) - ([15de869](https://github.com/szinn/IssueBoss/commit/15de8697a6afbc7a46feb0316d3a1ece0bd4213c))
- _(api,cli)_ Implement super-admin gRPC handler and CLI command - ([729696b](https://github.com/szinn/IssueBoss/commit/729696b9ff2b1bc90ad0524cfe4e28ffb64e5beb))
- _(cli)_ Add relationship subcommand (add, remove, list) - ([8ad764a](https://github.com/szinn/IssueBoss/commit/8ad764a1942454a6389b5acb3ffb1c6f6747d690))
- _(cli)_ Artifact subcommand (add, update, remove, list, move) - ([5d732d3](https://github.com/szinn/IssueBoss/commit/5d732d3c76fe9d27fefd6914a480937f992404ba))
- _(cli)_ Add issue transition subcommand - ([63511a1](https://github.com/szinn/IssueBoss/commit/63511a12989786c21f2e609f9bff4567e1f6a7cd))
- _(cli)_ Add issue subcommands (create, list, get, update) - ([0d8b2f6](https://github.com/szinn/IssueBoss/commit/0d8b2f618f7d271ed0826002a20124ebe44468fd))
- _(cli)_ Rename --slug to --project in project subcommands - ([0ce42e5](https://github.com/szinn/IssueBoss/commit/0ce42e514c5212eed82565c72549f551bd4f9bc4))
- _(cli)_ Add project subcommands - ([93ddeac](https://github.com/szinn/IssueBoss/commit/93ddeac5225ddc00a2b0377e7ab44e5a1afb7b4c))
- _(cli)_ Add user subcommand group (create, list, get, update, delete, rotate-api-key) - ([cad1434](https://github.com/szinn/IssueBoss/commit/cad1434deaef67d04cfafb3af5d7d997faf693cf))
- _(cli)_ Add global --host / --port args with env var fallback - ([ab3fd32](https://github.com/szinn/IssueBoss/commit/ab3fd32d12ada694aab6acd7edd84ccb12dcab50))
- _(core)_ Record authenticated user as created_by on StatusTransition artifacts - ([4e43c2c](https://github.com/szinn/IssueBoss/commit/4e43c2ce64614c6c55bdc20158109cd01baadc0d))
- _(core)_ Thread triggered_by through transition_issue - ([8f314f0](https://github.com/szinn/IssueBoss/commit/8f314f01852c27943d7506604445c66955577cc6))
- _(core)_ Add exclude_blocked filter to IssueFilter and list adapter - ([144ab90](https://github.com/szinn/IssueBoss/commit/144ab9022b7fab55c7455011dc7afba35c2be1c3))
- _(core)_ Add exclude_blocked filter to IssueFilter and list adapter - ([50e249a](https://github.com/szinn/IssueBoss/commit/50e249a7545666fac18b61f13e57e84d0a8ed342))
- _(core)_ Implement IssueRelationshipService with BFS cycle detection - ([6717ba0](https://github.com/szinn/IssueBoss/commit/6717ba0b2a21f2542e7895668e344272743370e3))
- _(core)_ Slug auto-assignment, validation, and slug-based artifact addressing - ([cb5e7f2](https://github.com/szinn/IssueBoss/commit/cb5e7f23643c4410c8d73883878533190daa9572))
- _(core)_ Admin/SuperAdmin implicitly grants all project capabilities - ([848b0ab](https://github.com/szinn/IssueBoss/commit/848b0abb55e568f59e25a63dd5995fbbd789b6c6))
- _(core)_ Add capabilities_for_user merging user and project-member capabilities - ([6209e9e](https://github.com/szinn/IssueBoss/commit/6209e9eb132cfc52c134af9c7a71909000334575))
- _(core)_ Allow Triage → ReadyForPlan transition - ([3571e2a](https://github.com/szinn/IssueBoss/commit/3571e2ad132aec8c55d72d69956029c7555fc817))
- _(core)_ Implement IssueStatus::can_transition_to() with pipeline rules - ([6692262](https://github.com/szinn/IssueBoss/commit/66922625fc797323e98c21e3681adf95a028cc6c))
- _(core)_ Add IssueService, IssueRepository, and CoreServices wiring - ([483e74d](https://github.com/szinn/IssueBoss/commit/483e74db65d04e8fca2d723f03d264da4ab2af64))
- _(core)_ Add Issue domain model, enums, and IssueRepository trait - ([4b2aab5](https://github.com/szinn/IssueBoss/commit/4b2aab501db1235ae00d74795a1e05e0d15318ae))
- _(core)_ Add ProjectService with full CRUD and membership operations - ([f4dced9](https://github.com/szinn/IssueBoss/commit/f4dced9bda0f66336b3130e74b969541134d4578))
- _(core)_ Add Project domain models and repository traits - ([f10416e](https://github.com/szinn/IssueBoss/commit/f10416e02af6ed888b37acbe48717d28c122b1bc))
- _(core)_ Add slugify utility to ib-utils - ([57cccbe](https://github.com/szinn/IssueBoss/commit/57cccbef8f34882fcd774d5f580c03338e5ed9cd))
- _(core)_ Add api_key domain with model, repository, and service - ([98d891f](https://github.com/szinn/IssueBoss/commit/98d891f33284b8d4ce522ed5a764dd4d7af40e73))
- _(core)_ Add API key utility and extend UserService with list, rotate, any_super_admin - ([c7d1a52](https://github.com/szinn/IssueBoss/commit/c7d1a5249abdbfe8869b24acb4c29fa04d77d6c5))
- _(core)_ Add change_password_on_login to User model and migration - ([1bb85e2](https://github.com/szinn/IssueBoss/commit/1bb85e25af86dbfdf115d05ec8565b437cf3eb9b))
- _(core)_ Add UserService with transaction helpers and mock infrastructure - ([cbcca1a](https://github.com/szinn/IssueBoss/commit/cbcca1ac834718db02cc1e5ae848a0ed47531556))
- _(core)_ Add User domain model, Capability, UserRepository trait, and error types - ([6e9f27f](https://github.com/szinn/IssueBoss/commit/6e9f27f76cb770512b4d809d53a80684eb3d2782))
- _(core)_ Add Token<P,Id,MAX> typed ID type and define_token_prefix! macro - ([fb98162](https://github.com/szinn/IssueBoss/commit/fb98162164d8c3c0f520c03acc92becc49c4aac5))
- _(core,api)_ Artifact lifecycle — data layer, pipeline gates, MCP tools - ([48e5e93](https://github.com/szinn/IssueBoss/commit/48e5e9332f2240cddc87c7cb69247a5f116f1441))
- _(core,database)_ Add slug field to artifact data model - ([ca3668e](https://github.com/szinn/IssueBoss/commit/ca3668e61d3bbd1620d2645f96cb11fd2c4f89e6))
- _(database)_ Migration + entity/adapter for submitter/assigned fields - ([d6bc666](https://github.com/szinn/IssueBoss/commit/d6bc666e20a4e5d164db164797f06b828bd4131b))
- _(database)_ Add submitter_id / assigned_id migration - ([0d9f22e](https://github.com/szinn/IssueBoss/commit/0d9f22e70538f6a7452610c5ed31ea8047449915))
- _(database)_ Add issue_relationships migration, entity, and adapter - ([922569e](https://github.com/szinn/IssueBoss/commit/922569ea4f00ce82f92235cde9025f3038dde075))
- _(database)_ Add slug column to issue_artifacts - ([ed28283](https://github.com/szinn/IssueBoss/commit/ed28283e353f93844847abdb5f32e6a34eccd73a))
- _(database)_ Implement IssueRepositoryAdapter and increment_issue_counter - ([e6c3cd7](https://github.com/szinn/IssueBoss/commit/e6c3cd7fc36b829c5fb919341168a8baa76a0309))
- _(database)_ Add issues migration and SeaORM entity - ([d536527](https://github.com/szinn/IssueBoss/commit/d536527c538cdae98d8a87f7427e95f60ca3f687))
- _(database)_ Add optimistic locking guards to project update and delete - ([2378316](https://github.com/szinn/IssueBoss/commit/2378316a39fb21efac727e4b1368ace9f9725b3e))
- _(database)_ Add version and description to Project model and entity - ([0c686b0](https://github.com/szinn/IssueBoss/commit/0c686b0d85d62107fb87a458a5ac66986566a7a5))
- _(database)_ Add project and project_member entities and adapters - ([5667f4b](https://github.com/szinn/IssueBoss/commit/5667f4b214534f847a992f64a56ad893efd12aa2))
- _(database)_ Add projects and project_members migrations - ([9d5accf](https://github.com/szinn/IssueBoss/commit/9d5accfec4c99f1de7ed459a851ec5932065abd6))
- _(database)_ Add users migration, SeaORM entity, and UserRepository adapter - ([4cf316f](https://github.com/szinn/IssueBoss/commit/4cf316f4f3e00a1bb3b44c1afda6f11c5438f493))
- _(frontend)_ Start frontend subsystem - ([d2d090d](https://github.com/szinn/IssueBoss/commit/d2d090d432822d45074f3094b1a29171634dad5d))
- _(frontend)_ Add stay tuned placeholder page on GET / - ([57d520f](https://github.com/szinn/IssueBoss/commit/57d520f96485f9f46e54feec846efb033a295f63))
- _(insights)_ Print confirmation message after sync, commit, and init - ([3750ddc](https://github.com/szinn/IssueBoss/commit/3750ddc2c1a00e9b01fcbde1b0b401765e6f2ee2))
- _(insights)_ Triage agent discovers and links related issues - ([42a4af0](https://github.com/szinn/IssueBoss/commit/42a4af0dabcde6c2455ce11a9192268b1e8ae8be))
- _(insights)_ Add front-matter guidance to triage agent and issueboss skill - ([d4bf8d5](https://github.com/szinn/IssueBoss/commit/d4bf8d5cb14a478a1bd9b6496a8c36b845c67b44))
- _(insights)_ Write schema.md on init and add front-matter hint to CLAUDE.md - ([281abd3](https://github.com/szinn/IssueBoss/commit/281abd384f373a1a43f3fe0ad3e705f340126964))
- _(insights)_ Guard commit against empty changesets - ([3a231cf](https://github.com/szinn/IssueBoss/commit/3a231cf1628acb3576562480eac33e642c2911f8))
- _(insights)_ Add status command - ([14c6600](https://github.com/szinn/IssueBoss/commit/14c66006c4d7352987a3f8ba983733f91dd93c64))
- _(insights)_ Add git_is_dirty helper to core/git - ([2020f56](https://github.com/szinn/IssueBoss/commit/2020f56b6888d1c3454ac8ee3da0729fed64b41c))
- _(insights)_ Wire commands to core, complete v1 implementation - ([affcc3b](https://github.com/szinn/IssueBoss/commit/affcc3b0e60fca80c4be8cc9dc92a538d3092484))
- _(insights)_ Add core::commit orchestration with tests - ([980c7bb](https://github.com/szinn/IssueBoss/commit/980c7bb7ba70b2425c863284d7851e0b59de3621))
- _(insights)_ Add core::init full setup orchestration with tests - ([18399ba](https://github.com/szinn/IssueBoss/commit/18399bab155c54080d460a45cbfa1cbe24355936))
- _(insights)_ Add core::sync orchestration with tests - ([1db98c2](https://github.com/szinn/IssueBoss/commit/1db98c2e639fcac3c853b415d7c593d665a1e49f))
- _(insights)_ Add core::sync orchestration with tests - ([d82ca2e](https://github.com/szinn/IssueBoss/commit/d82ca2e4777650ea573c569b937f2827f1832e1b))
- _(insights)_ Add core::searchable hard-link mirror tree with tests - ([2e941d9](https://github.com/szinn/IssueBoss/commit/2e941d9d1b5668c656d1519b85f5a29d8a236b52))
- _(insights)_ Add core::searchable hard-link mirror tree with tests - ([2cf7317](https://github.com/szinn/IssueBoss/commit/2cf7317e32e59e26f0fc7cd505162bba4d190333))
- _(insights)_ Add core::gitignore with ensure_gitignore_entry and tests - ([20b3aec](https://github.com/szinn/IssueBoss/commit/20b3aec629c13f4ffaf144a5e6720dd71342654c))
- _(insights)_ Add core::gitignore with ensure_gitignore_entry and tests - ([16f8202](https://github.com/szinn/IssueBoss/commit/16f8202a1694e15d46b8cf0b2fdb6c7bd928f4dd))
- _(insights)_ Add core::symlinks with ensure/remove and tests - ([2a48fbd](https://github.com/szinn/IssueBoss/commit/2a48fbdeb1c237c7c519d9696b1b4af868ad21a7))
- _(insights)_ Add core::symlinks with ensure/remove and tests - ([925ecaa](https://github.com/szinn/IssueBoss/commit/925ecaa0bd0864fb56670ca2cf59436485fa1f15))
- _(insights)_ Add core::git shell-out helper with tests - ([730ca9f](https://github.com/szinn/IssueBoss/commit/730ca9f7b87585c49cad37d1b70194eaa648e080))
- _(insights)_ Add core::git shell-out helper with tests - ([061da45](https://github.com/szinn/IssueBoss/commit/061da45031b7d801b33ee29cec6308d6f482a384))
- _(insights)_ Add tracing initialisation, verbose mode - ([136c1dc](https://github.com/szinn/IssueBoss/commit/136c1dc4e9ab2a2e8a30de744c5db5e95bd55b0a))
- _(insights)_ Add CLI skeleton with --verbose flag and subcommand stubs - ([13dccc7](https://github.com/szinn/IssueBoss/commit/13dccc786a47598955daecb3f664cfc835006079))
- _(insights)_ Add Config model with load/write and tests - ([1bf8f94](https://github.com/szinn/IssueBoss/commit/1bf8f9419cd79ddf8f86a8b72d787d3b30c65785))
- _(insights)_ Scaffold insights crate in workspace - ([1b771c1](https://github.com/szinn/IssueBoss/commit/1b771c16924b282fa76d7a8b8cad2bc4afa54bdb))
- _(insights)_ Hooking up insights - ([4f0a701](https://github.com/szinn/IssueBoss/commit/4f0a701bf08a7f23c7d631e345816190fd3a341f))
- _(issueboss)_ Wire three-port binary with graceful shutdown - ([0021f8d](https://github.com/szinn/IssueBoss/commit/0021f8d31106a891b92f90bdb1fff4a15710f211))
- _(skill)_ Enforce DevReview transition across skill handoffs - ([c5db58d](https://github.com/szinn/IssueBoss/commit/c5db58d7af71d45d6e87605f5ae9cf9c3c83e4af))
- _(skill)_ Extract triage into self-contained background agent (IB-19) - ([f63203c](https://github.com/szinn/IssueBoss/commit/f63203cd06c7719dc64c3b8f3615a45b090c6ec6))
- _(skills)_ Add research dispatch guidance to issueboss skill - ([d58c785](https://github.com/szinn/IssueBoss/commit/d58c78568f14bc0880d1f3201572e19078bbdc39))
- Add Claude Code plugin structure - ([b2131c2](https://github.com/szinn/IssueBoss/commit/b2131c2031390f2ff955eb09145c11345ab005dc))

### Bug Fixes

- _(api)_ Apply buf lint fixes to admin proto - ([a0e1777](https://github.com/szinn/IssueBoss/commit/a0e1777414618953f6f8c17dca6c7b9476ba91b4))
- _(api)_ Expose artifact token in list_artifacts MCP response - ([8f465bf](https://github.com/szinn/IssueBoss/commit/8f465bf96539e54c63ebeb2a5850d056596454c1))
- _(api)_ Disable rmcp host check to restore MCP connections after 1.4.0 upgrade - ([0769056](https://github.com/szinn/IssueBoss/commit/07690562fa9d9b28f9e560ba93ad2b625982b08e))
- _(api)_ Standardize artifact MCP tool parameter slug → issue_slug - ([e345929](https://github.com/szinn/IssueBoss/commit/e345929c24802bcaa7031b0bcffc02567c542700))
- _(api)_ Update MCP router path params to axum v0.8 syntax - ([57ecc15](https://github.com/szinn/IssueBoss/commit/57ecc15c6ffc641616b2e53394d1b4b8ae18b55f))
- _(cli)_ Defer Config::load() to server command only - ([d149c6c](https://github.com/szinn/IssueBoss/commit/d149c6c5b97f0ffda076ac101e87f2c6cf2d3201))
- _(core)_ Throttle api_key last_used_at writes to once per 24 hours - ([cef44aa](https://github.com/szinn/IssueBoss/commit/cef44aa2eb1f7973620c3506e0db34147d6f0f5d))
- _(core)_ Allow *Review→Done and *Needed→Done transitions in DAG - ([8d63df9](https://github.com/szinn/IssueBoss/commit/8d63df9645a07806436bcd0adae589dc59356fc7))
- _(core)_ Rename IssueStatus::InDev to InProgress - ([8c5e344](https://github.com/szinn/IssueBoss/commit/8c5e3440d5615dd95895ba4d2748bac491e25fd0))
- _(core)_ Update delete_user_success test to mock api_key_repository - ([c5425ff](https://github.com/szinn/IssueBoss/commit/c5425ffc761a7593fe0b139523125bedddaa361d))
- _(core)_ Fix sha2 upgrade api change - ([46849a0](https://github.com/szinn/IssueBoss/commit/46849a0d9775c2b00dcddbe5c1a0baa222745f48))
- _(core)_ Add Copy + serde rename_all to Capability, use UserId alias in UserRepository - ([d539a1d](https://github.com/szinn/IssueBoss/commit/d539a1d437845b90dac1737b988c63ba9115d375))
- _(database)_ Add version conflict guard to user update - ([7a61cd0](https://github.com/szinn/IssueBoss/commit/7a61cd096e134da61f38f95001542354e1c7ead0))
- _(database)_ Use expect for capabilities serialization, log deserialization errors, add unique annotation - ([0dc5c60](https://github.com/szinn/IssueBoss/commit/0dc5c604666b9a4e90f625bb3e1e995ad158aef4))
- _(ib-utils)_ Add try_from_id returning Result, fix FromStr double-check in Token - ([4500472](https://github.com/szinn/IssueBoss/commit/4500472f9b288c9488fa996cc8a84a92ae7bfe23))
- _(insights)_ Check .claude/CLAUDE.md before root when writing insights section - ([d93a623](https://github.com/szinn/IssueBoss/commit/d93a623b6353982a7924a9ba750958d646c5c4b2))
- _(insights)_ Rewrite insights_snippet to use raw string, eliminating rust-analyzer false positives - ([5b37424](https://github.com/szinn/IssueBoss/commit/5b374246996372b0ee89d9b6a3b97c19379e0891))
- _(insights)_ Simplify zero-results step-skip phrasing in insights-research - ([5c12dda](https://github.com/szinn/IssueBoss/commit/5c12dda77b2130be3863bd2d8111bc5f8bb329a1))
- _(insights)_ Fix omit-empty example and clarify ordering in insights-analyzer - ([bd200f4](https://github.com/szinn/IssueBoss/commit/bd200f4f36c15f5db405205d4d2dcb06fd630849))
- _(insights)_ Clarify dedup and personal notes in insights-locator - ([af28614](https://github.com/szinn/IssueBoss/commit/af28614aef60ff9ec37c1e71b2809b2a8ddbe53f))

### Refactor

- _(api)_ Moved grpc commands into modules - ([978a261](https://github.com/szinn/IssueBoss/commit/978a261b727186f8e8658dd12aeaf4e535c607a3))
- _(api,bin)_ Refactoring to follow BookBoss - ([a461fc2](https://github.com/szinn/IssueBoss/commit/a461fc2eeb09023838aa087a632ec07a12575f14))
- _(api,cli)_ Identify projects by slug instead of token - ([70de4b6](https://github.com/szinn/IssueBoss/commit/70de4b6368826877fa3164bb21782a4bee5479ea))
- _(api,core,database)_ Minor refactoring - ([ae0726a](https://github.com/szinn/IssueBoss/commit/ae0726a67dc18ea3e21a399cfa33fe9784d9db1b))
- _(cli)_ Extract api-key subcommand from user - ([3ec0f0f](https://github.com/szinn/IssueBoss/commit/3ec0f0f0b98f6d76027e890b853ff3a97cbd8348))
- _(core)_ Move Capability and Capabilities to types domain - ([e84af55](https://github.com/szinn/IssueBoss/commit/e84af55eeff94d233e319e57c8bb5adeda2c5069))
- _(core)_ Rename IssueStatus::InProgress to InDev - ([8b3dc2d](https://github.com/szinn/IssueBoss/commit/8b3dc2d4113104e69f39d8d5c1492fc666821012))
- _(core)_ Redesign issue slug as stable user-facing reference - ([0c6b7ad](https://github.com/szinn/IssueBoss/commit/0c6b7ad3f6aca1e820f84f2ae62dbb6e24c7beec))
- _(core,database,api,cli)_ Extract api_key domain and remove token field - ([2d57a17](https://github.com/szinn/IssueBoss/commit/2d57a17fd4a3d02005dba6b1dfa3f51e700dd6f8))
- _(database)_ Collapse issue table migration - ([ede9634](https://github.com/szinn/IssueBoss/commit/ede96348ffc08d4422a17d9665fadbcc5860654b))
- _(database)_ Fixed from() to follow pattern - ([b4464e6](https://github.com/szinn/IssueBoss/commit/b4464e6f3f5d5834435db0400bf8eca25463e284))
- Move subsystems into owning crates - ([5682b3b](https://github.com/szinn/IssueBoss/commit/5682b3b41c45b93b08760e3a6938965fa6c93410))

### Documentation

- _(skills)_ Compress issueboss skill for token efficiency - ([a6f13bf](https://github.com/szinn/IssueBoss/commit/a6f13bf57e174961279c84ad960d7247bfb56c17))
- Expand README and strengthen issueboss skill config requirement - ([d62fdf1](https://github.com/szinn/IssueBoss/commit/d62fdf1fa88c8bc098e0d310035592b65e1ea8ed))
- Write README with project highlights - ([058135f](https://github.com/szinn/IssueBoss/commit/058135fc0304d5bdb4c17b48229b2b937c33eb75))

### Testing

- _(core)_ Add integration tests for issue relationships - ([593e5c1](https://github.com/szinn/IssueBoss/commit/593e5c1b4ba8c9f646622656e238c90b9a34ae17))
- _(integration)_ Move_artifact end-to-end tests - ([a320e93](https://github.com/szinn/IssueBoss/commit/a320e93fcd9e7cfa401203ebef51c315f1cd8a3d))
- _(integration)_ Artifact slug lifecycle, update, remove, uniqueness - ([880ca0d](https://github.com/szinn/IssueBoss/commit/880ca0d2ecc743a8d8fea1ff92ab56135d3573ba))
- _(integration)_ Artifact lifecycle end-to-end tests - ([97b248f](https://github.com/szinn/IssueBoss/commit/97b248f2dbe22255b607751cb9f8ec92137efb2d))
- _(integration)_ Add cascade and sequence integration tests - ([3694519](https://github.com/szinn/IssueBoss/commit/369451989f20b5e85ced7c9ae60052bb7598fea2))

### Miscellaneous Tasks

- _(api)_ Suppress dead_code warning on tool_router field - ([74da96c](https://github.com/szinn/IssueBoss/commit/74da96c47d87ee75773d99923fe78ccd62e66f94))
- _(claude)_ Tell claude how to access IssueBoss - ([96f4446](https://github.com/szinn/IssueBoss/commit/96f444648691e735429cc77b1ec6d356e87d1f0a))
- _(claude)_ Tell claude how to access IssueBoss - ([cca03a7](https://github.com/szinn/IssueBoss/commit/cca03a79d19b74b2a13acbbb90521185868cbcbd))
- _(core)_ Remove TransitionStatus capability - ([73a7f03](https://github.com/szinn/IssueBoss/commit/73a7f033769af219a27d2f560bd78e25c467ad3a))
- _(core)_ Add rmcp and dioxus workspace deps, clean up stale bb-\* profile comments - ([748d3b5](https://github.com/szinn/IssueBoss/commit/748d3b5930452d7cd37bbd8b0e2e55d134983611))
- _(core)_ Scaffold workspace — all crates registered, workspace deps declared - ([ebf1029](https://github.com/szinn/IssueBoss/commit/ebf1029d205b0012f1e522c954847f6d89f5c1af))
- _(database)_ Don't need down migrations - ([c948017](https://github.com/szinn/IssueBoss/commit/c948017efa3a5d76ef0518c905f89591c6ae8125))
- _(database)_ Note when database is connected - ([7095995](https://github.com/szinn/IssueBoss/commit/7095995e58de7ff26da2318ce003ac6e6a3c35d2))
- _(database)_ Remove migration - ([0879fad](https://github.com/szinn/IssueBoss/commit/0879faded53488718a2a3204e346440d4289507b))
- _(logging)_ Adjust logging filters - ([5664fce](https://github.com/szinn/IssueBoss/commit/5664fce6ee7d9b3d6decb4d1bfcd4103ed7e15a5))
- _(migration)_ Absorb migration into original table - ([13e823d](https://github.com/szinn/IssueBoss/commit/13e823dc798692adecaa247448cba5abefce6dd0))
- _(skill)_ Stronger definition of what an open issue is - ([bd31bdd](https://github.com/szinn/IssueBoss/commit/bd31bdd76f9c70bb0260a0fdd1debb966c580570))
- _(skill)_ Refine issueboss triage skill - ([6f984f2](https://github.com/szinn/IssueBoss/commit/6f984f2edb4de593f1fd54183203e2013280c690))
- _(tests)_ Fix clippy warnings in integration tests - ([b3b7e18](https://github.com/szinn/IssueBoss/commit/b3b7e1829758205141e35b691c76dd2d409fbeb1))
