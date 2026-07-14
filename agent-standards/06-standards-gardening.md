# Standards Gardening

## Purpose

Standards gardening turns repeated agent friction into durable improvement
without bloating every prompt or adding hooks just to have hooks.

## Signal Threshold

Act on high-signal friction immediately when it is likely to recur:

- moderate or severe false completion;
- wrong review team, model, reasoning, or stale review anchors;
- broken worktree initialization or unsafe parent orchestration;
- product cohesion miss on a user-facing/control-surface claim;
- security, privacy, resolver, path, digest, or dependency trust failure;
- stale proof, stale queue state, backward state transition, or approval escape
  hatch;
- memory, wiki, replay, or archive capture that injects stale, generic, raw
  routing, hook, or future-dated payloads into ordinary work;
- durable repo hygiene drift where ephemeral worktree, cache, debug, or replay
  state appears in normal diffs;
- semantic namespace drift where names describe current goal work, proof
  chores, phases, slices, goal-work status, reviewer history, or session context
  instead of product behavior or domain responsibility;
- agents repeatedly needing chat context to understand why a path, module,
  function, helper, test, id, receipt, fixture, or artifact path segment exists;
- repeated user micromanagement required for a plugin flow that should be
  agent-owned.
- repeated opaque failures, stale or wrong-digest escapes, bad tool calls, bad
  repairs, slow workflows, reviewer findings, security near misses, product
  proof substitutions, docs drift, architecture violations, and lane
  regressions that should become evals, fixtures, laws, or validators.

Wait for repeated evidence before acting on low-signal friction:

- one-off wording confusion;
- local preference mismatch;
- rare repo-specific exception;
- issue already prevented by an existing gate that was not loaded.

Do not act on noise:

- purely subjective one-offs;
- broad rewrites without a concrete repeated failure class;
- hooks that add always-on context but do not mechanically prevent a real
  failure;
- new standards that cannot be discovered, enforced, or reviewed.

## Promotion Ladder

Choose the smallest durable surface that solves the failure class:

1. Mechanical validator, linter, schema, red fixture, or check when the rule is
   deterministic.
2. `agent-standards/enforcement.*` row update when the obligation must survive
   setup, resume, review, and completion.
3. Skill prompt update when the issue is workflow routing or procedure.
4. Persona prompt update when reviewers missed a recurring class.
5. Routed standards module when the rule is semantic but broadly applicable.
6. Product/resource map update when agents choose the wrong plugin surface.
7. Hook only when the check is cheap, deterministic, low-context, and prevents
   a frequent or severe error at the moment it happens.
8. Backlog row when the fix is real but too broad for the current lane.

Memory, wiki, replay, and archive quality fixes should prefer retrieval
thresholds, suppression rules, scrubbers, and quality receipts over always-on
prompt hooks.

Semantic namespace fixes should prefer product-behavior renames plus validator
fixtures over explanatory comments. If the only way to understand a name is to
read a goal document, receipt ledger, or parent-session steer, the name is not
durable agent infrastructure yet.

Improvement loops must close the chain from observed failure to durable law:
trace, feedback, cluster, eval or fixture, validator or standard, repair,
before/after telemetry, and promotion. Repeated friction cannot remain only in
chat, memory, or a reviewer note.

## Output

A gardening change records:

- triggering signal;
- failure class;
- chosen surface and why;
- rejected heavier options;
- validation command or blocker;
- claim ceiling after the change.

If the signal creates a durable standard, update `enforcement.json`,
`enforcement.tsv`, and `enforcement-audit.tsv` in the same change. Unclassified
rows are blockers, not reminders.
