# AGENTS.md

This file is the compact routing entrypoint for agents. Keep it short.

Read in this order for non-trivial work:

1. `AGENT_STANDARDS.md`
2. The routed `agent-standards/` module for the task
3. `ARCHITECTURE.md`
4. `GOAL_CONTRACT.md`
5. `PLANS.md`
6. The specialized root doc for the task:
   `SECURITY.md`, `RELIABILITY.md`, `PRODUCT_SENSE.md`, `DESIGN.md`,
   `PRODUCT_FITNESS.md`, `FRONTEND.md`, or `QUALITY_SCORE.md`
7. `docs/exec-plans/active/`
8. Other routed `docs/` material
9. `validation_artifacts/` only when a current claim names an artifact there

Hard rules:

- Follow the current goal contract and its single active ExecPlan.
- Historical v2 plans, lane registries, backlogs, completion manifests,
  receipts, and mandatory-law projections are frozen compatibility inputs.
  Do not refresh them or use their staleness to gate ordinary delivery.
- Preserve user changes and isolate concurrent work.
- Use ExecPlans for long-running or multi-lane work.
- Use exclusive ownership for parallel lanes and one root fan-in.
- Test changed behavior and relevant failure paths. Make coverage claims only
  from current coverage measurement; ordinary work does not create a durable
  coverage artifact by default.
- Persist proof only for a current claim, irreproducible observation,
  cross-process custody, recovery need, or Tree-authorized release decision.
- Invalidate proof only when a declared relevant dependency changes or
  contradictory same-surface evidence appears.
- Keep project-specific facts in repo-local docs.
- Product claims require same-surface Product Fitness and Product Cohesion
  evidence at the tier named by the current contract. Source checks, install
  success, smoke tests, receipts, and reviewer agreement are not substitutes.
- Do not claim completion without fresh named evidence and an honest ceiling.

Run the repository check entrypoint before completion claims. A failing legacy
or generated projection blocks only a claim that consumes it; record the exact
gap rather than launching a receipt-refresh loop.
