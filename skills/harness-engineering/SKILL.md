---
name: harness-engineering
description: Use when turning a fresh or existing repo into an agent-first codebase with durable guidance, enforceable gates, proof artifacts, and reusable goal-run contracts.
---

# Harness Engineering

Use this as the umbrella skill for high-trust agent repo setup. It routes to
the right sub-skill and keeps the acceptance bar from collapsing into a doc
dump.

Read `../../docs/plugin-resource-map.md` when present. It is the plugin-level map
for how skills, agents, templates, validators, and receipts compose.

## Route

- Fresh repo or new workspace: use `agent-first-repo-init`.
- Existing repo or migration: use `agent-first-repo-retrofit`.
- Agentic app, workflow engine, UI, service, or local runtime: also use
  `agent-runtime-legibility`.
- Consumer-facing product, UI, dashboard, launcher, or control surface: also use
  `product-cohesion-gate`.
- Large goal run: also use `ultragoal`, `execplan-lane`,
  `orchestrator-reconciler`, and `proof-gate`.
- Drift or recurring review feedback: use `standards-gardener`.

## Non-Negotiable Shape

A harnessed repo has durable guidance, mechanical enforcement, and proof
surfaces that a fresh agent can discover without chat history. Apply
`../../docs/agent-first-repo-shape.md` as the shape authority. Runtime claims apply
`agent-runtime-legibility`; do not duplicate that receipt contract here.
The standards enforcement surface is part of the harness: fitted repos create
or verify `agent-standards/enforcement.json`, `agent-standards/enforcement.tsv`,
`agent-standards/enforcement-audit.tsv`, and `scripts/check-agent-standards`.

## Core Laws

- Human steers; agents execute against written contracts and receipts.
- Repo files are the system of record; memory and chat are context only.
- Names, directories, command flags, logs, and receipts are agent interfaces.
- Parse external inputs into typed records at boundaries; do not rely on
  scattered validation prose.
- Mechanical gates replace repeated reminders.
- Typed standards rows outrank reviewer memory. Reviewer agreement cannot close
  a standards obligation unless the row is mechanized, blocked, or backlogged
  with affected claims withheld.
- Runtime, UI, API, static, fixture, and live-use proofs are distinct surfaces.
- Product cohesion is a proof surface for consumer-facing work, not a cosmetic
  afterthought.
- Plugin cohesion is also a product surface: agents should be able to choose
  the next skill, gate, or receipt without the user manually sequencing the
  plugin.
- Claims must name exact evidence, current commit/worktree, and claim ceiling.
- Worktrees or equivalent isolated environments are required for parallel
  implementation.

## Acceptance

Do not claim the harness exists until:

- a fresh agent can find the law from `AGENTS.md`;
- at least one real gate runs and produces named output;
- the repo separates reusable law from project facts;
- always-loaded standards route to semantically named modules instead of a
  mega-document;
- current blockers and unmechanized standards are recorded;
- the standards enforcement checker has run or its blocker is claim-limited;
- install/setup instructions are runnable from a clean checkout;
- a reviewer can trace every completion claim to a file, command, or receipt.

## Observability Routing

When a target repo is runnable, apply `agent-runtime-legibility` by default. Invoke `agent-observability-stack` only when the user requests observability setup, the goal contract requires it, or a runtime/performance/UI claim cites logs, metrics, traces, spans, or agent context summaries. Once invoked, the stack must be installed throughout the repo and verified by its gate marker or an exact blocker.

## Product Cohesion Routing

Invoke `product-cohesion-gate` when a goal changes how a user starts,
understands, monitors, trusts, or completes work through a product surface.
The gate is conditional, but once invoked it must produce a product journey
receipt with UI evidence, runtime truth, claim ceiling, and human-attention
policy. Engine proof alone cannot close a consumer-facing product claim.
