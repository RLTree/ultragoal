---
name: agent-first-repo-init
description: Use when initializing a fresh repo or workspace so agents can work from durable guidance, mechanical gates, codemaps, proof roots, and fast feedback from the first commit.
---

# Agent-First Repo Init

Use this for a fresh repository or new workspace. The goal is not a larger
README. The goal is an environment where agents can discover the contract,
make changes safely, and prove claims without chat history.

This skill is subordinate to `harness-ultragoal:fit-repo` for Harness
Ultragoal setup. A fresh repo is not initialized, harnessed, ultragoal-ready,
or product-ready until `fit-repo` emits a schema-valid receipt or records a
withheld-claim blocker naming the missing setup surface.

## Required Starting Shape

Apply the baseline in `../../docs/agent-first-repo-shape.md`. That file is the
single repo-shape authority; do not copy a separate baseline into this skill.

For runnable products or user-facing feature claims, apply
`agent-runtime-legibility` and emit its receipt instead of copying a separate
runtime checklist into this skill.

## Init Procedure

1. Identify the product type, primary workflows, and proof surfaces.
2. Write a short `AGENTS.md` that routes to deeper docs and names the gate.
3. Write compact `AGENT_STANDARDS.md` as a router and install the
   `agent-standards/` modules from the template. Do not create a mega
   standards document.
4. Install the standards enforcement surface: `agent-standards/enforcement.json`,
   `agent-standards/enforcement.tsv`, `agent-standards/enforcement-audit.tsv`,
   and `scripts/check-agent-standards` from the plugin templates.
5. Add `scripts/check-agent-standards` to the fast/local gate, or record the
   exact blocker in `VERIFICATION_BACKLOG.json` and the claim ceiling.
6. Write `ARCHITECTURE.md` as a codemap, not an essay.
7. Install `PLANS.md` as stable ExecPlan law. Adapt only repo terminology or
   path conventions; do not rewrite it as active project state. Project status,
   worker/thread ids, phase progress, backlog items, receipt state, and
   completion claims belong in active ExecPlans, `LANE_REGISTRY.json`,
   `VERIFICATION_BACKLOG.json`, `COMPLETION_MANIFEST.json`, receipts, or
   `AMENDMENTS.jsonl`.
8. Add `DESIGN.md`, `FRONTEND.md`, `PRODUCT_SENSE.md`, `QUALITY_SCORE.md`,
   `RELIABILITY.md`, and `SECURITY.md` as routed policy surfaces. If a surface
   is absent, say so in the file and name the trigger that would activate it.
9. Add `docs/design-docs/`, `docs/exec-plans/`, `docs/generated/`,
   `docs/product-specs/`, and `docs/references/` with index files and the
   tech-debt tracker.
10. Add `scripts/check` with focused subcommands and actionable failures.
11. Add `.codex/setup-worktree-env.sh`,
   `.codex/environments/environment.toml`, and `.gitignore` so Codex
   App-created worktrees get isolated state under `.codex-worktree/`.
12. Mirror `.codex/setup-worktree-env.sh` to
    `~/.codex/bin/codex-worktree-env` when installing locally.
13. Add templates for receipts, verification backlog, completion manifest, and
    plugin/skill routing references when applicable.
14. If product-impacting surfaces exist, install `PRODUCT_FITNESS.md`,
    `PRODUCT_FITNESS_RECEIPT.json`, the Product Fitness standards row, and
    Product Fitness receipt path. If Product Fitness cannot be proved, record a
    blocker and withhold product readiness, release, daily-driver, and material
    product sign-off claims.
15. Expose the Ultragoal orchestrator automation template from
    `.codex/automations/ultragoal-orchestrator/` when the repo will run
    multi-lane Ultragoals. It is installed only after active goal setup and
    first-wave lane launch.
16. Emit or refresh the fit-repo receipt. If the receipt cannot be emitted,
    record the blocker and withhold initialized, harnessed, ultragoal-ready,
    product-ready, release, and material sign-off claims.
17. Run the gate once and record the exact output or blocker.

## Minimum Standards

- Keep law in files. Do not rely on kickoff prompts.
- Keep always-loaded law short. Route detail through semantically named
  standards modules and specialized docs.
- Prefer small named modules and agent-readable paths.
- Every active repo file must justify a current operational purpose. Remove
  purposeless or archive-only files instead of creating in-repo archives.
- Parse once at boundaries into typed records where the language supports it.
- Put repeated feedback into standards or validators.
- Standards debt is not closed by reviewer agreement. It is closed only by a
  mechanized row or by a blocked/backlogged row that withholds affected claims.
- Make the fastest useful check cheap enough to run often.
- Separate fixture, static, runtime, UI, and live-use proof.
- Source `.codex-worktree/env.sh` before validation commands when running in a
  Codex App worktree.
- Use Codex app worktree threads as the preferred owners for substantial
  ExecPlan macro-lanes.
- Launch future Codex app worktree lane owners with `gpt-5.5` and `low`
  reasoning by default; material reviewers still use `gpt-5.5` with `high`.

## Acceptance

Accepted only when a new agent can answer:

- Where is the law?
- Which standards module applies to the current task?
- What command proves the repo is healthy?
- Where are long plans tracked?
- Where are design, product, reference, generated, security, and reliability
  docs routed?
- Where are receipts stored?
- Where is the standards enforcement ledger and when did it last audit?
- What claims are not yet provable?
- How does the app or package run locally?
- How does a Codex App-created worktree get isolated scratch, temp, Cargo
  target, and port state?
