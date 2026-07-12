# Review And Completion

## Hypercritical Review Law

Reviewers falsify first. Approval is allowed only after likely counterexamples
were checked against current files, receipts, commands, fixtures, or runtime
evidence.

- Current artifacts outrank prose.
- No proof substitution: CLI proof does not prove UI behavior; mock proof does
  not prove live behavior; target fixtures do not prove external products; old
  receipts do not prove the current package.
- Source proof, install proof, cache proof, app-registry proof, reviewer
  exposure proof, final-packet proof, and update-goal proof are separate
  surfaces. A pass on one surface cannot close another.
- Missing receipts, stale digests, blocked approvals, dirty worktrees, absent
  required commands, or unreachable proof are `REVISE_BEFORE_NEXT_PHASE` or
  `BLOCKED`, not non-blocking notes.
- Receipt relevance decides severity. A mismatch blocks only when that exact
  receipt is the current claimed proof anchor for the reviewed phase, claim,
  install surface, review round, or distribution surface. Historical, detached,
  regenerated, superseded, or non-claimed receipt drift is cleanup or context,
  not a reason to fail the round unless it changes the claim ceiling.
- Check every repeated proof field when a count, id, digest, path, required list,
  generated artifact, or schema enum changes.
- Rerun the current local gate when feasible. If a reviewer cannot rerun it,
  the verdict must label the evidence as supplied-only and explain the
  exception.
- Negative fixtures must pin the bug class.
- Approval must name the proof anchors checked and the claim ceiling that
  remains unsupported.

## Four-Persona Review Team Law

Material review rounds use the repo-defined four-persona team. This template
is guidance for agents; the enforceable review result is the typed review
receipt bound to the validator, review-target, and archive anchors.

Required personas:

1. Contract and Claim Falsifier.
2. Orchestration and Recovery Falsifier.
3. Security Trust-Boundary Falsifier.
4. Product and Simplicity Falsifier.

Every material round uses all four personas, fresh-context reviewers, full
scope, current validator receipt, current review-target digest, current archive
receipt when relevant, and the current claim ceiling. Later rounds are not
scoped only to previous blockers.

Spawn installed custom agent types only after the active Codex registry exposes
them. Disk cache sync and `~/.codex/agents/` TOML presence are not enough.
Material sign-off requires a typed active-registry exposure receipt. Generic
reviewers are a degraded fallback for advisory development feedback only, and
only when seeded with the exact canonical persona prompt. Generic reviewers
cannot satisfy material sign-off.

Cadence:

- Every material review round is a sign-off attempt: regenerate validator,
  review-target, archive, and registry receipts first when relevant.
- Use all four installed personas with runtime-supported model and reasoning
  configuration, recording those values only when Codex exposes them.
- Give every reviewer the full current scope, current anchors, and current
  claim ceiling.
- Use fresh reviewers every round. Do not reuse agents across rounds.
- All four must return `SIGN_OFF` in the same round. Any
  `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED` invalidates the round; repair,
  regenerate anchors, close reviewers, and start a fresh full-scope round.

## Completion And Claim Ceiling Law

Do not say complete, ready, done, fixed, passing, or production-ready without
fresh named evidence.

Completion means the exact required commands or artifacts exist now. Missing
coverage, missing deterministic checks, blocked review or approval, dirty
worktrees, absent receipts, stale generated authorities, or insufficient live
proof must be called `BLOCKED`, `REVISE_BEFORE_NEXT_PHASE`, or explicitly
unsupported.

Final packets, archives, review targets, transaction finalization, and
update-goal eligibility must fail closed on stale evidence, forged proof,
private proof paths, missing Product Success lineage, missing same-surface
proof, circular dependencies, altered claim ceilings, or packet claims added
outside required claim ids.

Production-use proof is required before production-ready claims. A mechanic or
product surface is not complete until it has produced useful output on a real,
non-toy task large enough to reveal whether the feature serves its intended
purpose at the required quality bar.

## Completion Report Contract

For non-trivial work, final reports include verification, security review,
performance review, quality review, and claim ceiling. Write `N/A` when a
section truly does not apply.
