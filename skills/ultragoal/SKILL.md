---
name: ultragoal
description: High-tier goal-run contract hardening for large, multi-lane, proof-bound objectives. Use when the user wants the strongest goal mechanics, not lightweight planning.
---

# Ultragoal

Ultragoal is the top-tier goal-run skill. It does not seek the smallest viable slice. It creates a binding contract bundle for serious work where false completion, stale proof, or poor orchestration would be expensive.

## Tier Invariant

Ultragoal is monotonic:

- Missing proof raises or preserves obligations.
- Ambiguity raises or preserves obligations.
- Stale receipts raise or preserve obligations.
- Ownership conflict raises or preserves obligations.
- Validator failure raises or preserves obligations.

It never silently downgrades to a smaller planning mode.

## Required Bundle

An ultragoal run creates or verifies canonical artifacts:

- `GOAL_CONTRACT.md`
- `LANE_REGISTRY.json`
- `VERIFICATION_BACKLOG.json`
- `COMPLETION_MANIFEST.json`
- lane `EXECPLAN.md` files
- `AMENDMENTS.jsonl`
- `RED_FIXTURES.json`
- `agent-standards/enforcement.json`
- `agent-standards/enforcement.tsv`
- `agent-standards/enforcement-audit.tsv`
- `scripts/check-agent-standards`

Generated or optional projections may exist, but they are not authorities:

- `LANE_REGISTRY.md`
- `VERIFICATION_BACKLOG.md`
- `CONTRACT_AMENDMENT.md`
- validator receipts and ready receipts emitted by the validator

The bundle records:

- `contract_bundle_hash`
- `required_claim_ids_hash`
- contract version
- amendment log digest

Any contract change after launch requires a validated `AMENDMENTS.jsonl` append. Markdown amendment drafts are optional projections only.

## Goal Lineage

When runtime goal tools exist:

1. Call `get_goal`.
2. If no active goal exists, call `create_goal` with a prompt that binds the goal to the contract path.
3. Record goal id, objective, status, timestamp, thread id if available, and tool receipt.
4. Rebind on resume, handoff, replacement, and completion review by calling `get_goal` again.

Do not simulate persistent goal state in chat. If goal tools are unavailable, record a goal-tool discovery probe receipt and remove any goal-bound claim from the claim ceiling.

## Active Setup To Idle Orchestration

Use goal binding for active setup and launch: prepare the repo, establish
standards, write and validate macro-lane ExecPlans, create or verify branches
and Codex app worktree threads, and launch the first wave of lane owners.

After first-wave launch, the orchestrator may install and verify
`.codex/automations/ultragoal-orchestrator/automation.toml`, record a
transition receipt, and go idle. The automation is a heartbeat continuation
contract, not a generic reminder and not a substitute for active setup. It
wakes to inspect current cursors, sends `DONT_NOTIFY`, `STEER`, or `ESCALATE`,
and routes the next active action when lanes are ready or blocked.

## Proof Contract

The run must define closed `required_claim_ids`. Every advertised claim maps to exactly one required claim id or an explicit non-required informational claim.

Claim status uses the canonical `ClaimStatus` enum in `../../schemas/schema-authority-primitives.schema.json`. Only `proven_live` and carefully scoped `proven_static` claims may appear in a positive completion claim ceiling.

## Lane Contract

Ultragoal work uses macro-lanes, not tiny task fragments. Each lane gets one ExecPlan-scale contract unless a documented amendment splits it.

Each lane must define:

- purpose and claim ceiling;
- owner and role;
- worktree, branch, state roots, scratch roots, tool cache roots, validation artifact root, and ports;
- owned paths and forbidden paths;
- typed dependency claims it consumes, including upstream lane id, claim id, required status, validated status, evidence digest, commit, and validation time;
- exact verification commands;
- live beneficial end-to-end proof requirement;
- ready receipt path;
- teardown conditions.

Lane-local subagents are allowed only inside the lane and must be registered as child lanes or verifier agents.

## Parent Role

The parent session is orchestrator and reconciler. It may:

- create and harden contracts;
- launch or monitor lanes;
- inspect receipts;
- send lanes back for lane-owed work;
- reconcile merges;
- update root verification backlog;
- clean stale worktrees and sessions;
- perform final root verification.

It may not implement lane-owned goal work unless it creates a registered parent-owned lane entry first.

## Validation Contract

Completion requires a generated `ultragoal-audit` validator receipt where every
schema-owned required check id passes. Summaries may name claim closure,
evidence coupling, lane/session hygiene, backlog coverage, live beneficial
proof, and red-fixture results, but the schema-owned check set is authoritative.
Before material claims, rerun the standards checker or the Rust
`agent-standards-enforcement` audit check and refresh the enforcement audit.
Unclassified standards rows block completion; blocked or backlogged rows reduce
the claim ceiling for the affected claim ids.

## Stop Conditions

An ultragoal run may stop only when:

- all required claims are proven or explicitly withheld/blocked with claim ceiling reduced; and
- validator passes; and
- `VERIFICATION_BACKLOG.json` has no unclassified rows; and
- `agent-standards/enforcement.*` has no missing or unclassified obligations;
  and
- stale worktrees and stale sessions are closed or explicitly preserved with reason; and
- final report states proof, gaps, security, performance, quality, and claim ceiling.

Otherwise the run is still active or blocked.
