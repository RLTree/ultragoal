# AGENTS.md

This file is the compact routing entrypoint for agents. It should stay short. Do not turn it into the whole manual.

Read in this order for non-trivial work:

1. `AGENT_STANDARDS.md`
2. The routed `agent-standards/` module for the task
3. `ARCHITECTURE.md`
4. `PLANS.md`
5. The specialized root doc for the task:
   `SECURITY.md`, `RELIABILITY.md`, `PRODUCT_SENSE.md`, `DESIGN.md`,
   `PRODUCT_FITNESS.md`, `FRONTEND.md`, or `QUALITY_SCORE.md`
6. `docs/exec-plans/active/`
7. `docs/design-docs/`, `docs/product-specs/`, `docs/generated/`, or
   `docs/references/` when routed there
8. `validation_artifacts/`

Hard rules:

- Follow the repo standards before editing.
- Use ExecPlans for long-running or multi-lane work.
- Preserve user changes.
- Use isolated workspaces for concurrent lanes.
- Prefer Codex app worktree threads for substantial ExecPlan macro-lanes.
- Do not claim completion without named evidence.
- Documentation freshness is part of completion. When work changes
  architecture, commands, standards, runtime behavior, product behavior, proof
  surfaces, lane state, operational procedure, generated-doc freshness,
  validation receipts, backlog rows, or tech-debt records, update every affected
  repo-owned doc or record a named stale-doc blocker before making completion
  claims. Do not edit every doc every time; no affected doc may be stale without
  owner, reason, required follow-up, and claim-ceiling impact.
  Check routed surfaces such as `ARCHITECTURE.md`, `PLANS.md`, specialized
  root docs, active ExecPlans, `docs/**`, and `agent-standards/**`.
- Test pass counts, smoke tests, fixture tests, mocks, and reviewer signoff are
  not coverage proof. Coverage claims require a coverage receipt or an explicit
  blocker/ratchet floor.
- Treat memory and chat as context, not current proof.
- Keep project-specific facts in repo-local docs.
- For user-facing products or control surfaces, use `PRODUCT_SENSE.md` and
  Product Cohesion evidence before claiming the product makes sense.
- For product success, repeated-use, daily-driver, release, readiness, or
  quality-in-use claims, use `PRODUCT_FITNESS.md` and a valid Product Fitness
  receipt. Install success, smoke tests, feature delivery, reviewer agreement,
  and first use are not product success proof.

Run the repo's check entrypoint before completion claims.
