# Observability And Repair Loop

## Sensory System

Observability is the harness sensory system. It exists so an agent can diagnose
and repair a failure without spelunking raw source or receipt walls.

- Four channels are available: metrics for aggregate alerting, traces and wide
  events for multi-step causality, logs for durable reconstruction, and evals
  for model or behavioral quality. Select channels by the claim and risk; all
  four are required together only for the observability capability claim and
  representative product or release proof.
- Two planes are required: product/system health and agent quality.
- Product truth, observability truth, and artifact truth stay separate and are
  reconciled by candidate digest, run id, correlation id, receipt id, and claim
  id.

## Law-Bearing Envelope

Every law-bearing command emits a typed, redacted diagnostic envelope. Checks,
receipt paths, fixtures, setup, external effects, long-running operations, and
claim guards add the fields and durable channels their failure and claim
surface require:

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

An observable command surface requires real command telemetry appropriate to
its claim, not just tests.

- Validation proves mechanics: parser, help, schema, field emission, redaction
  helper, fixture shape, and unit branches.
- Production proof requires a real current-candidate command run, useful
  output, an explicit claim ceiling, and the claim-relevant channels: durable
  logs for effects or reconstruction, metrics for aggregate rate/latency/
  capacity claims, traces for multi-step or cross-process causality, and evals
  for model-behavior or improvement claims. The observability capability and
  representative product/release proof join all four channels.
- Local spool or local JSON is transition/debug evidence only unless the stack
  is explicitly unavailable and the unavailable-live proof blocks higher claims.
- Query commands reject stale digest, wrong run/correlation, private path leak,
  unbounded output, fake duration, missing failure class, and missing repair
  hint.

## Repair Loop

The routine repair loop is executable:

1. Recompute or read the current candidate digest.
2. Run the highest-authority failing command once.
3. Query the claim-relevant logs, metrics, traces, or evals by operation
   identity and digest.
4. Run the relevant explain command.
5. Repair the smallest root cause.
6. Rerun the narrow command.
7. Compare before/after telemetry.
8. Run broad audit only at the strict claim boundary.

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
