---
name: prove
description: "Plan and execute strict proof for one named Harness Ultragoal claim. Use for dependency-closed validation, current evidence binding, false-pass controls, negative cases, independent reconciliation, exact claim ceilings, or falsifying readiness and completion assertions."
---

# Prove

Prove one claim against its exact live truth surface. Keep implementation,
evidence production, independent reconciliation, and root claim decisions
separate.

## Inputs

Require one claim identifier, current candidate identity, prerequisite state,
expected behavior, exact truth surface, freshness rule, required tools and host
surfaces, positive checks, false-pass controls, designated independent
reconciler, bounded output path, and explicit write authority. A request for
generic readiness without one claim returns a scope question.

## Build and execute one proof plan

Inspect first:

```text
ultragoal --json inspect claims
ultragoal --json inspect context
ultragoal --json inspect capabilities
```

Close only the named claim's dependencies. Identify missing prerequisites,
wrong truth layers, stale evidence, unresolved decisions, unavailable tools,
and required external observations. Define positive behavior checks plus stale,
omitted, tampered, contradictory, self-signed, wrong-surface, bypassed, and
reward-hacked controls.

Run dependency-closed validation and proof only when the route is exposed and
the output is authorized:

```text
ultragoal --json check strict \
  --target <relative-path> \
  --claim <claim-id>

ultragoal --json prove \
  --claim <claim-id> \
  --output <relative-output-path>
```

Treat proof output as bounded `WorkspaceWrite`. Forbid hidden telemetry,
mutable claim authority, undeclared artifacts, or source/package/install/host
effects. Revalidate the candidate before and after execution.

## Independent reconciliation

Send raw observations and artifacts to an independent reviewer who did not
implement the material behavior or produce the evidence. The reviewer attacks
the exact claim, controls, freshness, candidate binding, and truth surface.
Preserve rejected evidence with its reason. Root and required human authority
alone may accept or promote the claim.

If the claim depends on package, installed bytes, host discovery, Plugins UI,
runtime behavior, or a real journey, require that exact live surface. A strict
source or test pass cannot cross those layers.

## Output

Report claim and prerequisites, proof plan, positive and negative controls,
exact observations, artifact digests, effects, failures, independent findings,
allowed ceiling, repair, and rerun.

Documentation, schemas, test existence, receipts, generated rows, telemetry,
signatures, provenance, package bytes, or green commands cannot substitute for
the named behavior. This skill never decides readiness, release, or completion.
