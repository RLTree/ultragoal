# Observability And Repair Loop

## Sensory System

Observability is the harness sensory system. It exists so an agent can diagnose
and repair a failure without spelunking raw source or receipt walls.

- Four channels are required: metrics for alerting, traces and wide events for
  investigation, logs for reconstruction, and evals for behavioral quality.
- Two planes are required: product/system health and agent quality.
- Product truth, observability truth, and artifact truth stay separate and are
  reconciled by candidate digest, run id, correlation id, receipt id, and claim
  id.

## Law-Bearing Envelope

Every law-bearing command, check, receipt path, fixture path, setup path,
external path, long-running path, and claim guard emits an agent-legible
envelope when applicable:

- run id, correlation id, trace id, span id, parent span id;
- candidate digest, command, subcommand, operation, surface;
- law id, check id, claim id, artifact or receipt paths;
- status, failure class, why failed, where failed, next repair, claim impact;
- duration, worker count, task count, queue depth, cache mode, retry/backoff,
  saturation or resource state, redaction and boundedness state;
- before/after repair anchors when comparing runs.

High-cardinality values stay in logs, traces, wide events, and eval records.
Metric labels must remain bounded and must not contain raw run ids, trace ids,
span ids, user ids, candidate digests, receipt paths, private paths, or secrets.

## Query And Explain Contract

A fitted observable surface requires production proof, not just tests.

- Validation proves mechanics: parser, help, schema, field emission, redaction
  helper, fixture shape, and unit branches.
- Production proof requires a real current-candidate command run, useful
  stdout, same-candidate receipt, non-empty bounded log query, bounded metric
  signal, trace tree with valid parentage, useful explain output, source
  inspection, relevant fixtures, and explicit claim ceiling.
- Local spool or local JSON is transition/debug evidence only unless the stack
  is explicitly unavailable and the unavailable-live proof blocks higher claims.
- Query commands reject stale digest, wrong run/correlation, private path leak,
  unbounded output, fake duration, missing failure class, and missing repair
  hint.

## Repair Loop

The routine repair loop is executable:

1. Recompute or read the current candidate digest.
2. Run the highest-authority failing command once.
3. Query logs, metrics, and traces by run id, correlation id, and digest.
4. Run the relevant explain command.
5. Repair the smallest root cause.
6. Rerun the narrow command.
7. Compare before/after telemetry.
8. Run broad audit only at the slice or claim boundary.

`ultragoal current-state --json` is a read model, not proof. `ultragoal next`
is the navigator that consumes current-state and explain output to name the
first legal blocker, exact next repair, narrow rerun, forbidden actions, and
claim ceiling. Generic status dumps are not sufficient.

## Live Stack And Span Integrity

Live observability proof requires ingestion and queryability.

- Stack health alone does not prove Gate 92. It proves only stack health.
- Command proof requires root spans and meaningful child spans for validators,
  receipt binding, fixture execution, claim guards, external probes, exporters,
  and long-running lifecycle stages where relevant.
- Broken parentage, one-blob traces, local-spool-only completion claims, public
  port leaks, stale rows, and row-shape-only inventories fail the row.
