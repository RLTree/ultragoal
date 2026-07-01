# Harness Ultragoal Full Compliance Execution Spine - 2026-06-30

This document controls execution order only. The canonical scope remains:

- `docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md`
- `docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md`

The spine exists to prevent receipt churn, broad-gate theater, premature lanes,
and readiness claims before the source-local proof graph is stable.

## Non-Negotiable Operating Rule

Work by dependency-closed slices, not by gate-number theater.

Exactly one broad slice may be active at a time. A slice may cross gates, but it
must close one claim-bearing production path. Do not start a second broad slice
while the first has dirty source, stale receipts, current failing focused tests,
no red/green/tamper proof where applicable, or no same-candidate receipt.

Every slice ends with:

1. recomputed package digest;
2. focused tests;
3. red/green/tamper or stale/wrong-surface proof where applicable;
4. current same-candidate receipt;
5. claim guard;
6. targeted manual source/runtime inspection when the slice closes or changes a
   claim-bearing validator, schema, fixture, receipt, claim guard, product
   claim, external-AI adapter, final packet, install/cache/app-registry surface,
   update_goal surface, or suspicious CLI pass;
7. existing checklist row updates only;
8. commit marked source-local/not readiness when a coherent checkpoint exists.

Checklist text is progress tracking only, never law evidence.

## Anti-Self-Validation Rule

The Ultragoal CLI is the system under development. Until final self-law
compliance is proven by the full contract, CLI `pass` output is a subject-under-
test signal, not claim authority by itself.

Manual validation is mandatory at claim-boundary points, not as a universal
receipt family stamped onto every mandatory-law row. Use it when a gate is being
marked complete, when a validator/check/schema/claim guard is added or changed,
when red/green/tamper semantics change, when Product Fitness/Product Success or
external-AI authority is involved, when final packet/update_goal/readiness/
install/cache/app-registry surfaces are touched, or when a CLI pass looks
suspicious.

Routine inner-loop checks may be tool-driven: fmt/build, focused unit tests,
line-cap, exact coverage, schema validation, package digest, targeted receipts,
and focused red/green/tamper tests. At slice boundaries, inspect representative
source paths, receipt JSON, schema or validator paths, fixture paths, and live
runtime/query output for the changed claim path. Full manual E2E dogfooding is
required before completion/readiness/release/update_goal claims.

Manual validation tunes and challenges the CLI. It does not replace the CLI, and
it must not become row-shape/manual-receipt theater. If manual inspection finds
the CLI passed too broadly, passed with stale evidence, passed a row-shape
substitute, or passed while a dependent law surface remains incomplete, repair
the validator/check/fixture/claim guard before the claim can move.

## Current State Assumption To Recompute

On every resume, recompute current package digest with the canonical CLI command.
Treat all receipts not bound to that digest as stale.

Do not trust older source-audit, red-report, coverage, Gate 92, Product/Fit,
Rust/GC, OpenAI, promptfoo, HALO, final-packet, install/cache, or update_goal
receipts until they are rebound or explicitly stale-marked.

## Builder-Contract Versus Package Boundary

The parent-session prompt, checklist, and execution spine are agent-governing
builder contracts for this work session. They are not package resources, plugin
product surfaces, coverage targets, package digest inputs, shipped law evidence,
valid fixture dependencies, product receipts, review/archive contents, install
inputs, cache inputs, registry inputs, or update_goal evidence.

The Ultragoal CLI and plugin package must not depend on these parent-session
files in any capacity. Editing them may change agent instructions and execution
order, but it must not stale package digest, coverage, source audit, Product/Fit,
review-target, archive, install/cache, registry, final-packet, or update_goal
receipts. Any current package manifest, package inventory, coverage manifest,
fixture, receipt, or validator check that treats these files as package-owned
must be repaired as a package-boundary bug before the affected claim can close.

## Carry-Forward Control Loop Requirements

The checklist is a progress surface, not a receipt ledger. Use only concise row
statuses: `not started`, `in progress`, `implemented, pending validation`,
`validated current`, and `stale due to source change`. Do not add progress-ledger
sections or churn receipt paths/digests through checklist rows during active
implementation.

Receipts are minted or refreshed only when they support a claim boundary: slice
closure, a phase gate requiring same-candidate evidence, final source-local proof
assembly, or an in-scope package/install/cache/final-packet/update_goal proof.
Otherwise use focused tests, stdout, direct source inspection, logs, metrics,
traces, and explain output as inner-loop evidence.

Validation is tiered:

- Inner loop: fmt/build, focused unit tests, line-cap, package digest, schema
  check, targeted receipt, and targeted red/green/tamper tests.
- Slice boundary: current digest, focused tests, touched red/green/tamper proof,
  same-candidate receipts, claim guard, targeted source/runtime inspection,
  checklist status updates only, and a source-local/not-readiness commit when
  coherent.
- Broad boundary: exact coverage, source audit, red fixture report, standards,
  source-obligation, and foundational trace closure.
- Completion boundary: full E2E/manual dogfood, CLI self-law, update_goal
  eligibility, final packet, install/cache/app-registry, and reviewer surfaces.

Do not rerun broad source audit/red report loops unless implementation or
evidence semantics changed. For repeated failures, run the narrow failing command
once, query telemetry by run/correlation/current digest, explain the failure,
repair the smallest production cause, rerun the narrow command, verify changed
telemetry, and only then run broad audit once.

Current dependency order is strict: if the red-fixture scheduler/
parallelization slice is dirty, close it immediately with focused proof and a
source-local/not-readiness commit, or explicitly stale-mark/shelve it without
claiming closure. After the current dirty slice is no longer ambiguous, full
research-rooted Gate 92 observability and agent legibility is the next broad
slice. Product Usage Fitness, Phase 4 rebinding, final proof assembly, and
worktree lanes wait until Gate 92 is fully fitted and committed.

## Phase Order

### Phase 0 - Stabilize WIP And Candidate Boundary

Classify dirty files, finish or shelve incomplete Gate 93-97/Gate 92 source
scaffolds, and prevent generated receipts or dependency installs from changing
candidate truth invisibly.

Exit requires: current digest, dirty-state inventory, no ambiguous partial
source scaffolds, package/evidence boundary plan, and no broad audit loop.

### Phase 1 - Package Inventory, Namespace, And Generated-Artifact Boundary

Repair package/resource/classification failures before chasing receipts. Resolve
`node_modules`, promptfoo artifacts, root lock/config files, package inventory
closure, exact-once listing, namespace classes, and generated/proof artifact
boundaries.

Exit requires: focused package/namespace tests, no broad orphan inventory
explosion, schema/catalog paths listed, current receipt, and claim guard.

### Phase 2 - Research-Rooted Full Gate 92 Observability And Agent Legibility

Gate 92 is now the top source-local priority after the current dirty slice is
unambiguous. Do not treat it as a current-blocker-only rescue, a representative
sample, or a later fitting backlog.

First bind the observability work to the foundational papers and additional
research already governed by Gate 93: OpenAI Harness Engineering, OpenAI Codex
repair loops, OpenAI Agents observability/tracing, Google SRE monitoring and
four golden signals, structured-event/high-cardinality doctrine, OpenTelemetry
semantic conventions, OpenAI improvement-loop research, and the self-improving
domain-agent article where it affects traces/evals/feedback loops. If any
observability requirement is not mapped through the research-source registry,
article-to-law trace, source obligations, foundational trace, standards rows,
validators, fixtures, package inventory, claim guards, and setup/retrofit
outputs, repair that mapping before claiming Gate 92 progress.

Complete observability fitting for every CLI/plugin production path: every
command and subcommand, validator check family, receipt/proof path,
fixture/report path, package/plugin/setup/retrofit surface, operating-loop
stage, signal class, long-running path, external/live path, and claim guard.
No minimum-surface, sample-based, current-failure-only, or adjacent-surface
substitution is allowed.

Required repair loop for every opaque failure encountered: digest -> run the
failing command once -> query logs/metrics/traces by run/correlation/digest ->
explain failure through CLI -> repair smallest cause -> rerun narrow command ->
compare telemetry -> only then broad audit.

Exit requires a passing fitting control board with every inventory row fitted on
same-candidate query proof, every mandatory research requirement current and
mapped to the Gate 92 law surface, full logs/metrics/traces/explain coverage for
all law-bearing command families and plugin surfaces, focused tests, red/green/
tamper fixtures, source inspection, current digest, concise checklist status
updates, and a source-local/not-readiness commit.

### Phase 3 - Gates 93-97 Remaining Touched-Surface Closure

After full Gate 92 closure, close remaining research, improvement-loop, OpenAI,
promptfoo, and HALO surfaces already touched by WIP. Fix schema enum drift,
standards TSV/JSON drift, source obligations, red fixture schema/digests, valid
fixtures, package inventory, observability binding, and claim guards.

Exit requires focused tests and receipts for 93-97 source-local claims only.

### Phase 3.5 - Product Usage Fitness And CLI Discoverability

After full Gate 92 closure and before Phase 4 evidence rebinding, close one
dependency-closed source-local product-usage slice.

The CLI/plugin must be usable as a product for plugin-activated repositories, not
only as a self-audit machine. Required validation paths must be obvious, simple,
and hard to skip. Preserve individual advanced entrypoints, but provide one
routine CLI entrypoint for ordinary required validation and self-contained help
that tells an un-oriented agent or user what to run, when, why, which proof
surface is affected, and which claims remain unsupported.

Exit requires focused help/routine-path tests and red fixtures for missing
routine entrypoint, non-navigable help, leaf-only validation substitution,
hidden fit-repo path, omitted target-repo/plugin-activated path, and
`scripts/check` failing to delegate or declare itself a narrow helper. Update
checklist rows with progress statuses only, then commit as source-local/not
readiness.

### Phase 4 - Rebind Gates 0-91 Acceptance Spine

Only after Gate 92 is fully fitted and Phase 3.5 closes or is explicitly
source-local blocked, rerun or stale-mark coverage, line caps, namespace/maximal
factoring, typed boundaries, Product/Fit/Journey, Rust/GC, standards, source
obligations, foundational trace, source audit, and red report on one digest.

Exit requires current source-local audit/red/coverage spine or named failures.

### Phase 5 - Gate 104 Closure For Gates 93-97

Prove Gates 93-97 exist across all mandatory law surfaces, not only files/rows.

Exit requires Gate 104 focused tests and source-audit coverage for Gates 93-97.

### Phase 6 - Gates 98-103

Implement domain-agent pattern, setup/retrofit, active-repo rollout, TypeScript
DevX, privacy/data minimization, and surface separation.

Worktrees may start only after Phase 4 is committed and parent owns all
`validation_artifacts/**` writes.

### Phase 7 - Gate 92 Regression And Propagation Guard

Full Gate 92 fitting is no longer deferred here. This phase only revalidates
that later Gates 93-103 work did not regress any observability inventory row,
research mapping, query proof, pass/fail output contract, trace parentage,
metric/log binding, help discoverability, or setup/retrofit propagation.

Exit requires the fitting control board to remain pass on same-candidate query
proof after later source changes.

### Phase 8 - Gate 105 Measured Improvement

Add baselines, current values, regression guards, telemetry comparison,
standards-gardener promotion, and claim guards for improvement claims.

### Phase 9 - Final Source-Local Proof

Run exact coverage, line-cap scan, focused tests, source audit, red report, CLI
self-law, update-goal eligibility, and final source-local claim ceiling.

### Phase 10 - Distribution Surfaces

Only after Phase 9 passes: install/cache refresh, version bump, package sync,
final packet, reviewer/app-registry exposure proof or unsupported-claim blocking,
and update_goal eligibility.

## Lane Rules

No lanes before Phase 4 is committed.

After Phase 4:

- Parent owns package digest, source audit, red report, receipts, final packet,
  package sync, version bump, and update_goal.
- No lane writes `validation_artifacts/**`.
- Lanes must own disjoint source paths.
- Candidate lanes:
  - Lane A: Gate 98 domain-agent pattern.
  - Lane B: Gate 99 setup/retrofit.
  - Lane C: Gate 100 active-repo rollout.
  - Lane D: Gate 101 TypeScript DevX.
  - Lane E: Gate 102 privacy/data minimization.
- Parent retains Gates 92, 94, 104, 105, and final synthesis.

## Forbidden Actions

Until Phase 9 passes: no install/cache refresh, version bump, final packet
finalization, app-registry/reviewer exposure claim, readiness/release/completion
claim, update_goal call, or worktree lane launch unless explicitly allowed by
this spine.

If any forbidden action happens, stop and repair the execution violation before
continuing.

## Parallel-First Default

Every safe CLI, plugin, validator, fixture, package scan, setup/retrofit,
observability, shell-helper, and proof path must use available parallelism by
default. Serial behavior is allowed only when the path is a typed
`shared_authority_write_serial`, `destructive_or_mutating_serial`, or externally
bounded live phase with an explicit reason.

The canonical scheduler task classes are `pure_read_parallel`,
`isolated_temp_write_parallel`, `external_live_bounded_parallel`,
`shared_authority_write_serial`, and `destructive_or_mutating_serial`. Safe
multi-item work must use the scheduler/executor or emit a fail-closed reason
showing why parallelization is impossible. Default workers are
`available_parallelism - 1`, minimum `1`, with bounded `--jobs N` where exposed.
Worker results must be deterministically ordered, fixture workers must use
isolated temp roots, and no worker may write shared `validation_artifacts/**`.

Scheduler/performance evidence must record worker count, task count, queue
depth, wall time, CPU time when available, memory/IO when available, cache mode,
resource-measurement status, candidate digest, and claim impact. Missing timing
or concurrency metadata blocks speed, routine-usability, product-readiness,
release, and update_goal claims.

## Disobedience Detection

A parent action is invalid if it:

- runs a broad audit repeatedly without implementation or evidence semantics
  change;
- treats coverage, red report, source audit, or fail-closed receipts as
  completion;
- uses stale or wrong-digest receipts as current;
- lets local JSON, Grafana, packet text, or checklist text substitute for CLI
  proof;
- launches lanes before Phase 4;
- writes shared receipts from lanes;
- claims source/install/cache/app-registry/reviewer surfaces interchangeably.

Invalid actions must become validator/CLI guard work, not prose.
