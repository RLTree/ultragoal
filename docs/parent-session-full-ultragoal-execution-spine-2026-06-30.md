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

### Phase 2 - Gate 92 Repair-Loop Spine

Make observability usable for the current highest-authority failing command.
Required loop: digest -> run failing command once -> query logs/metrics/traces by
run/correlation/digest -> explain failure through CLI -> repair smallest cause ->
rerun narrow command -> compare telemetry -> only then broad audit.

Exit requires same-digest stdout/log/metric/trace/explain proof for source audit
or final-packet proof. Full Gate 92 remains incomplete until Phase 7.

### Phase 3 - Gates 93-97 Touched-Surface Closure

Close research, improvement-loop, OpenAI, promptfoo, and HALO surfaces already
touched by WIP. Fix schema enum drift, standards TSV/JSON drift, source
obligations, red fixture schema/digests, valid fixtures, package inventory,
observability binding, and claim guards.

Exit requires focused tests and receipts for 93-97 source-local claims only.

### Phase 4 - Rebind Gates 0-91 Acceptance Spine

Rerun or stale-mark coverage, line caps, namespace/maximal factoring, typed
boundaries, Product/Fit/Journey, Rust/GC, standards, source obligations,
foundational trace, source audit, and red report on one digest.

Exit requires current source-local audit/red/coverage spine or named failures.

### Phase 5 - Gate 104 Closure For Gates 93-97

Prove Gates 93-97 exist across all mandatory law surfaces, not only files/rows.

Exit requires Gate 104 focused tests and source-audit coverage for Gates 93-97.

### Phase 6 - Gates 98-103

Implement domain-agent pattern, setup/retrofit, active-repo rollout, TypeScript
DevX, privacy/data minimization, and surface separation.

Worktrees may start only after Phase 4 is committed and parent owns all
`validation_artifacts/**` writes.

### Phase 7 - Full Gate 92 Fitting

Complete observability fitting for every law-bearing CLI command, validator
check family, receipt/proof path, fixture/report path, package/plugin surface,
operating-loop stage, and signal class.

Exit requires fitting control board pass with same-candidate query proof.

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
