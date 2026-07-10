# CLI and Custom Tooling Contract

## CLI role

`ultragoal` is a fast, agent-legible authority kernel. It owns Harness product semantics that ecosystem tools do not know: product surfaces, law applicability, affected-set legality, evidence congruence, claim guards, repair compilation, and exact next action.

It SHOULD use mature ecosystem primitives for parsing, compilation, test execution, coverage collection, serialization, tracing, and telemetry transport. It MUST NOT wrap those tools merely to create a branded duplicate.

## Command-effect types

Every command has a compile-time effect class:

- `read_only` — no repository, receipt, cache, install, application, or external mutation;
- `writes_artifact` — writes only an explicitly named output after `--emit <path>`;
- `writes_repository` — requires an `apply` verb, plan identity, root confinement, and preservation checks;
- `writes_install_state` — explicit install/upgrade action with before/after reconciliation;
- `writes_cache` — explicit cache operation; transparent cache population during computation must be isolated, atomic, and reported;
- `external_effect` — explicit authorization and captured remote identity;
- `destructive` — explicit plan, user authority, scope preview, and rollback or irreversibility warning.

Read-only commands MUST construct dependencies that cannot obtain a writer. A flag may narrow output; it may not silently upgrade effects.

## Successor command surface

```text
ultragoal status [--json]
ultragoal next [--json]
ultragoal fit inspect [--json]
ultragoal fit plan [--json] [--emit PATH]
ultragoal fit apply --plan PATH
ultragoal fit verify [--json]
ultragoal check [--changed|--all] [--json]
ultragoal prove --surface SURFACE [--json] [--emit PATH]
ultragoal package snapshot [--json] [--emit PATH]
ultragoal package verify --snapshot PATH [--json]
ultragoal package diff --left PATH --right PATH [--json]
ultragoal observe query QUERY [--json]
ultragoal observe explain FINDING [--json]
ultragoal goal inspect [--json]
ultragoal goal reconcile [--json] [--emit PATH]
ultragoal maintain plan [--json] [--emit PATH]
ultragoal maintain apply --plan PATH
ultragoal maintain verify [--json]
ultragoal help [COMMAND]
ultragoal version [--json]
```

Specialized validators remain library selectors or hidden diagnostic commands unless a user journey justifies public exposure. `audit`, `review`, `standards`, `product`, `final-packet`, `source-obligations`, `mandatory-law`, `foundational-law`, and similar overlapping authorities consolidate behind `check`, `prove`, and the canonical findings graph.

### Public command effects

| Command | Base effect | Allowed additional effect |
|---|---|---|
| `status`, `next`, `help`, `version` | `read_only` | none |
| `fit inspect`, `fit verify` | `read_only` | none |
| `fit plan` | `read_only` | `writes_artifact` only with explicit `--emit` |
| `fit apply` | `writes_repository` | only effects enumerated and digest-bound by the supplied plan |
| `check` | `writes_cache` | product-managed build/query cache only; no receipt/artifact write |
| `prove` | `writes_cache` | `writes_artifact` only with explicit `--emit`; external/live probes only when the requested surface declares them and authority is present |
| `package snapshot` | `read_only` | `writes_artifact` only with explicit `--emit` |
| `package verify`, `package diff` | `read_only` | none |
| `observe query`, `observe explain` | `read_only` | none |
| `goal inspect` | `read_only` | none |
| `goal reconcile` | `read_only` | `writes_artifact` only with explicit `--emit`; it never mutates platform goal state |
| `maintain plan`, `maintain verify` | `read_only` | `writes_artifact` only for `plan --emit` |
| `maintain apply` | effect set in plan | destructive effects require the explicit destructive plan protocol |

Cargo target output, coverage profiles, temporary fixture state, and query-cache population are writes even when they are implementation substrate. The command MUST disclose them as `writes_cache`, confine them to product-managed locations, reconcile interruption, and exclude them from contract/product evidence unless separately captured and reconciled. A subprocess cannot launder a write through a read-only parent command.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Command completed and requested non-claim query has no blocker, or named claim is satisfied at its requested ceiling |
| 1 | Product finding prevents the requested claim or action |
| 2 | CLI usage or typed input error |
| 3 | Required product surface is unavailable or unauthorized |
| 4 | Evidence is stale, invalid, tampered, or wrong-surface |
| 5 | Internal tool failure or contradictory authority state |
| 130 | Interrupted; partial effects are reconciled before return where possible |

Machine output MUST include `schema_version`, `command`, `effect`, `context_id`, `status`, `claim_ceiling`, `findings`, and `next_action`. Human output MUST be concise and stable enough to copy the repair command.

## Parsing and domain types

Use a maintained Rust parser such as `clap` after dependency and build-cost review. Public inputs MUST parse into domain types, not merely non-empty strings:

- `RepositoryRoot` — canonicalized authorized root with symlink policy;
- `ProductSurface` — closed enum with versioned extension path;
- `ContextId`, `CandidateId`, `PackageDigest`, `ArtifactId` — algorithm-qualified identities;
- `EffectClass` — closed effect enum;
- `ClaimCeiling` — ordered enum;
- `DiagnosticId` and `LawId` — validated stable semantic IDs;
- `EvidenceRef` — typed scheme and dereference rules;
- `OutputPath` — confined and collision-aware;
- `Query` — parsed observability expression with bounded resource use;
- `PlanId` — digest-bound mutation plan.

Unknown enum values in forward-compatible machine input MUST be preserved or rejected according to schema version, never silently coerced.

## Library architecture

The binary should be a thin adapter over these product modules:

```text
context -> inventory/query graph -> product rules -> findings
findings -> repair compiler -> status/next views
evidence + independent observations -> reconciler -> claims
explicit plan -> effect executor -> postcondition reconciler
```

Rules are pure where possible. Subprocesses, filesystem, clocks, environment, install state, application state, network, and telemetry are explicit ports. Command result capture is structured and independent from claim evaluation.

## Definitive custom-tool model

The machine-readable inventory in `CUSTOM_TOOL_INVENTORY.json` is canonical. The following sections define required behavior.

### CT-AUDIT-CONTEXT — AuditContext and immutable snapshots

**Status:** partial and fragmented.  
**Role:** capture all relevant current inputs once and give every result a congruent identity.  
**Substrate:** `serde`, SHA-256 or reviewed successor digest, Git/file metadata, Rust environment adapters.  
**Independent proof:** two independently enumerated snapshots agree; changed inputs invalidate only affected nodes; dirty/untracked/symlink cases are exercised.  
**Replaces:** command-local roots, ad hoc source digests, receipt-local timestamps, inconsistent candidate IDs.  
**Retirement:** remove secondary context constructors after every authority command accepts the canonical type.

### CT-SURFACE-SPEC — Product-surface input specifications

**Status:** partial in manifests, schemas, and checks.  
**Role:** declare exact inputs, applicability, identity, effect, proof, and invalidation for each product surface.  
**Substrate:** versioned Rust/Serde schema and generated JSON views.  
**Independent proof:** omit, add, rename, and misclassify a required input; verify fail-closed impact and diagnostic.  
**Replaces:** hand-maintained file lists and gate-specific input enumerations.  
**Retirement:** generated legacy manifests remain compatibility views for one migration version.

### CT-INCREMENTAL-GRAPH — Verified incremental query graph

**Status:** partial in affected-set, coverage, package, and current-state code.  
**Role:** compute legal affected closure and reusable node results.  
**Substrate:** Rust DAG/query engine, Cargo metadata/messages, semantic inventory, filesystem watcher only as advisory input.  
**Independent proof:** compare incremental results to strict no-cache results over mutation corpus; false negatives are blockers.  
**Replaces:** static changed-file manifests and independent affected-set calculators.  
**Retirement:** remove legacy mappers only after equivalence coverage spans their surfaces.

### CT-NODE-CACHE — Node-local cache and invalidation authority

**Status:** fragmented; verified-reuse concepts exist.  
**Role:** store immutable query-node results keyed by semantics, dependencies, toolchain, environment predicates, and candidate.  
**Substrate:** content-addressed local store, atomic file operations, Serde, integrity digest.  
**Independent proof:** tamper, partial write, tool-version change, environment change, transitive-input change, and collision simulations.  
**Replaces:** receipt reuse and cache claims based on path/timestamp.  
**Retirement:** unverified result reuse becomes observation-only or is deleted.

### CT-SEMANTIC-INVENTORY — Semantic surface and symbol inventory

**Status:** partial across Rust DevX, package, schema, and component inventories.  
**Role:** identify product components, public commands, laws, validators, schemas, symbols, generated ownership, and dependency edges.  
**Substrate:** Cargo metadata, rustdoc/ compiler JSON where useful, parsers, manifest metadata.  
**Independent proof:** compare to compiler/package outputs and known omission fixtures.  
**Replaces:** giant hand-edited resource and standards lists as primary authority.  
**Retirement:** retain generated human-readable catalogs only.

### CT-PACKAGE-SNAPSHOT — Package truth snapshotter

**Status:** substantial but fragmented among package inventory/digest/cohesion surfaces.  
**Role:** produce canonical normalized package contents, ownership, modes, links, generated provenance, version, and digest.  
**Substrate:** filesystem/archive readers, manifest generator, Serde, digest library; optional release provenance/signature adapters.  
**Independent proof:** build archive independently, compare extracted bytes, modes, links, omissions, unexpected files, and installed tree.  
**Replaces:** independent package list, digest, manifest, and cohesion claim authorities.  
**Retirement:** old commands become aliases or internal views, then are removed.

### CT-COVERAGE — Routine and strict coverage intelligence

**Status:** implemented in breadth, weakly reconciled and over-coupled.  
**Role:** explain uncovered changed behavior quickly and produce strict whole-surface coverage when required.  
**Substrate:** `cargo llvm-cov`, rustc instrumentation, nextest, source/semantic inventory, optional mutation runner.  
**Independent proof:** no-cache strict comparison, known-covered/uncovered mutations, stale-profdata rejection, excluded/generated code review.  
**Replaces:** embedded Python policy, duplicate scripts, one-dimensional percentage authority.  
**Retirement:** shell scripts become thin non-authoritative adapters or disappear.

### CT-IMPACTED-TESTS — Impacted-test mapping

**Status:** partial.  
**Role:** map changed semantic nodes to the smallest legal test set and explain uncertainty.  
**Substrate:** Cargo metadata/messages, nextest listings/partitions, semantic inventory, historical traces as advisory input.  
**Independent proof:** mutation corpus compares selected tests with strict suite; misses expand graph and block verified reuse.  
**Replaces:** static path-pattern and manual changed-test lists.  
**Retirement:** heuristic rules may remain only as conservative seeds.

### CT-FIXTURE-SCHEDULER — Isolated fixture scheduling

**Status:** partial with useful typed classes and temporary isolation.  
**Role:** schedule behavioral red/green/tamper cases without cross-test state leakage.  
**Substrate:** nextest where appropriate, Rust process execution, temp directories, port/resource allocator.  
**Independent proof:** deliberate file, environment, process, port, cache, telemetry, and install-state leakage.  
**Replaces:** giant hand-maintained fixture catalog and narrow marker-only leak checks.  
**Retirement:** generate catalogs from fixture metadata; delete boolean-only cases after behavioral replacement.

### CT-COMMAND-CAPTURE — Command result capture and reconciliation

**Status:** fragmented across receipts and subprocess helpers.  
**Role:** capture argv, cwd identity, environment allowlist, stdout/stderr references, exit/signal, duration, resource scope, and output artifacts without interpreting claim success.  
**Substrate:** Rust process API, monotonic clock, bounded capture store, Serde, tracing.  
**Independent proof:** signal, timeout, truncation, non-UTF8, partial output, spoofed text, and output-file mismatch cases.  
**Replaces:** command-specific receipt shapes and “printed PASS” parsing.  
**Retirement:** migrate readers through typed adapters, then remove legacy writers.

### CT-TELEMETRY-RECONCILE — Telemetry roundtrip reconciliation

**Status:** partial.  
**Role:** prove that a semantic event emitted for the current operation is queryable locally and, when required, through the selected export/backend.  
**Substrate:** `tracing`, OpenTelemetry SDK/exporters, local JSON event spool, backend query adapters.  
**Independent proof:** correlation mismatch, dropped event, duplicate, reorder, redaction, exporter failure, backend delay, and wrong-candidate cases.  
**Replaces:** receipt-only telemetry presence and backend dashboard screenshots.  
**Retirement:** external stack remains optional integration substrate, not core authority.

### CT-ARTIFACT-DEREF — Receipt and artifact dereferencing

**Status:** fragmented.  
**Role:** resolve typed evidence references, verify identity/integrity/schema/provenance, and state whether the artifact is current and congruent.  
**Substrate:** URI-like typed references, Serde schemas, digest store, path confinement.  
**Independent proof:** missing, moved, symlinked, tampered, truncated, wrong-schema, wrong-context, and private-path cases.  
**Replaces:** raw path strings and receipt-presence checks.  
**Retirement:** legacy paths accepted only through explicit migration adapters.

### CT-CURRENT-STATE — Current-state read model

**Status:** partial.  
**Role:** project one immutable, queryable view of surfaces, findings, work packages, proof, and claim ceilings.  
**Substrate:** authority graph and event log; no separate state declarations.  
**Independent proof:** rebuild from canonical inputs/events and compare; injected contradictory receipt cannot change state.  
**Replaces:** status checklists, gate progress blocks, lane launch statuses, and isolated current-state files.  
**Retirement:** historical progress moves to release history outside the active contract.

### CT-NEXT — `ultragoal next`

**Status:** partial.  
**Role:** select exactly one highest-priority legal repair/action from current findings and dependency state.  
**Substrate:** current-state model, typed repair compiler, deterministic priority rules.  
**Independent proof:** tie, blocked dependency, optional surface, stale context, dirty tree, and no-op cases; repeated queries are side-effect-free.  
**Replaces:** manual queue selection, static lane launch order, and checklist scanning.  
**Retirement:** no separate “next action” authoring surface remains.

### CT-OBSERVE-REPAIR — Observe query/explain repair compiler

**Status:** partial.  
**Role:** query bounded semantic events, explain causal findings, and compile exact safe repairs.  
**Substrate:** local event spool/query parser, tracing semantics, evidence dereferencer, authority graph.  
**Independent proof:** injected causes, ambiguous correlations, missing events, untrusted attributes, and operator comprehension.  
**Replaces:** raw JSON greps, dashboard dependence, and command-specific repair prose.  
**Retirement:** keep backend-specific query adapters only at the boundary.

### CT-PLUGIN-DISCOVERY — Plugin installation and discovery verifier

**Status:** fragmented and incomplete.  
**Role:** reconcile package, install, cache, application registry, skill discovery, and representative runtime identity.  
**Substrate:** package snapshotter, filesystem, host-supported registry/API/browser adapters, new-task probe.  
**Independent proof:** byte mismatch, stale cache, duplicate version, wrong home, registered-hidden, source-only substitution, and real task.  
**Replaces:** install receipts, manifest presence, and manual plugin-UI assertion as standalone proof.  
**Retirement:** environment-specific scripts become adapters with no claim authority.

### CT-FIT-RECONCILE — Setup and retrofit reconciler

**Status:** partial and receipt-heavy.  
**Role:** inspect/plan/apply/verify repository fitting while preserving user decisions.  
**Substrate:** file ownership inventory, templates, semantic diff, host load probe, routine check.  
**Independent proof:** pre-existing instructions, local edits, partially installed harness, conflicting commands, generated drift, rollback.  
**Replaces:** separate initializer/retrofit authorities and file-presence success.  
**Retirement:** custom agents may remain UX roles but invoke one fit engine.

### CT-AGENT-EVAL — Agent-quality evaluation and failure harvesting

**Status:** partial, adapter- and receipt-heavy.  
**Role:** validate evaluation data, run reproducible agent/product journeys, harvest failures, and propose scoped improvements.  
**Substrate:** command capture, trace/event data, replay fixtures, approved model/eval APIs, review workflow.  
**Independent proof:** broken-task audit, scoring perturbation, leakage/contamination, replay stability, negative controls.  
**Replaces:** score-only promotion, prompt/eval adapter presence, and self-reported improvement.  
**Retirement:** vendor-specific adapters remain optional transport, not authority.

### CT-CLAIM-GRAPH — Claim authority and evidence reconciliation

**Status:** fragmented across audit, product, standards, review, and final-packet commands.  
**Role:** join law, surface, behavior, evidence, independent observation, finding, guard, and ceiling.  
**Substrate:** typed Rust graph, Serde schemas, all tools above.  
**Independent proof:** contradictory, omitted, stale, wrong-surface, self-signed, and partial evidence cases.  
**Replaces:** every duplicate top-level readiness/completion engine.  
**Retirement:** legacy commands become read-only filtered views for one compatibility version.

## Substrate policy

Preferred substrate, subject to measured suitability:

- Cargo for build graph and package metadata;
- nextest for test listing, partitioning, retries policy, archives, and isolated execution where appropriate;
- rustfmt for formatting, not semantic correctness;
- rustc instrumentation and `cargo llvm-cov` for coverage collection;
- Serde for versioned machine contracts;
- `tracing` and OpenTelemetry for event production/export;
- conventional shell execution only as captured substrate, never a source of Harness claim semantics;
- `clap` or a similarly mature parser for CLI grammar.

A custom layer is justified only when it owns Harness-specific semantics, provides measured performance/reliability value, or closes a missing integrity boundary. Justification MUST name the ecosystem primitive considered, the gap, measurement, maintenance owner, and retirement condition.

## Performance budgets

Budgets are product targets to validate, not authored facts:

- `status` and `next`: warm p95 under 250 ms for representative repositories;
- routine changed-surface planning: warm p95 under 500 ms;
- routine checks: bounded by selected tool work and reported by phase; no hidden global audit;
- cache hit validation: cheaper than recomputation and independently measured;
- strict proof: no universal wall-clock promise, but phase timings, executed/reused work, critical path, and interruption behavior are mandatory.

Repository-size tiers and hardware profiles MUST be defined before promotion. Bookkeeping-only timing cannot satisfy a budget.

## CLI completion blockers

- any read-only command can mutate artifacts, cache authority, repository, install, app, or external state;
- duplicate commands can produce different claim ceilings for the same context;
- routine mode performs unannounced strict/global work;
- unverified cache reuse can pass;
- parsing accepts semantically invalid roots, surfaces, identifiers, or output paths;
- output lacks exact repair, rerun, effect, or ceiling;
- package/install/app/runtime identity cannot be reconciled;
- ecosystem primitives are duplicated without measured product need;
- independent behavioral proof is absent for a completion-bearing tool.
