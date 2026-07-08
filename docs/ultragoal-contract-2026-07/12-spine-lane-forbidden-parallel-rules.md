## Lane Rules

No general downstream lanes before Phase 4 is committed.

Exception: Phase 2 may use source-local custom-tooling worktree lanes after
Phase 1A active-surface/namespace work is cleanly closed, explicitly blocked, or
isolated away from root authority. This exception exists to build the Harness
authority layer required for Gate 92: `AuditContext`, product-surface input
specs, verified incremental query graph, routine hot-loop executor, coverage
lineage/intelligence, product-semantic surface inventory, command telemetry
roundtrip/reconciliation, observe explain/current-state/next-action, fixture
scheduling, and package truth snapshotting.

Every exception lane must:

- be launched by the parent through `create_goal()` with a lane-specific goal;
- own disjoint source paths and product-semantic module names;
- name forbidden paths, including shared root receipts and unrelated source;
- declare validation proof and production proof separately;
- forbid install/cache refresh, version bump, final packet finalization,
  registry/reviewer exposure claims, readiness/release/completion claims, and
  update_goal calls;
- avoid shared `validation_artifacts/**` writes from lane workers;
- preserve source-local/not-readiness claim ceiling;
- return a clean lane worktree, focused tests, red/green/tamper proof where
  touched, source inspection notes, and a not-readiness commit; and
- wait for the parent to reconcile root proof before any row or claim is marked
  validated/fitted/current.

The exception does not allow broad Phase 4 rebinding, install/cache proof,
reviewer exposure, final-packet work, Product Usage closure, or update_goal
eligibility before their normal phase gates.

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
