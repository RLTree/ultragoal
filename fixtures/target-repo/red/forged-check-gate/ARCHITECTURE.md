# ARCHITECTURE

This is the repo codemap. It should help a new agent answer "where is the
thing that does X?" without reading every file. Keep it short, current, and
structured by responsibility.

## Bird's-Eye View

Describe what this repo produces and how a user, service, CLI, package, or
workflow consumes it. Name the durable artifacts and runtime surfaces.

## Layer Boundaries

List the main layers in dependency order. For each layer, state what it owns,
what it may depend on, and what must not depend on it.

## Codemap

Use a small tree or prose map of the important paths. Prefer domain names over
implementation accidents.

```text
src/ or app/          primary implementation
tests/ or e2e/        executable behavior checks
docs/                 design, plans, specs, references, generated maps
scripts/check         repo health gate and focused subcommands
validation_artifacts/ receipts, generated proof, run evidence
```

## Agent-First Information Architecture

Root files are the first-stop interface for agents:

- `AGENTS.md` routes the task and stays short.
- `AGENT_STANDARDS.md` is the operating law.
- `ARCHITECTURE.md` is this codemap.
- `PLANS.md` defines the ExecPlan format.
- `DESIGN.md`, `FRONTEND.md`, `PRODUCT_SENSE.md`, `QUALITY_SCORE.md`,
  `RELIABILITY.md`, and `SECURITY.md` hold specialized policy.
- `docs/design-docs/`, `docs/product-specs/`, `docs/generated/`, and
  `docs/references/` hold deeper routed material.

## Invariants Stated As Absences

State what must not happen. Examples:

- No product code bypasses the typed boundary for external input.
- No service widens a local-only bind address without approval and tests.
- No run ledger, receipt, or proof artifact is rewritten after closeout.
- No UI/product claim ships without matching journey evidence.

## Where To Change Things

For each recurring change type, name the owning path, check, and proof surface.
When a new path or rule appears repeatedly, update this codemap and add a check
or fixture if the rule matters mechanically.
