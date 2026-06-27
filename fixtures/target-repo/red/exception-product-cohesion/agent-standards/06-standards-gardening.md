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
- repeated user micromanagement required for a plugin flow that should be
  agent-owned.

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
2. Skill prompt update when the issue is workflow routing or procedure.
3. Persona prompt update when reviewers missed a recurring class.
4. Routed standards module when the rule is semantic but broadly applicable.
5. Product/resource map update when agents choose the wrong plugin surface.
6. Hook only when the check is cheap, deterministic, low-context, and prevents
   a frequent or severe error at the moment it happens.
7. Backlog row when the fix is real but too broad for the current lane.

Memory, wiki, replay, and archive quality fixes should prefer retrieval
thresholds, suppression rules, scrubbers, and quality receipts over always-on
prompt hooks.

## Output

A gardening change records:

- triggering signal;
- failure class;
- chosen surface and why;
- rejected heavier options;
- validation command or blocker;
- claim ceiling after the change.
