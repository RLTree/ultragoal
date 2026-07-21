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

### Evidence-led sequencing

- Project initiation and recovery select one high-information first truth loop
  bound to a real operator, job, repository, task, public entry surface, first
  value event, protected invariants, causal failure or recovery control,
  evidence class, and honest claim ceiling.
- The Product Success Brief is a typed inception projection, not a second goal
  contract. The Product Success Contract and append-only amendments remain
  normative. The inception route is read-only and cannot create, update, or
  approve the brief or promote a claim.
- Among dependency-legal actions, integration of an accepted candidate remains
  first. Remaining work ranks protected invariants before the first broken
  transition of the active truth loop, then false passes or false rejections,
  repeated cross-context gaps, bounded experiments, and speculative work.
  Numeric priority and caller-authored prose cannot bypass this ordering.
- Additional depth activates only from a protected invariant, a current typed
  observed failure, a repeated cross-context gap, or an explicit bounded
  experiment. Parked depth is derived on read; no trigger-state mirror,
  scheduler, tracker, receipt family, or permanent operator/observer pair is
  introduced.
- Repository fit, routine work, goal execution, diagnosis, and product-journey
  review remain the execution and observation authorities. Routine observations
  stay ephemeral until an existing canonical Product Fitness claim boundary
  requires one receipt.
- Intent, research, prototype, source, package, installed, runtime, agent-use,
  human-use, and repeated-human-use evidence remain distinct. Legacy dogfood
  orchestration receipts cannot substitute for Product Fitness, real use,
  human use, daily-driver status, readiness, release, or completion.

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
source increments remain preserved. The sealed host-lifecycle source increment
is integrated, but current-source package/install/discovery/runtime behavior,
Product Fitness, real-repository journeys, readiness, release, completion, and
mastery remain withheld until their exact dependency-closed, same-surface
evidence passes.

## Dependency and evidence semantics

- Source acceptance, execution outcome, and claim availability are orthogonal.
  N11 is source-accepted for its audit route and fail-closed public refusal;
  its positive execution outcome is externally blocked and
  `CL-EVAL-IMPROVEMENT` remains withheld.
- The N11 external blocker releases only root-owned N12 claim-ceiling
  reconciliation. It does not mean that N11 executed successfully and cannot
  promote an evaluation, product, runtime, release, or completion claim.
  N11 reopens when a host can jointly prove confinement and
  identity-conditioned cleanup, or when its source, consumed dependency,
  authority, or host-capability identity changes.
- `N12_A_STAGED` is a source-integrated implementation state consumed only by
  N14. It promotes no claim and binds its own source identity plus every
  contract, schema, claim-registry, dependency, generated-output, fixture, and
  effect identity it consumes. Any change reopens it.
- Preliminary N13 work is isolated, advisory, non-citable, and discarded before
  the final journey freeze. All tracked source, generated authority, public CLI,
  migration, retirement, and distribution mutations finish before that freeze.
- Product proof seals distinct source, generated, package, install, discovery,
  and runtime identities. A later mutation invalidates every affected proof;
  proof outputs and final reconciliation artifacts are excluded from the
  product identity only when their schema declares that exclusion.
- Aggregate review records preserve separate persona rows, operation IDs,
  prompt and schema hashes, structured-output validation, evidence pointers,
  and individual dispositions. An aggregate summary cannot replace those rows.
- N13 supports a single installed-journey ceiling by default. Daily-driver,
  broadly reusable, or mastery claims require the repeated real-repository and
  Product Fitness evidence above.
- N16 may establish release mechanics while blockers remain, but it cannot
  promote readiness or release. N17 is non-mutating and emits the typed final
  reconciliation artifact; completion remains withheld for every blocked or
  unsupported required claim.

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
