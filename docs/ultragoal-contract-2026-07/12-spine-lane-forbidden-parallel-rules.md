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
