# Harness Ultragoal Builder Contract Root - 2026-07

This directory is the binding builder-contract collection for the Harness
Ultragoal June 25 full compliance work. Load this file first. The legacy parent
prompt, checklist, and execution spine files are compatibility entrypoints only;
the active contract lives in this collection.

This collection is builder-contract authority, not package evidence. Editing
these files does not prove source, install, cache, app-registry, reviewer,
final-packet, readiness, release, completion, or `update_goal` status. If a
package digest or package evidence surface changes because these parent-session
builder-contract files changed, treat that as a package-boundary bug.

## Loading Order

1. Read this root file.
2. Read the spine module for the current phase:
   - `10-spine-operating-rules-and-doctrine.md`
   - `11-spine-phase-order.md`
   - `12-spine-lane-forbidden-parallel-rules.md`
3. Read the prompt module governing the active gate or surface.
4. Read the matching checklist module only for concise progress status.
5. Read additional modules only when the current task touches that authority
   surface. Do not load every file by default unless doing a full proof packet or
   contract migration.

## Module Map

Prompt authority:

- `01-prompt-product-doctrine-and-base-contract.md`: product doctrine,
  synthesis doctrine, stack addendum, validation evidence model, and base
  contract through the beginning of Gate 89.
- `02-prompt-gates-089-091-cli-rust.md`: CLI control plane, namespace topology,
  Rust developer experience, runtime resource discipline, and GC law.
- `03-prompt-gate-092-observability-fast-loop.md`: Gate 92 observability, the
  agent sensory system, `ultragoal loop run`, verified incremental audit,
  command telemetry roundtrip/reconciliation, telemetry, query/explain, and
  live-loop proof.
- `04-prompt-gates-093-105-research-product-evolution.md`: research authority,
  improvement loop, OpenAI/promptfoo/HALO, setup/retrofit, rollout, privacy,
  surface separation, and measured improvement laws.
- `05-prompt-update-goal-stop-conditions.md`: additional `update_goal` stop
  conditions and completion-response requirements from the prompt.

Execution spine authority:

- `10-spine-operating-rules-and-doctrine.md`: non-negotiable operating rules,
  anti-self-validation, package-boundary doctrine, product doctrine, stack
  doctrine, and carry-forward control-loop requirements.
- `11-spine-phase-order.md`: dependency order from Phase 0 through Phase 10,
  including the parent-owned Gate 92 fast-loop/fitting-compiler slice.
- `12-spine-lane-forbidden-parallel-rules.md`: lane rules, forbidden actions,
  parallel-first default, and disobedience detection.

Checklist tracking:

- `checklist/20-checklist-context-ledger-and-gates-000-088.md`: contract
  reference, live progress history, context loading, session audit, and gates
  before Gate 89.
- `checklist/21-checklist-gate-089-cli-control-plane.md`: Gate 89 tracking.
- `checklist/22-checklist-gates-090-091-namespace-rust-gc.md`: Gates 90-91
  tracking.
- `checklist/23-checklist-gate-092-observability-fast-loop.md`: Gate 92
  tracking, including live-loop and fitting compiler progress rows.
- `checklist/24-checklist-gates-093-105-research-product-evolution.md`: Gates
  93-105 tracking.
- `checklist/25-checklist-validation-update-goal-final-stop.md`: validation
  evidence list, `update_goal` blockers, doctrine status rows, final response
  fields, and explicit stop conditions.

## Operating Rules

- First action in a parent resume remains the canonical live package digest
  command unless the active user instruction explicitly says this is a
  builder-contract-only side edit. Treat receipts not bound to the current
  digest as stale for claim purposes.
- Work by dependency-closed slices. Do not start broad downstream work while the
  active slice has dirty source, stale receipts, no focused proof, or unresolved
  claim-boundary gaps.
- Phase 1A namespace/source-topology work includes purpose-backed active surface
  enforcement. Redundant binaries, commands, modules, functions, helpers,
  scripts, schemas, fixtures, receipt producers, generated rows, compatibility
  aliases, dead fallbacks, and coverage-only wrappers must be removed or typed
  with a product role, canonical owner, authority level, proof surface, claim
  limits, and sunset/removal rule before Gate 92 resumes.
- Checklist files are progress status only: `not started`, `in progress`,
  `implemented, pending validation`, `validated current`, or
  `stale due to source change`. Do not use checklist rows as receipt ledgers.
- Receipts are minted or refreshed only at claim boundaries: slice closure,
  phase gate same-candidate evidence, final source-local proof, or the exact
  package/install/cache/final-packet/update_goal proof surface in scope.
- Manual validation is risk-tiered. It is mandatory at claim-boundary points and
  suspicious CLI passes, but it must inspect real source/runtime behavior and
  tune validators. It must not become universal manual-receipt theater.
- Genuine proof is mandatory for every claim-bearing statement. Parser tests,
  schema shape, receipt existence, generated rows, current-state projections,
  workflow output, cache-key construction, synthetic timing, and CLI pass output
  are observations until they reconcile to actual current-candidate product
  behavior or to a verified current-input reuse/equivalence chain with explicit
  claim limits. A reused row from another candidate can support routine
  verified-local acceleration only; it cannot be called same-candidate
  production proof. Any unsupported proof-shaped output must lower the claim
  ceiling and fail the affected row.
- Harness-owned custom tooling is required wherever Rust ecosystem tools expose
  useful raw observations but cannot encode Harness product truth. Cargo,
  nextest, rustfmt, llvm-cov, tracing, OpenTelemetry, serde, schema parsers, and
  file watchers are execution substrate. They do not own law affected sets,
  proof-surface separation, verified reuse, claim ceilings, semantic namespace
  roles, receipt/artifact reconciliation, or agent-legible repair plans. Those
  authorities belong in `ultragoal` product tools such as `AuditContext`, the
  verified incremental query graph, product-surface input specs, command
  telemetry roundtrip/reconciliation, current-state, `ultragoal next`, and
  observe query/explain.
- Validation and proof remain separate for custom tools. Unit tests,
  schema-valid generated rows, fixture mechanics, cache-key construction,
  nextest output, llvm-cov JSON, tracing spans, workflow output, or local spool
  rows validate mechanics or emit observations. Production proof requires real
  current-candidate command behavior or verified current-input reuse with
  explicit claim limits, reconciled against independent surfaces such as stdout,
  receipts, source inspection, logs, metrics, traces/wide events, evals where
  applicable, and claim guards.
- Claim ceiling remains source-local until same-candidate source, install,
  cache, app-registry, reviewer, final-packet, and update_goal surfaces support
  stronger claims.

## Current Parent Priority

The current next broad source-local priority is Gate 92 production
observability and agent legibility, including the Harness-owned custom tooling
layer for the fast-loop, verified incremental audit engine, and command
telemetry roundtrip/reconciliation surfaces. Before broad Gate 92 work
continues, resolve any active Gate 90 semantic-namespace violations in paths,
modules, functions, tests, helpers, ids, and artifact paths.

Once the active Phase 1A/source-topology slice is cleanly closed, explicitly
blocked, or isolated away from root authority, the parent becomes an
orchestrator/reconciler for the Gate 92 custom-tooling program. It should launch
a small number of dependency-closed source-local worktree lanes instead of
implementing every custom tool itself. Every lane must be started through
`create_goal()` with a goal-specific contract, owned paths, forbidden paths,
proof-vs-validation requirements, no shared `validation_artifacts/**` writes,
and a source-local claim ceiling. The parent finalizes only after a lane returns
clean source, focused validation, production proof where the lane claims product
behavior, source inspection, and a not-readiness commit.

Do not refresh install/cache, finalize packets, claim readiness/release/
completion, or call `update_goal` before the applicable phase gates allow it.

## Done Means

The contract split is complete only when:

- every old prompt/checklist/spine section has a successor module or explicit
  compatibility-shim disposition;
- the old files point to this root and contain no unique hidden authority;
- module concatenation was checked against the pre-shim files;
- root routing lets a fresh agent find the active phase, first legal next
  action, proof versus validation requirements, and forbidden claims without
  reading every module;
- markdown/link/search checks pass for the split files; and
- any package digest movement caused by these builder-contract files is treated
  as a package-boundary bug, not new package proof.
