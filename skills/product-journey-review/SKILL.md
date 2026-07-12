---
name: product-journey-review
description: "Falsify Harness Ultragoal journeys from independent operator perspectives. Use for fresh-user, fresh-agent, maintainer, security, recovery, usability, acquisition-through-completion, wrong-surface, stale-proof, or quality-in-use review."
---

# Product Journey Review

Review one named journey independently and read-only. Attack whether the
intended operator can enter, complete, diagnose, recover, and understand the
outcome on the current candidate.

## Inputs

Require a journey identifier, operator perspective, supported host and
environment, current candidate identity, entry and discovery surface, expected
outcome, allowed effects, raw runtime observations, prior findings, and exact
claim boundary. Implementation prose is context, not observed behavior.

## Select the reviewer

Use the current project-scoped read-only role that matches the attack:

- `.codex/agents/product-journey-reviewer.toml` for operator comprehension and
  acquisition-through-completion.
- `.codex/agents/security-reviewer.toml` for effects, confinement, secrets, and
  supply-chain boundaries.
- `.codex/agents/orchestration-recovery-reviewer.toml` for interruption,
  recovery, and reconciliation.
- `.codex/agents/claim-falsifier.toml` for stale, wrong-surface, bypassed, or
  reward-hacked proof.

If the current host cannot discover the named role, report discovery as
unsupported and lower the review ceiling. Do not copy agent files globally or
replace the reviewer with the implementer.

## Review from a fresh boundary

1. Start as a fresh user, agent, maintainer, security reviewer, or recovery
   operator without hidden implementer context.
2. Probe every required command, plugin, agent, and host capability. Missing or
   incompatible surfaces are blockers, not invitations to simulate output.
3. Observe entry, route selection, authority disclosure, task execution,
   diagnosis, next action, interruption, recovery, and final comprehension.
4. Attack stale candidate identity, wrong truth surface, hidden writes,
   permission escalation, secret exposure, symlink or race escape, dirty-tree
   damage, misleading success, reward hacking, and unrecoverable partial work.
5. Exercise the named journey's positive case and at least one causal failure or
   false-pass control. For fit, include fresh, partial, and conflicting
   authority; for routine work, include dirty and repeat-use state; for goal
   execution, include interruption and recovery. For plugin lifecycle, include
   fresh install, monotonic update, failed-update recovery, authorized
   rollback, idempotent reinstall, uninstall and teardown, stale-cache
   recovery, and repeat use.
6. Distinguish observed fact, inference, recommendation, and unsupported
   surface. Return findings to the root; do not fix or decide acceptance.

For a Product Fitness disposition, bind exactly accessibility, cognitive load,
recovery burden, continuance, and real-use evidence to the same candidate.
Keep the reviewer falsification-only: it may lower a ceiling but cannot raise
one. Reject documentation, fixture success, package publication, installation,
smoke tests, receipts, or reviewer agreement as substitutes for the named
real-use behavior.

The reviewer remains read-only with zero hidden writes. When a journey requires
mutation, observe a separately authorized executor and verify its declared
effect. Screenshots, logs, receipts, and tests support observations but do not
replace the real journey.

## Output

Report environment and candidate, observed steps, operator comprehension, task
result, causal failures, security and recovery findings, false-pass results,
exact repair and rerun, unsupported surfaces, and an independent ceiling
recommendation.

Never claim installation, discovery, runtime behavior, journey acceptance,
readiness, release, or completion from this skill alone.
