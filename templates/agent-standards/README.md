# Agent Standards Modules

`AGENT_STANDARDS.md` is the compact router. These files contain the detailed
operating laws. Load the modules that match the task instead of stuffing every
rule into the prompt.

The enforcement surface is not optional:

- `enforcement.json` is the typed standards obligation ledger.
- `enforcement.tsv` is the agent-readable projection of the same rows.
- `enforcement-audit.tsv` records current audit evidence.
- `scripts/check-agent-standards` must run from the repo root before material
  completion claims, or the blocker must be recorded with affected claims
  withheld.

Reviewer agreement cannot close a standards obligation by itself. A row is
closed only when mechanized, or when blocked/backlogged with an explicit repair
action and claim-ceiling impact.

| Task shape | Load |
| --- | --- |
| File names, directories, modules, functions, symbols, ids, artifact paths, codemap, routing, context budget | `01-namespace-and-progressive-disclosure.md` |
| Tests, parsing, validators, mechanical checks, feedback loops | `02-boundaries-validation-and-enforcement.md` |
| ExecPlans, macro-lanes, worktrees, parent orchestration | `03-execplans-worktrees-and-orchestration.md` |
| Security, reliability, product surfaces, human attention | `04-security-reliability-and-product-cohesion.md` |
| Review teams, proof, claim ceilings, completion reports | `05-review-and-completion.md` |
| Recurring friction, standards gardening, self-improvement | `06-standards-gardening.md` |
| CLI authority, receipts, coverage authority, source/install/cache/app proof separation | `07-cli-authority-and-proof-surfaces.md` |
| Observability, current state, next action, repair loops, telemetry, evals | `08-observability-and-repair-loop.md` |
| Product Success Contract, Product Fitness, Product Cohesion, quality-in-use | `09-product-success-and-quality-in-use.md` |
| Plugin activation, retrofit, package inventory, distribution surfaces, clean-room proof | `10-plugin-activation-and-distribution-surfaces.md` |
| Research sources, improvement loop, quality gates, capability gaps, authority graph | `11-research-improvement-and-quality-gates.md` |
| Tool contracts, risk tiers, Rust/toolchain substrate, dependency and privacy boundaries | `12-tool-risk-and-runtime-substrates.md` |
