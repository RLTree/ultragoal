# Standards, Orchestration, And Product Cohesion Ultragoal

## Purpose

Upgrade the Harness Ultragoal Plugin Proposal so fitted repositories receive
clearer operating standards, stronger orchestration mechanics, correct review
team usage, and more cohesive plugin-skill routing without recreating the
mega-document anti-pattern the foundation articles warn against.

This is a binding goal contract. It supersedes chat-only intent for this work
until completed, amended, or explicitly abandoned.

## Problem Statement

- Operating standards are useful but too concentrated in `AGENT_STANDARDS.md`.
- Review teams can drift from required personas, models, reasoning, and
  same-anchor review cadence.
- Parent sessions can act like passive status collectors instead of true
  orchestrators.
- Lane agents lack a precise parent-thread completion handoff contract.
- Plugin skills can feel like a loose collection instead of a cohesive product.
- Recurring friction is not yet systematically promoted into surgical template,
  skill, agent, or validator improvements.

## Source Grounding

- Harness Engineering: short routers, repo as system of record, mechanical
  invariants, agent-legible environments, and cleanup of recurring entropy.
- Symphony: every open task should have an agent in its own workspace, with
  durable workflow contracts and prose summaries treated as observability.
- ExecPlans: plans must be self-contained, restartable, current, and tied to
  demonstrably working behavior.
- Agent-native architecture: features live in prompts and tools, explicit
  completion signals beat heuristic detection, and products improve through
  observed usage patterns.

## Non-Negotiable Outcomes

1. `AGENT_STANDARDS.md` becomes a compact router into semantically named
   standards modules.
2. Initialization and retrofit skills install the routed standards structure,
   not a monolithic law file.
3. Lane contracts specify review persona team, model, reasoning, parent message
   protocol, ready package, and handoff expectations.
4. Orchestrator guidance requires branch-first worktree creation, app-visible
   macro-lane owners, dependency frontier advancement, merge/teardown, and
   current proof anchors.
5. Plugin skills expose a clear resource map: when to use each skill, script,
   template, agent, receipt, and validation path.
6. Standards gardening defines when friction becomes a plugin improvement and
   when it should stay as a one-off.
7. Deterministic validation passes after the changes.

## Review Requirement

This work requires a material sign-off review using all four current falsifier
personas, full scope, fresh reviewers, and detached
validator/review-target/archive/registry anchors. Use only model and reasoning
configuration supported by the current runtime, and record those values only
when exposed. A `REVISE_BEFORE_NEXT_PHASE`
or `BLOCKED` verdict closes the round; repairs require fresh anchors and a
fresh full-scope round.

## Claim Ceiling

Supported only after validation:

- package/static/template changes;
- skill/resource routing improvements;
- validator acceptance of the updated package inventory.

Unsupported until future phases:

- local install/app visibility for the changed package;
- real target-repo adoption;
- real multi-lane dogfood proving the new orchestration flow in practice;
- production effectiveness beyond static/package review.

## Completion Standard

Completion requires:

- current validator receipt regenerated;
- package digest refreshed;
- review-target and archive receipts refreshed if the package is handed off or
  installed next;
- clean four-persona sign-off review disposition recorded;
- final report naming verification, residual risks, and unsupported claims.

## Disagreement Register

### D1: Hooks Are Not The Default Fix

Hooks are valuable only when the trigger is cheap, deterministic, low-context,
and prevents a frequent or severe error at the moment it happens. Always-on
hooks can recreate the context-bloat problem this standards split is designed
to avoid. This is not blocking; it is implemented through the standards
gardening promotion ladder.

### D2: Do Not Turn Semantic Judgment Into Fake Linters

Deterministic linters should enforce deterministic invariants. Semantic
obligations such as good orchestration or product cohesion need a mix of routed
standards, skill contracts, structured receipts, review prompts, and targeted
fixtures. This is not blocking; it is implemented by separating mechanical
checks from semantic standards and review obligations.

## Review Findings And Repairs

- Simplicity review found that requiring `agent-standards/README.md` in the
  always-load path added a ceremonial hop. The router now sends agents directly
  to the matching standards module; the README remains only a directory index.
- Simplicity review found that the temporary plan duplicated this active
  contract. Its disagreement register was folded into this file before the
  temporary plan was removed.
