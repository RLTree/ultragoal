# AGENTS.md

This file is the compact routing entrypoint for agents. It should stay short. Do not turn it into the whole manual.

Read in this order for non-trivial work:

1. `AGENT_STANDARDS.md`
2. `ARCHITECTURE.md`
3. `PLANS.md`
4. The specialized root doc for the task:
   `SECURITY.md`, `RELIABILITY.md`, `PRODUCT_SENSE.md`, `DESIGN.md`,
   `FRONTEND.md`, or `QUALITY_SCORE.md`
5. `docs/exec-plans/active/`
6. `docs/design-docs/`, `docs/product-specs/`, `docs/generated/`, or
   `docs/references/` when routed there
7. `validation_artifacts/`

Hard rules:

- Follow the repo standards before editing.
- Use ExecPlans for long-running or multi-lane work.
- Preserve user changes.
- Use isolated workspaces for concurrent lanes.
- Do not claim completion without named evidence.
- Treat memory and chat as context, not current proof.
- Keep project-specific facts in repo-local docs.
- For user-facing products or control surfaces, use `PRODUCT_SENSE.md` and
  Product Cohesion evidence before claiming the product makes sense.

Run the repo's check entrypoint before completion claims.
