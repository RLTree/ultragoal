---
name: routine-work
description: "Run conservative affected validation while preserving dirty repository state. Use during normal development to select changed-impact checks, verify safe reuse, expand uncertain dependencies, report exact execution, or avoid unnecessary global and release work."
---

# Routine Work

Run the smallest dependency-closed routine profile that is safe for the current
change. Preserve the operator's dirty tree and report executed and reused work
separately.

## Inputs

Require one canonical repository root, current candidate identity, modified,
staged, untracked, ignored-artifact, and worktree state, the requested change
boundary, semantic impact graph, available checks, reuse records, and declared
local artifact roots. A repository-dependent request without a target returns
a blocker instead of guessing.

## Execute the current affected set

Probe the route and snapshot the full declared repository boundary with
read-only operations, then run:

```text
ultragoal --json check routine --target <relative-path>
```

1. Compute the conservative semantic affected set from current changes.
2. Expand every unknown dependency edge and include required prerequisites.
3. Reuse work only when candidate, inputs, tool identity, environment, command,
   and artifact digests all match. Unverifiable reuse becomes work to execute.
4. Constrain the structural `WorkspaceWrite` effect to declared local build and
   test artifacts. Forbid hidden telemetry, caches, receipts, or generated
   authority elsewhere.
5. Compare recursive content, modes, links, relevant timestamps, Git status,
   and worktrees at the same scope after the command. Report every unexpected mutation.

Never stash, clean, reset, checkout, stage, commit, rewrite unrelated files, or
use a broad global check merely because impact analysis is uncertain. Expand
the affected set until it is legally closed.

## Repeat use, failure, and interruption

On an unchanged second run, report each verified reuse and the matching key;
do not relabel skipped or cached work as executed. On failure, preserve the
tree, identify the causal check and smallest rerun, and route diagnosis to
`$harness-ultragoal:diagnose-and-observe`. On interruption, report completed,
in-flight, unstarted, and invalidated reuse separately before retrying.

A named claim or strict dependency boundary routes next to
`$harness-ultragoal:prove`. Routine work never promotes a claim or becomes
release ceremony by itself.

## Output

Report selected, expanded, executed, verified-reused, skipped, interrupted, and
failed work separately; include causal findings, exact reruns, declared
artifacts, measured timing when available, preservation comparison,
unsupported capabilities, and highest candidate-only ceiling.

Tests, receipts, timing fields, generated rows, documentation, telemetry, or a
green subset do not prove correct impact closure, safe reuse, preserved state,
strict proof, readiness, release, or completion.
