## Checklist Addition: Gate 90 - Validator Source Namespace Topology And Semantic Repo-Law Enforcement

This checklist section is a tracking surface only. It does not weaken Gate 90 and does not replace Gates 8, 24, 89, or 89.22. Do not check an item unless the physical source topology is repaired, the broad exception escape hatch is removed, typed enforcement exists, red/green/tamper fixtures pass, package inventory is current, coverage remains 100 percent, and the full source audit passes on the same candidate digest.

### Gate 90.1: Live Violation Verification

- [ ] Verify current top-level `validator/src/*.rs` file count before repair.
  - Evidence: Side-thread read-only inspection found 155 top-level Rust files.
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Verify current top-level `validator/src/internal_*.rs` count before repair.
  - Evidence: Side-thread read-only inspection found 105 top-level `internal_*.rs` files.
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Verify current top-level `validator/src/internal_coverage*.rs` count before repair.
  - Evidence: Side-thread read-only inspection found 16 top-level `internal_coverage*.rs` files.
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Verify typo variants such as `validator/src/iinternal_*.rs` are searched and rejected by enforcement even if none currently exist.
  - Evidence: Side-thread read-only inspection found 0 current `iinternal_*.rs` files, but Gate 90 requires red fixtures for the typo class.
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Verify `docs/namespace-law-exceptions.json` contains or no longer contains broad validator-source repeated-prefix exceptions.
  - Evidence: Side-thread read-only inspection found `repeated-prefix-validator-src-internal` at lines 2748-2758 with `directory = "validator/src"`, `prefix = "internal"`, and `applies_to = ["validator/src/internal*"]`.
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Verify `plugin-manifest-draft.json` contains or no longer contains stale top-level `validator/src/internal_*.rs` resource entries.
  - Evidence: Side-thread read-only inspection found top-level internal test resources listed at lines 3850-3954.
  - Command:
  - Candidate digest:
  - Status: validated current

### Gate 90.2: Physical Source Topology Repair

- [ ] Move validator self-tests and internal test surfaces out of flat `validator/src/internal_*.rs` names into semantically routed directories such as `validator/src/self_tests/coverage/`, `validator/src/self_tests/claim/`, `validator/src/self_tests/cli/`, `validator/src/self_tests/review/`, `validator/src/self_tests/schema/`, `validator/src/self_tests/target_repo/`, `validator/src/self_tests/package/`, `validator/src/self_tests/audit/`, `validator/src/self_tests/product/`, and `validator/src/self_tests/boundaries/`.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Remove `internal_` from final validator test filenames where the directory already communicates test/internal scope.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Remove coverage-wave history from final filenames when it records coverage-chase chronology instead of domain responsibility.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Rename moved files to semantic domain-behavior names such as `receipt_authority.rs`, `schema_dispatch.rs`, `target_fixture_boundaries.rs`, `semantic_receipt_boundaries.rs`, `claim/evidence_boundaries.rs`, or equivalent domain-specific names.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Split large test clusters by domain responsibility rather than chronological wave, coverage chase, or implementation history.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Update Rust module routing so tests remain discoverable by domain without reintroducing one opaque mega-router of flat historical names.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Preserve line caps after the topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Preserve exact 100 percent coverage after the topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

### Gate 90.3: Exception Model And Validator Enforcement

- [ ] Delete the broad `repeated-prefix-validator-src-internal` exception.
  - Evidence:
  - Command:
  - Candidate digest:
  - Status: validated current

- [ ] Forbid broad repo-owned source exceptions for `validator/src/internal*`, `validator/src/*_wave*`, `validator/src/*coverage*`, and typo variants such as `iinternal_*`.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Parse namespace exceptions into closed typed kinds: generated fixture/catalog exception, public distribution surface exception, external compatibility surface exception, and narrow source-layout exception.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Reject broad hand-authored source globs such as `validator/src/*`, `validator/src/internal*`, and equivalent source exception patterns.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace validation inspects actual repo-owned source files as well as package manifest resources.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Missing manifest entries cannot let source topology escape namespace law, and manifest entries cannot bless non-compliant source topology.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Repeated-prefix validation distinguishes generated fixture catalogs from hand-authored source.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace validation fails when a directory has more than two hand-authored files sharing a non-semantic prefix and no typed narrow exception.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace validation rejects implementation-history names such as `internal`, `wave`, `coverage_wave`, `tmp`, `old`, `misc`, `helpers`, `utils`, `common`, `shared`, `support`, `lib`, `services`, and typo variants when used as source authority.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace failures are agent-remediating and include directory, offending prefix, count, representative paths, missing-directory rationale, required repair class, affected claim classes, and typed-exception eligibility.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

### Gate 90.4: Semantic Repo-Law Enforcement

- [ ] File names, module names, test module names, schema file names, receipt names, fixture ids, check ids, and authority object names are treated as semantic authority surfaces.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Rust modules, nested modules, public/private functions, helper functions, test functions, type names, enum variants, constants, local authority identifiers, and artifact path segments are treated as semantic authority surfaces.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Generic source/module names that encode storage status or implementation history fail when used as authority surfaces.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Goal-work, evidence-purpose, phase/slice, session-history, and progress labels fail when used as source paths, module names, function names, helper names, test names, ids, or artifact path segments instead of product behavior.
  - Working examples that must fail unless a typed external compatibility boundary applies: `fitting`, `production_proof`, `gate92`, `phase4`, `slice`, `workstream`, `checkpoint`, `progress`, `todo`, `wip`, `scratch`, `helpers`, `utils`, `common`, `misc`, `shared`, and `support`.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Product vocabulary allowlist is contextual, not word-based.
  - Working means `fit-repo` is allowed as a user-facing product command, `contract` is allowed only for real product/runtime/data contracts, `closure` is allowed only for package/dependency closure behavior, and `proof` is allowed only for literal `prove` command surfaces or proof-artifact validators.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Compatibility aliases remain at parser/schema boundaries only and route into product-semantic implementation names.
  - Working means existing public commands or schema fields can remain temporarily only when the implementation below them uses behavior names such as command roundtrip, telemetry reconciliation, receipt dereference, span parentage, cache invalidation, source topology, or command inventory. New internal paths/modules/functions may not inherit compatibility jargon.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Semantic source modules name the law, domain, authority, boundary, receipt, fixture, product surface, or workflow they govern.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Domain directories carry the repeated concept and filenames inside those directories drop redundant prefixes.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Typed semantic-name exceptions are narrow, parsed, package-included, claim-limited, and independently red-fixtured.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace failures for symbols inside files are agent-remediating.
  - Working means failures include offending symbol, containing path, offending segment, why it encodes goal work/history/evidence purpose/generic bucket, suggested product-behavior naming class, affected claims, and typed-exception eligibility.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

### Gate 90.5: Red, Green, And Tamper Fixtures

- [ ] Red fixture: top-level `validator/src/internal_coverage_wave99_tests.rs` style file fails.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: top-level `validator/src/internal_claim_tests.rs`, `validator/src/internal_review_tests.rs`, and `validator/src/internal_schema_tests.rs` style clusters fail when more than two files share the prefix.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: typo variant such as `validator/src/iinternal_coverage_tests.rs` fails.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: broad source exception for `validator/src/internal*` fails even when it names `plugin-manifest-draft.json` as a contract.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: generated/catalog exception cannot be used for hand-authored validator source.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: package manifest listing cannot satisfy namespace compliance for a non-compliant source path.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: moved file that keeps coverage-wave history still fails semantic repo-law when the name remains non-semantic.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: source path or module named for goal work fails.
  - Required examples: `validator/src/cli/observe/fitting/mod.rs`, `validator/src/cli/observe/production_proof/mod.rs`, `validator/src/audit/gate92/mod.rs`, `validator/src/audit/phase4_rebind.rs`, and `validator/src/cli/progress/checkpoint.rs`.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: functions and helpers named for goal work fail even when the file path is otherwise semantic.
  - Required examples: `fit_command`, `fit_path`, `production_proof`, `phase4_rebind`, `checkpoint_progress`, `todo_repair`, and modules named only `helpers`, `utils`, `common`, `shared`, or `support`.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: observability artifact path segments named for goal work fail.
  - Required examples: `validation_artifacts/observability/fitting/...` and `validation_artifacts/observability/production-proof/...` unless a typed compatibility schema boundary exists and the product-semantic replacement path is present.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Red fixture: namespace errors cannot be hidden by capped reporting, row presence, foundational trace presence, source-obligation presence, coverage pass, line-cap pass, package inventory pass, Product Fitness pass, reviewer approval, or lowered claim ceiling.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Green fixture: semantically routed validator test directory passes with names such as `validator/src/self_tests/coverage/receipt_authority.rs`.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Green fixture: product-semantic observability paths and symbols pass.
  - Required examples: `validator/src/cli/observe/command_roundtrip/mod.rs`, `validator/src/cli/observe/telemetry_reconciliation/mod.rs`, `validator/src/audit/observability/command_inventory/mod.rs`, `run_command_roundtrip`, `query_roundtrip`, `reconcile_same_candidate`, and `write_command_inventory`.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Green fixture: product command vocabulary such as `fit_repo` passes when and only when it represents the user-facing fit-repo product surface.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Green fixture: generated fixture catalogs may use repeated prefixes only when a generator/catalog route owns the family and the exception is narrow, typed, and claim-limited.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Tamper fixture: widening a narrow namespace exception after receipt generation invalidates the receipt and blocks claims.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

- [ ] Tamper fixture: contextual allowlist cannot be reused to bless goal-work names.
  - Working means `fit-repo` can pass as product vocabulary while `fitting`, `fit_goal`, `fit_slice`, and `production_proof` fail as path/module/function/artifact namespaces.
  - Evidence:
  - Fixture:
  - Candidate digest:
  - Status: validated current

### Gate 90.6: Standards, Trace, Source-Obligation, Package, And Claim Integration

- [ ] Agent-standards rows explicitly cover validator source namespace topology and semantic repo-law enforcement, not only generic namespace or semantic-domain row presence.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Plugin-activated and retrofitted repo standards carry product-semantic naming law without requiring parent-session context.
  - Working means shipped standards say paths, modules, functions, helpers, tests, ids, receipt/artifact paths, and generated/package paths describe product behavior or domain responsibility, not goal work, evidence purpose, phase/slice labels, session history, or generic buckets.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Foundational trace maps filesystem-as-agent-interface and scoped-module article requirements to validator source topology enforcement, red fixtures, valid fixtures, receipts, package inventory, and claim ceilings.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Source-obligation parity represents this law as first-class or as a typed child law with independent failure proof.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Claim-ceiling guards block completion, review, package, readiness, release, product-readiness, CLI self-law, source audit, final packet, and update_goal eligibility while validator source topology violates namespace or semantic repo-law.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Feedback-to-rule and historical regression corpus rows record the side-thread `internal_*`/`internal_coverage_*` sprawl and broad exception loophole signal.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] `plugin-manifest-draft.json`, package inventory, component graph, review target, candidate archive, source/install/cache package surfaces, and any installed/cache sync references include moved files exactly once and contain no stale top-level internal paths.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

### Gate 90.7: Validation And Confidence

- [ ] `cargo fmt --check` passes after topology repair.
  - Evidence:
  - Candidate digest:
  - Status: validated current

- [ ] `cargo test --offline` passes after topology repair.
  - Evidence:
  - Candidate digest:
  - Status: validated current

- [ ] Namespace law proof passes through the CLI after topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Semantic domain-type naming proof passes through the CLI after topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Package inventory closure and exactly-once proof pass after topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Line-cap proof passes after topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Coverage proof remains exactly 100 percent with `uncovered_records = []` after topology repair.
  - Evidence:
  - Status: validated current

- [ ] Full source audit with red fixture report passes after topology repair.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Red report:
  - Status: implemented, pending validation

- [ ] Source/install/cache package evidence is regenerated after source passes, and only after source passes.
  - Evidence:
  - Command:
  - Source digest:
  - Installed receipt:
  - Cache receipt:
  - Status: not started

- [ ] Final packet includes calculated confidence for Gate 90 using the required 100-point model: 25 root cause observed directly, 20 source-law alignment, 20 direct enforcement path, 15 Rust/source topology refactor feasibility, 15 red/green/tamper proof completeness, and 5 residual integration risk.
  - Evidence:
  - Calculated confidence:
  - Candidate digest:
  - Status: not started

- [ ] Final packet explains why physical source topology repair plus typed exception tightening plus red/green/tamper enforcement was chosen over row-only, exception-only, validator-only, coverage-only, line-cap-only, or claim-ceiling-only alternatives.
  - Evidence:
  - Candidate digest:
  - Status: not started

## Checklist Addition: Gate 91 - Rust Developer Experience, Runtime Resource Discipline, And Workspace Garbage Collection

This checklist section is a tracking surface only. It does not weaken Gate 91 and does not replace Gates 5, 8, 9, 14, 21, 34, 46, 51, 76, 87, 89, 89.22, or 90. Do not check an item unless the implementation is represented in standards rows, source-obligation rows, foundational trace entries, schemas, validator checks, red/green/tamper fixtures, receipts, package inventory, source audit evidence, and claim-ceiling guards on the same candidate digest.

### Gate 91.1: Doctrine And Authority

- [ ] Rust developer experience is governed as a product surface, not treated as developer preference.
  - Evidence:
  - Standards row:
  - Source-obligation row:
  - Foundational trace:
  - Candidate digest:
  - Status:

- [ ] Raw tools are observations only; only `ultragoal` can convert observations into typed, digest-bound, same-surface receipts and claim ceilings.
  - Evidence:
  - CLI command:
  - Receipt:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Cargo is declared as canonical Rust substrate and explicitly not claim authority.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Raw `cargo`, `nextest`, `llvm-cov`, `deny`, `audit`, `sccache`, watcher, rust-analyzer, CI log, Makefile, shell script, and editor output cannot satisfy completion/review/package/release/product/update_goal claims.
  - Evidence:
  - Red fixtures:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.2: Required Rust Substrate

- [ ] `rust-toolchain.toml`, Cargo lock policy, Cargo workspace metadata, Cargo profiles, feature strategy, MSRV policy, and `.cargo/config.toml` policy are declared and receipt-bound.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Toolchain verification records active toolchain, `rustc --version --verbose`, `cargo --version --verbose`, installed components, `rust-toolchain.toml` digest, `Cargo.lock` digest, Cargo metadata digest, and environment allowlist.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] Required Cargo observation commands are CLI-routed: `cargo check`, `cargo build`, `cargo fmt`, `cargo clippy`, doctests/compatibility tests, `cargo metadata`, and dependency tree duplicate checks.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] `cargo nextest` is required for standard and release normal test execution after bootstrap, and absence emits a missing-tool/bootstrap failure rather than silent fallback.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] `cargo llvm-cov` is the canonical Rust coverage proof path for declared Rust scope, with exact 100 percent coverage, `uncovered_records = []`, candidate/source/toolchain binding, and anti-gaming checks.
  - Evidence: Status: validated current.
  - Status: validated current

- [ ] Required typed-boundary and diagnostics crates or same-law replacements are governed: `serde`, `schemars`, `jsonschema`, `serde_path_to_error`, `clap` typed enums, `camino`, `thiserror`, `miette`, and `tracing`.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] `anyhow` or catch-all errors are forbidden in law-core APIs and allowed only at binary/application edges with claim limits.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Parser/property/fuzz/compile-fail/snapshot/CLI/tempdir proof tools are governed where corresponding code exists: `proptest`, `cargo-fuzz`, `trybuild`, `insta`, `snapbox` or equivalent, `assert_cmd`, and `tempfile`.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.3: Acceleration, Cache, And Watch Governance

- [ ] `sccache`, Cargo incremental compilation, linker acceleration, Cargo target dirs, Cargo registry/git caches, nextest recordings, rust-analyzer target dirs, CI caches, plugin caches, install caches, and remote caches are declared in cache/no-cache receipts before supporting any timing or iteration claim.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Warm-cache proof cannot support cold/no-cache claims.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] `sccache` is default-on acceleration when available and declared; absence is an acceleration-unavailable event, not correctness failure; remote cache is forbidden unless declared with redacted endpoint identity, cache key, policy, and claim limit.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Linker acceleration is governed: `lld` default-on where platform policy permits, `mold` governed adapter, and linker acceleration supports timing/iteration claims only.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] `cargo-binstall` is default-on for bootstrap speed only under version/digest policy, with `cargo install --locked` fallback; install receipts support tool/bootstrap claims only.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] `watchexec`/Bacon watcher flow is governed so watchers emit feedback observations only, never completion receipts.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] `cargo-watch` as standard watcher, Makefile authority, unwrapped shell script authority, global aliases, editor-only proof, and hidden global Cargo config are rejected as claim paths.
  - Evidence:
  - CLI command:
  - Red fixtures:
  - Candidate digest:
  - Status:

### Gate 91.4: Required Rust Command Loops

- [ ] FAST loop `ultragoal rust fast` exists and proves only fast feedback / changed-source structural observations.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] STANDARD loop `ultragoal rust standard` exists and includes toolchain receipt, metadata receipt, workspace topology, namespace, line-cap, typed-boundary, fmt, clippy, check, nextest standard, doctest, required fixture triads, receipt verification, and claim ceiling computation.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] RELEASE loop `ultragoal rust release` exists and runs full law proof for the requested claim, including standard loop, exact coverage, dependency/security/supply-chain, feature matrix, performance, memory/resource, package inventory, clean install, cache separation, product proofs when required, GC dry-run, receipt verification, claim ceiling, and packet build when requested.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Product Success is required in release only when product success, daily-driver, user outcome, product readiness, external-user, marketplace, or release claim depends on product outcome; it cannot be substituted into or out of unrelated claims.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] COLD/CLEAN loop `ultragoal rust clean-proof` proves clean checkout and no hidden local cache dependency with isolated Cargo home/target dir, no undeclared `RUSTC_WRAPPER`, no editor/watcher state, and no global Cargo config unless inventoried.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] WATCH loop `ultragoal rust watch` invokes declared watcher and routes to `ultragoal rust fast`, emitting watch observations only.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] MEMORY/RESOURCE loop `ultragoal rust memory prove` exists and proves static resource audit, async-task audit, lifecycle tests, memory-budget performance, selected leak checks, and long-running telemetry where applicable.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

- [ ] GC loop `ultragoal gc plan`, `ultragoal gc dry-run`, `ultragoal gc apply`, and `ultragoal gc verify` exists and enforces plan digest, protected artifacts, deletion receipt, and post-delete verification.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status: validated current

### Gate 91.5: Receipt And Staleness Models

- [ ] Rust loop receipts include schema version, receipt id, law ids, command argv, cwd digest, toolchain identity, toolchain/Cargo digests, candidate/source digest, law graph digest, schema catalog digest, fixture catalog digest when applicable, environment class, OS/arch, env allowlist digest, Cargo home class, target dir class, cache mode, tool observations, proof surface, subject digest, timestamps, result, claim support, claim exclusions, and staleness policy.
  - Evidence:
  - Schema:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Rust receipts stale on source, Cargo lock/manifest, toolchain file, law registry, schema registry, validator binary, fixture suite, tool version, tool config, feature matrix, proof surface, package digest, install tree digest, runtime config digest, environment equivalence, or command identity changes outside allowed policy.
  - Evidence:
  - Red fixtures:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Cache receipts include cache mode, Cargo incremental, `RUSTC_WRAPPER`, target dir, Cargo home, sccache stats before/after when used, remote endpoint hash or none, cache policy digest, cache hit/miss where available, and timing class.
  - Evidence:
  - Schema:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.6: Runtime Memory And Resource Discipline

- [ ] Rust ownership, borrowing, RAII/drop, owned values, references, `Box<T>`, `Arc<T>`, and `Weak<T>` back edges are enforced as default memory/resource discipline where applicable.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] `Rc<T>`, `RefCell`, `Mutex`, `RwLock`, `DashMap`, `arc-swap`, crossbeam epoch tools, arenas, object pools, memory-mapped files, and alternate allocators are governed adapters with reason, scope, budget, metrics, drop/reset policy, tests, and receipts.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Tracing garbage-collection crates are rejected for the correctness-critical core.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Bounded caches are required wherever caching exists, and global unbounded maps/static lazy caches/non-canonical path keys/no-invalidation proof caches fail.
  - Evidence:
  - CLI command:
  - Red fixtures:
  - Candidate digest:
  - Status:

- [ ] Streaming parsers or explicit size-bound whole-file reads are required for large artifacts.
  - Evidence:
  - CLI command:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Every long-running task has owner, cancellation token or shutdown channel, bounded queue, tracked join handle, shutdown timeout, drop cleanup path, panic/error reporting, and memory/queue telemetry.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Spawn-and-forget tasks, unbounded channels, unbounded join sets, unmanaged background watchers, leaked tempdirs, child processes without kill/reap policy, and long-running loops without shutdown proof fail.
  - Evidence:
  - Red fixtures:
  - Candidate digest:
  - Status:

- [ ] Memory/resource receipts record scenario, binary digest, runtime config digest, duration, RSS budget/observed values, file descriptors, queue depth, child processes, temp bytes, tempdirs created/removed, child processes spawned/reaped, shutdown signal, cancellation delivery, joined task status, shutdown duration, leak adapters, result, and claim impact.
  - Evidence:
  - Schema:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Scheduled/scoped leak detection exists through selected Miri checks, long-running RSS stability, resource lifecycle tests, tempdir/child-process/fd cleanup tests, and governed adapters where applicable.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.7: Workspace, Artifact, And Cache Garbage Collection

- [ ] Runtime resource cleanup and workspace/artifact/cache cleanup are both represented; Cargo target/cache cleanup alone does not satisfy Harness proof hygiene.
  - Evidence:
  - Standards row:
  - Source-obligation row:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Every non-source workspace artifact is classified before cleanup.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Artifact classes include Cargo target, Cargo registry cache, Cargo git cache, sccache entries, nextest recordings, coverage artifacts, validation artifacts, current/stale receipts, final/review packets, generated source/non-source, package artifacts, install copies, plugin cache copies, worktree lanes, temp dirs, lock files, pid files, port reservations, trace logs, heap profiles, flamegraphs, and performance baselines.
  - Evidence:
  - Schema:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Unclassified artifacts cannot be deleted by automated cleanup and cannot support claims.
  - Evidence:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Protected artifacts include active-claim receipts, final packets, repair-needed failure receipts, release SBOM/provenance/signature/checksums, performance baselines, package/install/cache inventory receipts, Product Fitness/Cohesion/Success evidence, law/schema/source-obligation/standards registries, fixtures, source files, `Cargo.lock`, `rust-toolchain.toml`, and active review/archive evidence.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Protected artifacts cannot be deleted unless replacement proof exists or active goal is retired with typed disposition.
  - Evidence:
  - Red fixture:
  - Candidate digest:
  - Status:

- [ ] Cleanup after failed/interrupted agents inspects locks, pids, ports, child process records, temp dirs, partial receipts, partial package/install/cache copies, watcher state, and abandoned worktree lanes.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Blind `rm -rf` cleanup fails and cannot support Harness claims.
  - Evidence:
  - Red fixture:
  - Candidate digest:
  - Status:

### Gate 91.8: Rust Workspace And Module Topology

- [ ] Rust workspace/module shape mirrors authority boundaries: CLI, core/domain types, law registry, namespace, receipt, claim ceiling, fixtures, package, install, cache, product proof, diagnostics, observability, plugin integration, and xtask/bootstrap.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] CLI crate parses and dispatches only; law logic is not hidden in CLI command modules.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Rust module tree remains maximally factored with no residual prefix encoding, generic buckets, coverage-wave names, or unclassified generated/test/cache paths.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Active line-cap law applies to Rust source, tests, fixture manifests, CLI modules, validator modules, schemas, and generated files with generator receipts where applicable.
  - Evidence:
  - CLI command:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.9: Standards, Trace, Source-Obligation, Fixtures, And Package Integration

- [ ] Agent-standards rows cover Rust Developer Experience, Rust toolchain/substrate authority, Rust command loops, Rust cache/no-cache honesty, Rust memory/resource discipline, and workspace/artifact/cache GC.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Foundational trace maps AI Is Forcing Us To Write Good Code, Parse Don't Validate, Harness Engineering, Symphony, and ExecPlans requirements to Rust DevX/memory/GC enforcement surfaces.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Source-obligation parity represents Rust DevX, memory/resource discipline, and GC cleanup as first-class same-law surfaces or typed child laws with independent failure proof.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Schema catalog, validator check ids, red fixtures, green fixtures or valid receipts, tamper/stale/wrong-surface fixtures, package inventory entries, receipt schemas, claim-ceiling guards, and final packet fields exist for Gate 91.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status:

- [ ] Claim-ceiling guards block relevant claims when Rust DevX, cache/no-cache, memory/resource, or GC cleanup proof is missing, stale, wrong-surface, hidden-cache-dependent, unbounded, destructive, or non-CLI-built.
  - Evidence:
  - Command:
  - Receipt:
  - Candidate digest:
  - Status:

### Gate 91.10: Red, Green, Tamper, Validation, And Confidence

- [ ] Red fixtures prove raw-tool substitution, `cargo test` overclaim, hidden local state, stale toolchain/metadata/feature matrix, missing nextest/bootstrap, rejected watcher/script authority, typed-boundary failures, memory/resource violations, tracing GC as core, and unsafe cleanup all fail.
  - Evidence:
  - Fixture report:
  - Candidate digest:
  - Status:

- [ ] Green fixtures/valid receipts prove canonical fast, standard, release, clean-proof, watch observation, memory/resource, and GC dry-run/apply/verify flows pass on compliant minimal and realistic fixtures.
  - Evidence:
  - Fixture report:
  - Candidate digest:
  - Status:

- [ ] Tamper fixtures prove changed tool version, tool config, cache mode, source digest, Cargo lock digest, law graph digest, plan digest, protected artifact state, or deletion receipt invalidates proof and blocks claims.
  - Evidence:
  - Fixture report:
  - Candidate digest:
  - Status:

- [ ] Gate 91 validation commands run as applicable: toolchain verify, rust fast, rust standard, coverage prove exact, dependency audit, performance prove, memory prove, clean-proof, workspace topology check, GC plan/dry-run/apply/verify, source audit, and red fixture report.
  - Evidence:
  - Commands:
  - Receipts:
  - Candidate digest:
  - Status:

- [ ] Final packet includes Gate 91 confidence calculation using the required 100-point model: 15 doctrine/root cause alignment, 15 Rust ecosystem maturity, 20 direct CLI receipt enforcement, 15 cache/no-cache and clean-checkout honesty, 15 memory/resource proof, 10 GC protected-deletion proof, and 10 red/green/tamper completeness.
  - Evidence:
  - Calculated confidence:
  - Candidate digest:
  - Status:

- [ ] Gate 91 confidence is at least 96 percent only if command loops, cache/no-cache receipts, memory/resource receipts, GC receipts, standards/source-obligation/foundational trace entries, red/green/tamper fixtures, package inventory, source audit, and coverage proof pass on the same candidate.
  - Evidence:
  - Candidate digest:
  - Status:
