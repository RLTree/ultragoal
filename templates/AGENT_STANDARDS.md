# Agent Standards

This file is the compact operating-law router for agent-first repositories.
`AGENTS.md` points here first; this file points to the detailed standards
modules. Do not turn this router into the whole manual.

Detailed standards live in `agent-standards/`.

## Always Load

For non-trivial work, load:

1. The module or modules matching the task.

If a module is missing, stale, or contradicted by repo-specific instructions,
record the gap before acting.

## Module Routing

| Task shape | Load |
| --- | --- |
| File names, directories, codemap, routing, context budget | `agent-standards/01-namespace-and-progressive-disclosure.md` |
| Tests, parsing, validators, mechanical checks, feedback loops | `agent-standards/02-boundaries-validation-and-enforcement.md` |
| ExecPlans, macro-lanes, worktrees, parent orchestration | `agent-standards/03-execplans-worktrees-and-orchestration.md` |
| Security, reliability, product surfaces, human attention | `agent-standards/04-security-reliability-and-product-cohesion.md` |
| Review teams, proof, claim ceilings, completion reports | `agent-standards/05-review-and-completion.md` |
| Recurring friction, standards gardening, self-improvement | `agent-standards/06-standards-gardening.md` |

## Non-Negotiable Entry Rules

- The repo is the source of truth. Chat and memory are context only.
- Preserve user changes and isolate concurrent work.
- Parse external inputs at boundaries before acting on them.
- Bind proof to fresh operation identity and artifact digests; metadata alone
  is not proof.
- State transitions for queues, approvals, dependency release, and closure must
  be explicit and forward-safe.
- Every active repo file needs a current operational purpose. If the purpose
  cannot be justified, remove the file instead of archiving it in the repo.
- Use self-contained ExecPlans for long-running or multi-lane work.
- Codex app worktree threads are preferred owners for substantial macro-lanes
  that need visibility, resumability, or handoff.
- Documentation freshness is a completion obligation. Load
  `agent-standards/01-namespace-and-progressive-disclosure.md` when work may
  affect repo-owned docs or generated docs.
- Coverage proof is a completion obligation when a claim cites coverage, test
  completeness, readiness, or production readiness. Load
  `agent-standards/02-boundaries-validation-and-enforcement.md`.
- Future Codex app worktree lane owners default to `gpt-5.5` with `low`
  reasoning unless a lane contract justifies higher reasoning. Material
  reviewers still use `gpt-5.5` with `high`.
- Branch first, worktree second. A missing branch ref is an orchestration
  failure.
- Product-surface claims require Product Cohesion proof; engine proof alone is
  not enough.
- Material review uses the four merged canonical personas with the required model,
  reasoning, full-scope, fresh-context cadence.
- Do not claim done, ready, fixed, passing, complete, or production-ready
  without fresh named evidence and an honest claim ceiling.
- Repeated friction becomes the smallest durable improvement: check, fixture,
  scrubber, quality receipt, skill update, persona update, routed standard,
  resource-map update, hook, or backlog row.

## Review Team Reminder

The canonical material review team is:

1. Contract and Claim Falsifier.
2. Orchestration and Recovery Falsifier.
3. Security Trust-Boundary Falsifier.
4. Product and Simplicity Falsifier.

Every material review round is a sign-off attempt using all four canonical
personas with `gpt-5.5`, `high`, full current scope, fresh reviewers, current
anchors, and the current claim ceiling. All four must return `SIGN_OFF` in the
same round. Any `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED` invalidates the round;
repair, regenerate anchors, close reviewers, and start a fresh full-scope
round.

Before launching reviewers, run the Material Review Scope Gate. Delta-only or
advisory review is allowed only for non-signoff follow-up or deterministic
validator deltas and cannot satisfy material `SIGN_OFF`. If deterministic
preflight blocks, repair the validator/receipt/package problem before spending
reviewer tokens.

## Completion Report

For non-trivial work, report:

- verification command, artifact, receipt, runtime proof, or explicit gap;
- security review or `N/A`;
- performance review or `N/A`;
- quality review and residual gaps;
- claim ceiling: supported, unsupported, and blocked.
