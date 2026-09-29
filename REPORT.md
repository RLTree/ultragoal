# Superseded Harness Ultragoal Plugin Proposal

> This report is historical rationale, not current product or planning
> authority. The current goal is `GOAL_CONTRACT.md`; the only active plan is
> `docs/exec-plans/active/usable-product-milestone.md`. Historical lane,
> receipt, and completion mechanics below must not be refreshed or used to gate
> ordinary work.

## Executive Summary

The proposed plugin packages the practices that made the `codex-workflow-rs` goal run unusually effective: real goal mechanics, self-contained ExecPlans, macro-lane delegation, worktree isolation, parent-session orchestration, mechanical repo law, claim ceilings, root verification backlogs, live beneficial end-to-end proof, and adversarial review loops that continue until no material gaps remain.

The core product is not "better prompting." It is a harness for turning goal work into durable, inspectable, restartable, and mechanically auditable operations.

The plugin should ship as a set of skills, custom agents, templates, schemas, and optional connectors:

- `ultragoal`: high-tier goal contract hardening and runtime goal binding.
- `harness-engineering`: umbrella router for agent-first repo setup.
- `agent-first-repo-init`: fresh repo/workspace setup into the full agent-first baseline.
- `agent-first-repo-retrofit`: existing repo conversion without erasing current truth.
- `agent-runtime-legibility`: runtime/UI/CLI/service proof surfaces for agent-readable e2e validation.
- `agent-observability-stack`: separate opt-in setup for article-inspired logs, metrics, traces, run events, and agent context summaries when requested or claim-required.
- `product-cohesion-gate`: conditional product/UX gate for consumer-facing changes, tying user journey, positioning, UI proof, engine truth, claim ceiling, and human-attention policy together.
- `execplan-lane`: self-contained macro-lane ExecPlan authoring and launch checks.
- `orchestrator-reconciler`: parent-session role discipline, lane registry, merge/reconciliation, and stale state cleanup.
- `proof-gate`: claim-to-evidence validation, verification backlog handling, and false-completion rejection.
- `standards-gardener`: recurring entropy cleanup and standards drift reduction.

The plugin should avoid turning every task into a ceremony. It is the top-tier mode for serious goal runs, not a lightweight planning helper.

## What Made The Goal Run Work

The successful run had several properties worth preserving:

1. Strong contracts were created in files, not left in chat.
2. The contracts were operationalized into checkable obligations.
3. Work was delegated as macro-lanes with ownership, not micro-tasks.
4. The parent thread stayed mostly out of implementation and focused on orchestration, reconciliation, verification debt, stale state cleanup, and final claim ceilings.
5. Lanes had isolated worktrees, branches, state roots, ports, validation roots, and clear owned/forbidden paths.
6. Completion was not accepted from "task_complete" or agent confidence. It required receipts, clean worktrees, commands, artifacts, claim ceilings, and downstream root checks.
7. Skipped verification was classified instead of smoothed over.
8. Historical lane gaps were backfilled with live beneficial end-to-end runs.
9. Users corrected weak process while the goal was active, and those corrections became policy.
10. Review loops had adversarial personas and did not stop after one pass.

The plugin should preserve these mechanics by default.

## Foundation Sources

The proposal draws from five external sources and the repo's own operating standards:

- OpenAI Harness Engineering: repository-local system of record, agent legibility, standard tool use, worktree-local runtime visibility, mechanical invariants, and entropy cleanup. Source: https://openai.com/index/harness-engineering/
- OpenAI Symphony: task-centered orchestration, one agent per workspace, documented workflow contracts, and simplification through multi-language implementation attempts. Source: https://openai.com/index/open-source-codex-orchestration-symphony/
- OpenAI Codex ExecPlans: self-contained, living plans that can be restarted from the plan alone and must produce demonstrably working behavior. Source: https://developers.openai.com/cookbook/articles/codex_exec_plans
- Parse, Don't Validate: preserve boundary knowledge in typed representations rather than discarding it after checks. Source: https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/
- AI Is Forcing Us To Write Good Code: 100 percent coverage as an ambiguity-removal mechanism, namespace design as an interface for agents, and fast ephemeral concurrent dev environments. Source: https://bits.logic.inc/p/ai-is-forcing-us-to-write-good-code

## Plugin Product Requirements

The plugin must provide:

- a way to harden a goal into a contract bundle;
- a way to bind a Codex thread goal to that contract when goal tools exist;
- a way to author macro-lane ExecPlans;
- a lane registry template and validator;
- root verification backlog template and validator;
- completion manifest schema with typed claim ids;
- product-cohesion receipt schema for consumer-facing claims;
- ready-for-merge receipt generator that rejects hand-written readiness;
- validator receipts with producer/validator separation;
- lane registry with stale session/worktree cleanup obligations;
- review persona contracts;
- guidance for optional connectors without making connectors core to correctness;
- red fixtures and complete negative packets that prove the validator catches false completion patterns.

## Non-Goals

The plugin should not:

- become a general low-tier planning skill;
- encourage smallest-slice work when macro-lanes are appropriate;
- replace project-specific engineering judgment;
- assume every repo needs the exact `codex-workflow-rs` lane numbering or command names;
- require a UI control plane before the file-contract loop works;
- store raw private transcripts or secrets as proof;
- treat memory, chat summaries, or humanized event strings as current proof;
- auto-merge or mutate production surfaces without explicit repo policy.

## Architecture

The plugin has five layers.

### Layer 1: Goal Binding

`ultragoal` turns a high-tier user objective into a contract bundle and binds it to real goal mechanics when available. It refuses to simulate persistent goal state in prose. If goal tools are unavailable, the contract says so and the claim ceiling excludes "goal-bound" claims.

### Layer 2: Repo Harness

`harness-engineering` installs or proposes the repo-local system of record:

- short `AGENTS.md` as table of contents;
- standards document;
- architecture codemap;
- ExecPlan law;
- specialized root docs for design, frontend, product sense, quality scoring,
  reliability, and security;
- routed docs indexes for design docs, generated docs, product specs,
  references, and tech debt;
- check entrypoint;
- validation artifact root;
- backlog and receipt conventions.

This layer is where agent legibility becomes a repo property.

### Layer 3: Lane Execution

`execplan-lane` and `orchestrator-reconciler` create macro-lane contracts, assign isolation, launch lanes, monitor without context churn, and reconcile only when receipts are sufficient.

The lane contract must include:

- owned paths;
- forbidden paths;
- dependencies;
- worktree/branch/state roots;
- exact verification commands;
- live beneficial end-to-end proof;
- ready receipt path;
- claim ceiling;
- teardown and cleanup conditions.

### Layer 4: Proof And Claim Ceiling

`proof-gate` owns the rule that every advertised claim must map to proof. It validates the completion manifest and rejects false closure.

Claim statuses, positive-status subsets, evidence kinds, and proof surfaces are schema-owned in `schemas/schema-authority-primitives.schema.json`. Only `proven_live` and narrowly scoped `proven_static` claims can appear in a positive release claim ceiling.

### Layer 5: Entropy Cleanup

`standards-gardener` runs recurring checks for stale contracts, oversized files, namespace drift, stale worktrees, stale lanes, missing docs, obsolete allowlist entries, and unlinked proof artifacts.

## Recommended Skills

### ultragoal

High-tier only. It hardens the goal into a contract bundle, binds it to runtime goal mechanics, and refuses downgrade. It is not a lightweight task planner.

### harness-engineering

Umbrella router for agent-first repo work. It chooses fresh init, retrofit, runtime legibility, ultragoal, lane/orchestration, proof gate, or standards gardening instead of trying to put all behavior in one bloated skill.

### agent-first-repo-init

Fresh repo setup. It creates the starting shape a user otherwise has to know by
hand: `AGENTS.md`, compact `AGENT_STANDARDS.md`,
`agent-standards/` routed standards modules, `ARCHITECTURE.md`, `PLANS.md`,
specialized root docs (`DESIGN.md`, `FRONTEND.md`, `PRODUCT_SENSE.md`,
`QUALITY_SCORE.md`, `RELIABILITY.md`, `SECURITY.md`), routed docs indexes, one
gate, ExecPlan folders, proof root, repo-local skills, and runtime surfaces
where applicable.

### agent-first-repo-retrofit

Existing repo migration. It starts from live repo truth, preserves behavior and user changes, wraps current checks before tightening rules, and records missing proof as backlog instead of pretending the harness is complete.

### agent-runtime-legibility

Runtime proof setup. It makes local commands, logs, state roots, CLI/API help, UI proof, and receipts discoverable to agents so user-facing claims are not inferred from static files.

### execplan-lane

Creates macro-lane ExecPlans from large goal chunks and verifies launch readiness before any lane starts.

### orchestrator-reconciler

Keeps the parent thread in its proper role, manages lane registry, monitors receipts, merges with explicit path discipline, and cleans stale worktrees/sessions.

### proof-gate

Validates receipts, manifests, proof surfaces, verification backlog, live beneficial evidence, and final claim ceilings.

### product-cohesion-gate

Consumer-facing product guard. It blocks the common failure where engine proof,
goal contracts, and runtime receipts all work but the product feels like a pile
of separate tools. It requires a named user job, product promise, critical
journey, surface map, UI evidence, runtime truth, claim ceiling, and
human-attention policy. "Needs you" style human handoff is treated as an
exceptional escalation surface, not the default path for work the harness should
carry autonomously, and the receipt must cite evidence that harness-owned
recovery paths were exhausted before the user was interrupted.

### standards-gardener

Periodic cleanup and drift prevention. It encodes human taste and run lessons into mechanical checks or small standards updates.

## Recommended Agents

Routine implementation uses cheap deterministic checks. At a material lane
freeze, one risk-matched specialist exhaustively reviews the complete named
invariant and batches all material defects against the exact candidate. The
four material-review personas defined in `agents/` and
packaged in `custom-agents/` are reserved for major root integration, Product
Fitness, protected cross-domain change, release, completion, or explicit
escalation. Re-review requires changed authority-bearing bytes, a changed
consumed dependency, a newly eligible claim surface, or contradictory observed
behavior; review evidence is retained only when a current claim, handoff,
audit, irreproducible observation, or recovery path consumes it.

Supporting agents such as `plugin-scout`, `standards-extractor`, repo
initializer, and retrofit planner can help discovery or setup work, but they do
not raise the claim ceiling or substitute for the review topology required at
the current boundary.

## Optional Connectors

Core correctness should not depend on connectors. Optional adapters can improve workflow:

- Codex goal tools: required for true goal-bound claims when available.
- Codex app thread and automation tools: useful for real thread lanes and scheduled orchestration ticks.
- GitHub/Linear: useful if the repo's task tracker lives there.
- Proof: useful for human-in-the-loop spec review, but not required for proof of runtime behavior.
- Browser/Computer Use: required only for UI-surface claims.
- OpenAI Developers: useful for current API integration semantics if the plugin directly invokes platform APIs.
- Codex Security: useful for threat modeling and validation of a plugin that manages local execution.

## Mechanical Validation Model

The plugin includes a Rust canonical validator command. It preloads `schemas/schema-catalog.json`; network schema fetch is a validation failure:

    cargo run --offline -- --root . audit --receipt <output-dir>/validator-receipt.json

Required checks are schema-owned by `schemas/schema-authority-primitives.schema.json#/$defs/requiredValidatorCheckId`. Prose may explain why the checks exist, but must not become a second checklist source. Validator receipts are generated outputs, not copyable authoring templates; current proof values must be read from `validation_artifacts/ultragoal-audit/`.

Current Rust-canonical proof is recorded in `validation_artifacts/ultragoal-audit/validator-receipt.json`. Quote the fresh generated receipt for run id, package digest, fixture counts, and generated-artifact counts; source docs intentionally avoid embedding those moving values. Retired Python validator/reference files have been removed from the package and manifest.

## Semantic Validator Boundary

JSON Schema alone does not prove cross-document truth. The Rust validator computes the package-level, fixture-level, semantic-classification receipt, target-repo, and red-fixture semantic checks required to prove the current package/static/fixture surface. The remaining plugin surfaces, connector adapters, local app install visibility, and real multi-lane dogfood still remain future implementation work.

## Red Fixtures

A useful validator must fail known-bad packets. Required red fixture identity is schema-owned by `schemas/schema-authority-primitives.schema.json#/$defs/requiredRedFixtureId` and `schemas/red-fixtures-catalog.schema.json`. Each packet is a compact executable JSON Patch spec against its declared valid base fixture; the validator/test runner materializes the bad bundle and then runs semantic validation. Prose mutation notes are not evidence.

## Implementation Roadmap

`docs/implementation-roadmap.md` is planning guidance for future phases. Implementation phase gates must be reflected in typed receipts and validation artifacts before they become proof; this report is rationale, not a second roadmap.

## Claim Ceiling

Current claim proof is represented by generated validator receipts, detached review-target receipts, archive receipts, and other validator-checked artifacts. Markdown files such as `docs/hypercritical-review-law.md` and `docs/implementation-roadmap.md` are guidance and context only; they do not carry moving proof counts, reviewer approval, claim ceilings, or remaining-work proof.

The current proposal scope remains package/static/fixture proof. Local Codex app installation, app visibility, marketplace or workspace publication, real multi-lane dogfood, and external product UX improvement require fresh same-surface receipts in later phases.

## Remaining Work

Candidate archives are generated only after fresh validation, and upload/distribution archives are generated only after the required same-anchor reviewer sign-off is represented by the appropriate typed receipts.

## Source Provenance

Foundation-source provenance is tracked in `docs/source-cards.json`. The source cards record retrieval time, URL, usage, and provenance basis.
