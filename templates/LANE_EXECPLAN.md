# Lane ExecPlan: <title>

This ExecPlan is a living document. Keep Progress, Surprises & Discoveries, Decision Log, and Outcomes & Retrospective current.

## Purpose / Big Picture

Describe the user-visible or project-visible outcome and how to see it working.

## Progress

- [ ] ...

## Surprises & Discoveries

- Observation:
  Evidence:

## Decision Log

- Decision:
  Rationale:
  Date/Author:

## Outcomes & Retrospective

Record achieved outcomes, gaps, and lessons.

## Context and Orientation

Describe current state, key files, and terms.

## Ownership And Isolation

- Lane:
- Owner:
- Role:
- Owner thread type: Codex app worktree thread is preferred for substantial
  macro-lanes.
- Launch prompt or prompt artifact:
- Branch:
- Worktree/workspace:
- State roots:
- Scratch roots:
- Tool cache roots:
- Artifact root:
- Ports:
- Browser/profile roots if applicable:
- Owned paths:
- Forbidden/shared paths:
- Overlap locks, if any:

## Dependencies

List every upstream claim consumed by this lane. Each dependency must include upstream lane id, claim id, required status, validated status, evidence digest, upstream commit, ready receipt, and validation time. Do not consume lane output from prose summaries.

If this lane is blocked by another lane, state the exact condition that lifts
the blocker. After the dependency lands, root verification passes, and no other
blockers remain, the orchestrator should launch or resume this lane without
waiting for another user prompt.

## Plan of Work

Describe the sequence of changes.

## Concrete Steps

List exact commands with working directories and expected observations.

## Validation and Acceptance

Include exact commands, cwd, expected exit, and artifact paths for:

- focused tests;
- integration/root checks if applicable;
- live beneficial end-to-end task with real input path, output artifact path, and inspection criteria;
- negative tests or red fixtures;
- performance/security/accessibility/runtime proof when applicable.

## Launch Readiness

- Model/reasoning class and risk justification:
  - Future Codex app worktree lane owner default: `gpt-5.5`, `low`.
  - Higher lane-owner reasoning requires a named risk and expected payoff.
  - Reviewer reasoning is separate and remains `high`.
- Review cadence:
  - Material sign-off: fresh validator/review-target/archive/registry anchors
    when relevant, four personas, `gpt-5.5`, `high`, full scope, fresh
    reviewers.
  - Any `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED` invalidates the round; repair,
    regenerate anchors, close reviewers, and launch a fresh full-scope round.
- Parent monitoring mode and cadence:
- Goal-binding status and receipt path, or unavailable probe path:
- Lane registry row path:
- Ready receipt destination:
- Teardown condition:

## Idempotence and Recovery

Explain retry, resume, rollback, and cleanup.

## Required Receipts

- ready receipt:
- validator receipt:
- evidence artifacts:

## Parent-Thread Ready Message

The lane is not ready for parent merge or teardown until it sends a final
parent message containing:

- lane id, branch, worktree, and current commit;
- changed owned paths and confirmation that forbidden/shared paths were not
  modified;
- commands run with exit codes and artifact paths;
- ready receipt path, validator receipt path when relevant, and claim ceiling;
- current worktree status, including preserved uncommitted paths if any;
- blockers, withheld claims, dependency changes, and next recommended parent
  action.

## Claim Ceiling

The claim ceiling is typed. Free-text claims are commentary only. Keep this block aligned with `schemas/schema-authority-primitives.schema.json#/$defs/claimCeilingEntry`.

```json
[
  {
    "claim_id": "CLAIM-001",
    "status": "contract_only",
    "claim_surface": "static",
    "effect": "blocked",
    "evidence_ids": ["EV-001"],
    "source_manifest_path": "COMPLETION_MANIFEST.json"
  }
]
```

This lane cannot claim completion, production readiness, live behavior, UI behavior, installed behavior, or root integration unless the corresponding typed entry is included and supported by evidence.
