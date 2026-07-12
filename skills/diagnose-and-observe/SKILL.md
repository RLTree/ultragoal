---
name: diagnose-and-observe
description: "Diagnose Harness Ultragoal failures and query local semantic state without mutation. Use for causal explanations, typed findings, repair planning, exact reruns, deterministic next actions, local event queries, privacy review, or explicitly authorized export."
---

# Diagnose And Observe

Derive one causal explanation and one legal next action from current
candidate-bound state. Local events support diagnosis but never become claim
authority.

## Inputs

Require current candidate identity, a finding identifier or concrete symptom,
exposed capabilities, the bounded local event store, requested query, privacy
constraints, and any separately granted repair or export authority. If neither
a current finding nor a reproducible symptom exists, return a focused input
request instead of inventing a cause.

## Read-first workflow

Probe and use only the routes required for the request:

```text
ultragoal --json inspect context
ultragoal --json inspect findings
ultragoal --json observe query
ultragoal --json observe query --filter <identifier>
ultragoal --json diagnose --finding <finding-id>
ultragoal --json next
```

1. `inspect` establishes the current context, findings, capabilities, and claim
   ceilings.
2. `observe query` reads bounded local semantic events, applies redaction and
   cardinality limits, and reports corruption or truncation explicitly.
3. `diagnose` links observed facts to cause, smallest safe repair, exact rerun,
   effect, authority, and resulting ceiling. Label inference as inference.
4. `next` returns one deterministic legal action or authority request.

An ambiguous cause, stale finding, conflicting stores, unavailable command, or
candidate drift fails closed. Do not read duplicate status stores or
compatibility receipts as authority.

## Effects

Help, parse, inspect, diagnose, query, and next are Read effects with zero
hidden writes, including telemetry, caches, receipts, and access metadata.
Diagnosis grants no repair authority; execute the repair only through the skill
that owns its declared effect.

Export is a separate `ExternalWrite` and requires an explicit destination,
privacy boundary, consent, and approval:

```text
ultragoal --json observe export \
  --output <relative-output-path> \
  --approve-export
```

If any export prerequisite is absent, keep local diagnosis available and lower
only the export ceiling.

## Output

Report observed facts, inferences, causal chain, typed finding, smallest safe
repair, exact rerun, effect, authority, selected next action, redactions,
missing or corrupt data, unsupported capabilities, and highest candidate-only
ceiling.

Event presence, dashboards, receipts, schemas, tests, generated views, or a
successful query do not prove the underlying behavior, repair, observability
acceptance, readiness, release, or completion.
