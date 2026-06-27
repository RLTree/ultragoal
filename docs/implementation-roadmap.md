# Implementation Roadmap

This roadmap is for turning the proposal into a real plugin.

Current status: package/static/red-fixture/target-repo validator work is
implemented in the Rust `validator/` crate; the current receipt is
`validation_artifacts/ultragoal-audit/validator-receipt.json`. Remaining
roadmap work is app visibility, connector polish, and real multi-lane dogfood.
Quote the generated receipt for exact run id, package digest, fixture counts,
and generated-artifact counts; source docs intentionally avoid embedding those
moving values. Retired Python validator/reference files have been removed.

## Phase 1: Contract Artifacts

Create the canonical templates and schemas defined by `docs/agent-first-repo-shape.md` and `plugin-manifest-draft.json`.

Canonical authorable artifacts include the agent-first baseline plus:

- goal contracts;
- lane registry;
- lane ExecPlans;
- completion manifest;
- verification backlog;
- amendments ledger;
- product-cohesion receipt;
- red fixture catalog.

Generated or optional human projections:

- `examples/generated/READY_FOR_MERGE.example.json`
- `LANE_REGISTRY.md`
- `VERIFICATION_BACKLOG.md`
- `CONTRACT_AMENDMENT.md`
- `validation_artifacts/ultragoal-audit/validator-receipt.json`

Acceptance:

- Authorable JSON/JSONL examples parse against schemas using `schemas/schema-catalog.json` with no remote schema fetch.
- Generated views are marked generated-only and are not canonical authorities.
- Validator receipts are never copyable authoring templates; they are emitted by `ultragoal-audit` from schema-owned checks and the red fixture catalog.
- The template set has no required project-specific paths.

## Phase 2: Validator

Schema-only validation is insufficient. This phase implements semantic checks for claim/evidence coupling, backlog-row joins, generated ready-artifact provenance, goal-binding receipt matches, derived amendment deltas, and live-beneficial non-fixture status.

`ultragoal-validator` is implemented for the package/static/red-fixture/target-repo
fixture scope. Required check identity is owned only by
`schemas/common-defs.schema.json#/$defs/requiredValidatorCheckId`; the
validator must implement every id and fail if any required id lacks a handler.
Roadmap prose may describe categories, not restate the checklist.

Acceptance:

- All red packets are executable JSON Patch specs against a declared valid base fixture; the validator materializes each bad bundle and fails it for the intended reason.
- `fixtures/valid/minimal-goal-run.json` passes schema/static validation only and does not claim semantic validator readiness.
- Validator output is parseable and suitable for agent feedback.

## Phase 3: ultragoal Skill

Implement the skill that creates and hardens high-tier goal contracts.

Acceptance:

- It refuses low-tier smallest-slice planning.
- It produces a contract bundle.
- It binds to real goal mechanics when available.
- It records when goal mechanics are unavailable.
- It creates a claim manifest and required-claim hash.

## Phase 4: Agent-First Repo Harness Skills

Implement `harness-engineering`, `agent-first-repo-init`,
`agent-first-repo-retrofit`, `agent-runtime-legibility`, the optional `agent-observability-stack` setup skill, and the conditional `product-cohesion-gate` skill.

Acceptance:

- Fresh repos receive the full starting shape: short `AGENTS.md`,
  standards, codemap, ExecPlan law, specialized root docs, routed docs indexes,
  validation root, repo-local skill folder, and one runnable gate.
- Existing repos are audited first; current commands and blockers are recorded
  before standards are tightened.
- Runtime claims apply `agent-runtime-legibility` and emit its receipt instead
  of copying runtime-proof checklists across skills.
- Observability setup is packaged as a separate skill and installed only when requested, goal-required, or claim-required; once invoked, it emits setup/audit receipts.
- Product-cohesion setup is invoked only for consumer-facing work, but once invoked it emits a product journey receipt with UI evidence, product promise, claim ceiling, and human-attention policy.
- Fixture/static proof cannot satisfy UI, package, installed-app, or live-use
  claims.

## Phase 5: Lane And Orchestrator Skills

Implement `execplan-lane` and `orchestrator-reconciler`.

Acceptance:

- A macro-lane cannot launch without a self-contained ExecPlan.
- Ownership paths are checked for overlap.
- Worktree/state isolation requirements are explicit.
- A lane cannot be accepted without generated receipts.
- The parent cannot implement goal work unless registered as a lane.

## Phase 6: Proof Gate

Implement `proof-gate` and ready receipt generation.

Acceptance:

- It classifies skipped checks as lane-owed, root-owed, externally-blocked, or withheld-claim.
- It blocks surface substitution.
- It blocks feature claims without live beneficial e2e evidence.
- It blocks product claims without a product-cohesion receipt and UI journey evidence.
- It blocks product flows that overuse human handoff instead of preserving agent autonomy.
- It writes ready receipts only from validator output.

## Phase 7: Review Agents And Custom Agents

Implement or package prompts for every manifest agent: Contract and Claim Falsifier, Orchestration and Recovery Falsifier, Security Trust-Boundary Falsifier, Product and Simplicity Falsifier, plugin scout, and standards extractor. Package app-visible Codex custom-agent TOML files under `custom-agents/` and install them to `~/.codex/agents/` for personal use.

Acceptance:

- The four-persona review loop runs until all four reviewers report no material revision in the same current-signoff round.
- Each round records findings, dispositions, and revised files.
- The simplicity auditor may simplify wording but cannot weaken obligations.

## Phase 8: Local Plugin Packaging And Install

Package the plugin for local marketplace installation.

Acceptance:

- `.codex-plugin/plugin.json` points at `./skills/`.
- `install/personal-marketplace.example.json` matches the Codex personal marketplace shape.
- Local installation copies the plugin under `~/.codex/plugins/harness-ultragoal` and exposes it through `~/.agents/plugins/marketplace.json`.
- Custom agents install under `~/.codex/agents/`.
- A post-copy smoke check validates the installed copy's manifest and custom-agent TOML syntax.

## Phase 9: Optional Connectors

Add adapters only after the file-contract path works.

Candidate adapters:

- Codex goal tools;
- Codex app thread tools;
- automation tools;
- GitHub/Linear;
- Proof;
- Browser/Computer Use;
- OpenAI Developers;
- Codex Security.

Acceptance:

- Connector absence cannot make core validation impossible unless the claim explicitly depends on that connector.
- Connector data is parsed into typed records before use.
- Connector failures degrade claim ceiling honestly.

## Phase 10: Dogfood

Run the plugin on a real multi-lane goal in a separate repo.

Acceptance:

- At least two macro-lanes run in isolated workspaces.
- At least one lane is sent back for lane-owed proof and then closes.
- At least one root-owed item is tracked and later verified.
- A final completion manifest has only supported claims.
- Stale worktrees and sessions are cleaned.
- External review finds no material false-completion loophole.

## Inventory Closure

Implementation must validate `plugin-manifest-draft.json` against `schemas/plugin-manifest.schema.json`; `plugin-inventory-closure` fails if any package file is absent from exactly one inventory bucket or if any manifest skill, agent, schema, fixture, template, generated example, source card, or resource path is missing. `ultragoal-audit` must also run a cross-bucket `plugin-inventory-exactly-once` semantic check because JSON Schema cannot prove membership across all inventory buckets. Source cards must be refreshed or explicitly marked not refreshed during implementation review; proposal snapshots are not live-source proof.
