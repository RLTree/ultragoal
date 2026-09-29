# Product Success And Quality In Use

## Product Success Contract

A product milestone, repeated-use claim, daily-driver claim, or release claim
requires initiation-time Product Success Contract authority.

- The contract names the claim id, product surface, intended actor, job,
  context, desired outcome, critical journey, first value event, relevant
  quality dimensions, evidence tier, forbidden substitutions, review owner,
  and claim ceiling.
- A focused implementation lane may inherit the current milestone contract; it
  does not create its own product-success contract or receipt.
- Markdown-only planning can authorize the work but cannot prove the product
  claim. Stale, wrong-goal, wrong-candidate, placeholder, or actorless evidence
  lowers only the claim that consumes it.

## Product Fitness

Product Fitness asks whether the product helps the intended actor in the
declared context.

- Evidence distinguishes observed, inferred, assumed, and missing facts.
- Quality dimensions are selected because the current job depends on them;
  do not expand every claim into a universal quality audit.
- Adoption and continuance claims require current usage evidence. A single
  representative journey may prove first useful value only.
- Accessibility, cognitive load, recovery burden, trust burden, and human
  attention cost are evaluated when they are material to the current surface
  or claim.
- Memory, old screenshots, old packets, historical receipts, install success,
  unit tests, or reviewer agreement do not substitute for same-surface use.

## Product Cohesion

Product Cohesion asks whether the critical journey makes sense from beginning
to end.

- The journey binds steps, interaction boundaries, actor, first value event,
  failure path, recovery path, and the same-surface requirement.
- Cohesion, source checks, package publication, discovery, smoke tests, or
  fixtures may support a milestone but do not independently prove product
  success.
- Persist one concise milestone outcome only when it supports a current claim,
  cross-process handoff, recovery, or release. Ordinary journey observations
  may remain in command output and the active plan.

## Review Ownership

Product proof is owned, not inferred.

- The active plan names the Product/Simplicity reviewer or product owner.
- Review checks current contract lineage, forbidden substitutions, journey
  evidence, failure and recovery, and the claim ceiling.
- A full release packet or broader Fitness record is required only for the
  corresponding release, repeated-use, or daily-driver claim.
