---
name: harness-ultragoal
description: "Route Harness Ultragoal requests through one concise front door. Use for first entry, intent classification, capability and authority disclosure, or selection among repository fitting, routine work, diagnosis, durable goal execution, strict proof, maintenance, and product-journey review."
---

# Harness Ultragoal

Select one primary workflow from the operator's immediate outcome. Keep this
front door read-only and load only the selected skill.

## Establish current truth

Collect the intended outcome, canonical repository target when relevant,
current candidate identity, exposed host and CLI capabilities, requested
effects, and explicit authority. Treat every missing value as unknown.

When the binary is exposed, probe only read paths:

```text
ultragoal --json --help
ultragoal --json inspect capabilities
ultragoal --json inspect context
```

Do not infer availability, model, mode, reasoning, permissions, installation,
discovery, or runtime identity from the prompt or source tree.

## Select exactly one route

Apply this order to the immediate outcome:

1. Independent falsification or quality-in-use review ->
   `$harness-ultragoal:product-journey-review`.
2. One named claim and strict proof -> `$harness-ultragoal:prove`.
3. Symptom, failure, causal diagnosis, event query, repair plan, or next action
   -> `$harness-ultragoal:diagnose-and-observe`.
4. Fresh setup, partial fit, retrofit, or authority conflict ->
   `$harness-ultragoal:repository-fit`.
5. Affected checks, routine validation, or verified reuse ->
   `$harness-ultragoal:routine-work`.
6. Coordinated dependency-bound execution across multiple scopes ->
   `$harness-ultragoal:goal-run`.
7. Evaluation, research refresh, compatibility, migration, or retirement ->
   `$harness-ultragoal:improve-and-maintain`.

For a multi-outcome prompt, choose the earliest outcome the operator must
complete and report the others as follow-ons. Select `goal-run` only when the
immediate outcome is coordination itself. Never combine skill authorities.

## Fail closed

- A repository-dependent route without one canonical target selects no route;
  ask one focused target question.
- A write route without authority may be selected for inspection or planning,
  but execution stays blocked.
- Two genuinely simultaneous outcomes select `goal-run`, which must delegate
  disjoint work packages to their owning workflows.
- An unknown intent selects no route; ask one question whose answer changes the
  product outcome.
- A missing or incompatible typed capability blocks only the dependent action.
  Do not fall back to a compatibility source, duplicate store, helper, lane,
  gate, finalizer, or prose receipt.

Classification, help, parse, capability inspection, and route selection are
Read effects with zero hidden writes, including telemetry, caches, receipts,
and access metadata.

## Output

Return the selected skill or `no_route`, the immediate outcome, candidate and
target used, effect ceiling, authority required, supported current action,
unavailable capability, follow-on routes, exact blocker, and highest honest
candidate-only ceiling.

Documentation, tests, schemas, receipts, generated rows, telemetry, or green
commands do not prove package, installation, discovery, runtime behavior,
product journeys, readiness, release, or completion.
