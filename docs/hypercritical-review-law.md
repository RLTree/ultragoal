# Hypercritical Review Law

This package uses stricter review guidance for parent work and every subagent
review. The guidance is adapted from the `codex-workflow-rs` operating contract:
artifact-first proof, live-validation law, claim ceilings, and BLOCKED
semantics. It exists because ordinary approve/revise prompts missed material
proof gaps in this repo.
It becomes binding proof only when represented in typed receipts, validator
checks, detached artifacts, or generated repo standards.

## Binding Review Rules

1. **Reviewer job is falsification first.** A reviewer must actively search
   for the smallest counterexample that makes a supported claim false. Approval
   is allowed only after likely counterexamples were checked against current
   files or receipts.
2. **Current artifacts outrank prose.** A claim is supported only by live files,
   generated receipts, command output, target fixtures, or explicit blockers.
   README, reports, prompts, and prior reviewer notes are context, not proof.
3. **Static proof stays static.** Schemas, red fixtures, generated examples,
   package inventory, and target-repo fixtures prove only the package/static
   and fixture surfaces they exercise. They do not prove local Codex app
   visibility, install success, marketplace publication, real multi-lane
   dogfood, or real product UX quality.
4. **Route success is not product success.** A workflow, validator, or engine
   route can execute correctly while the product journey is still confusing,
   shallow, or unusable. Product Cohesion claims require user-journey proof,
   UI evidence, artifact integrity, and human-attention exception evidence.
5. **No proof substitution.** CLI proof cannot prove UI behavior; mock proof
   cannot prove live behavior; target fixtures cannot prove external products;
   generated examples cannot prove runtime execution; old receipts cannot prove
   the current package digest.
6. **Missing receipts are blockers.** A required command, artifact, digest,
   target receipt, reviewer signoff, or inventory closure that is absent,
   stale, failing, or unreachable is `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED`,
   not a non-blocking note.
7. **Receipt relevance decides severity.** A receipt mismatch blocks only when
   that exact receipt is the current claimed proof anchor for the phase, claim,
   install surface, review round, or distribution surface under review.
   Historical, detached, regenerated, superseded, or non-claimed receipts are
   context or cleanup work. Do not fail a review on receipt mismatch theater:
   identify the current authority, regenerate the stale artifact if needed, and
   keep moving unless the mismatch changes the claim ceiling.
8. **Check every repeated proof field.** When a count, id, digest, path, required
   list, generated artifact, or schema enum changes, reviewers must check every
   canonical and generated artifact that repeats it.
9. **Negative fixtures must pin the bug class.** A red fixture that catches only
   a simpler case is insufficient when the bug involved ordering, multiple
   rows, path escapes, stale digests, waiver disguise, or cross-field coupling.
10. **Claim ceilings are part of correctness.** If evidence supports only a
    package/static/fixture claim, the review must say so and reject any language
    that implies live install, app visibility, production readiness, or useful
    real-world product improvement.
11. **No approval by vibes.** An approval must name the current proof anchor:
    command, validator receipt path, review-target digest, counts, targeted
    probes, and any residual unsupported claims.

## Required Reviewer Output

Each reviewer must return:

- `Verdict: SIGN_OFF` only when no material blocker remains in the current
  full-scope sign-off round against the current validator, review-target,
  archive, registry, and claim-ceiling anchors.
- `Verdict: REVISE_BEFORE_NEXT_PHASE` when any claim, receipt, generated
  receipt, fixture, or proof boundary that is current authority for the claimed
  surface is stale, under-tested, self-attested, or substituting one surface for
  another. Detached or historical receipt drift is not enough by itself.
- `Verdict: BLOCKED` when required proof is unreachable or the reviewer cannot
  make a defensible finding from the supplied artifacts.
- `Material blockers` with file/line or command/receipt evidence.
- `Counterexamples attempted`, listing at least three concrete ways the package
  could be lying or stale.
- `Proof anchors checked`, naming exact files, receipts, commands, digests, or
  probes.
- `Claim ceiling`, split into supported, unsupported, and blocked.

Approval is invalid if the reviewer did not inspect the current validator
receipt and at least one source/fixture path directly related to their persona.
