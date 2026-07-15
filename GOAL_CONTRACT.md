# Harness Ultragoal Successor Live Goal Contract

## Authority

This is the current readable projection of the adopted
`harness-ultragoal-successor-contract-v2`. The immutable adopted base remains
the bundle rooted at
`docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256`.
Post-adoption requirements are authoritative only when an append-only row in
`AMENDMENTS.jsonl` binds this projection and its affected required claim IDs.

The base contract hash is the SHA-256 of the exact handoff-manifest bytes. A
current contract hash is the SHA-256 of the exact `GOAL_CONTRACT.md` bytes.
Each amendment hash is SHA-256 over the repository's canonical JSON for that
row with `amendment_hash` omitted: object keys sorted recursively, arrays
preserved, UTF-8 compact JSON, and a `sha256:` prefix. The amendment-log digest is the
SHA-256 of the exact `AMENDMENTS.jsonl` bytes. The first row uses the all-zero
SHA-256 as `previous_amendment_hash`; later rows use the preceding row's
`amendment_hash`.

## Objective

Complete Harness Ultragoal as the smallest calibrated agentic-engineering
harness that makes ambitious repository work executable, inspectable,
recoverable, and cheaper to supervise.

The product loop is:

1. choose a falsifiable problem;
2. write one lean outcome contract;
3. select the lowest sufficient model, reasoning effort, and orchestration
   route;
4. account for important alternatives and failure states;
5. verify each claim on its real surface;
6. ship reversibly; and
7. use real-world evidence to improve, narrow, or retire the harness.

## Required Claim IDs

The adopted claim graph remains closed and unchanged:

- `CL-SOURCE`
- `CL-PACKAGE`
- `CL-INSTALL`
- `CL-DISCOVERY`
- `CL-RUNTIME`
- `CL-FIT`
- `CL-ROUTINE`
- `CL-OBSERVABILITY`
- `CL-STRICT`
- `CL-ORCHESTRATION`
- `CL-EVAL-IMPROVEMENT`
- `CL-REAL-JOURNEY`
- `CL-RELEASE`
- `CL-COMPLETION`

No amendment may remove, merge, hide, or weaken one of these claims without an
explicitly approved weakening amendment, blocking backlog, and lowered claim
ceiling.

## Strengthened Requirements

### Custody and convergence

- N06 custody safety is enforced primarily through typed state, ownership,
  module privacy, opaque leaves, and explicit transitions. Partial source-text,
  alias, or control-flow inference cannot carry the custody claim.
- Illegal custody mutation, implicit destructive cleanup, caller-forged cleanup
  evidence, premature settlement, and release with live custody must be
  structurally prevented and behaviorally exercised across failure, panic,
  interruption, replay, recovery, cleanup, and false-pass routes.
- After materially distinct bypass classes recur against the same enforcement
  mechanism, root must record one decision before further patches: retain it
  with new evidence, replace it, or narrow the affected claim. This decision
  reuses existing authority; it does not create a counter, validator, receipt,
  tracker, or review subsystem.

### Review and proof economy

- Repair-loop checks stay focused and ephemeral. One independent source review
  exhaustively falsifies the complete named invariant and batches all material
  sibling, descendant, rollback, recovery, reconciliation, cleanup, replay,
  security, and false-pass defects into one decision.
- One material defect still means REWORK, but review does not stop at the first
  defect. A fresh review follows an exact new freeze.
- `WorkerResult-v1` remains the single lane handoff receipt and is generated
  once only after source acceptance. Rejected candidates do not regenerate it.
- Logs, targets, caches, scratch trees, compiler output, reproducible test
  output, and duplicate receipts remain ephemeral. Durable evidence is retained
  only for a named current claim, cross-process custody, irreproducible
  observation, or recovery need, with one owner, invalidation trigger, retention
  boundary, and deletion path.
- Full multi-persona and same-surface proof runs only at material integration,
  product, release, and completion boundaries where it can support or block a
  current claim.

### Current-source product value

- After N06 integration, the next product milestone is the smallest
  dependency-closed current-source installed journey. Package, install,
  discovery, and runtime identities are measured separately.
- That journey must exercise repository fit, dirty-tree routine work, a
  representative failure, diagnosis, recovery, preserved unrelated state, and
  a useful operator outcome on a real non-toy repository.
- Before any mastery-level or broadly reusable claim, repeat the critical
  journey on two materially different real repositories: one established and
  dirty, and one fresh or substantially different. If unavailable, keep the
  claim narrow and name the blocker.
- Product Fitness records one minimal manual-first row per journey: time to
  verified value, human interventions, review rounds, recovery outcome,
  retained artifact/cache cost, and any observed false pass or false rejection.
  No telemetry platform is added solely for these measurements.

### Calibrated enforcement

- Security, privacy, destructive-effect, and authority boundaries remain
  mandatory regardless of observed frequency.
- Other rules become universal only when recurrence or strong cross-repository
  evidence shows lower correctness or attention cost and the check is precise,
  inexpensive, and actionable.
- Guidance, checks, agents, workflows, and proof machinery that do not earn
  their maintenance and attention cost are simplified, made repository-fit
  specific, or retired without weakening a protected claim boundary.
- Active plans, worktrees, caches, candidate identities, amendment lineage, and
  claim ceilings are reconciled at integration boundaries. Reproducible
  artifacts and disposable caches are removed when no active claim or recovery
  need depends on them.

## Current Claim Ceiling

This amendment promotes no claim. The adopted contract and accepted bounded
source increments remain preserved. N06, its dependent N10/N11 increments,
current-source package/install/discovery/runtime behavior, Product Fitness,
real-repository journeys, readiness, release, completion, and mastery remain
withheld until their exact dependency-closed, same-surface evidence passes.

## Product Success Lineage

`examples/generated/PRODUCT_SUCCESS_CONTRACT.json` is the current schema-bound,
package-visible product-success authority for this live goal. Its claims map
one-to-one to the adopted claim IDs above. Product Fitness, Product Cohesion,
journey, readiness, release, and completion evidence must bind its current
contract ID and exact target digest. Its zero receipt digest is an explicit
contract-only sentinel; root issues the single current receipt only at the
product proof boundary.

## Prompting Basis

- https://developers.openai.com/api/docs/guides/prompt-guidance-gpt-5p6
- https://developers.openai.com/api/docs/guides/latest-model
- https://learn.chatgpt.com/docs/models

These sources support lean prompts, representative evaluation, lowest
sufficient reasoning, and Ultra only for meaningfully separable work. They do
not replace repository behavior or same-surface proof.
