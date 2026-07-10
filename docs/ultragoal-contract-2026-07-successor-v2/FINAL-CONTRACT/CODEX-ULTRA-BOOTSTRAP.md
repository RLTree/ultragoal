# Codex Ultra Bootstrap

## Project to open

Open the **actual live Harness Ultragoal Git repository working tree intended for implementation** as the local Codex project. Do not open the archived snapshot as the implementation project. Keep this handoff bundle accessible read-only inside or adjacent to the project; the exact `/goal` prompt tells Ultra how to locate it.

## Verify the handoff before reading it

From the handoff root, run a SHA-256 verifier against `FINAL-HANDOFF-MANIFEST.sha256`. Reject the bundle if any listed file is missing, extra contract file is unexplained, or any digest differs. Then load:

1. `FINAL-CONTRACT/00-READ-ME-FIRST.md`
2. `FINAL-CONTRACT/CONTRACT_MANIFEST.json`
3. `FINAL-CONTRACT/ULTRA-INPUT-MANIFEST.json`
4. `FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json`
5. `FINAL-CONTRACT/REQUIREMENT_TRACE.json`
6. `FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json`
7. `FINAL-CONTRACT/CUSTOM_TOOL_INVENTORY.json`
8. `FINAL-CONTRACT/CLAIM_REGISTRY.json`
9. `FINAL-CONTRACT/MIGRATION-AND-RETIREMENT.md`
10. `FINAL-CONTRACT/PRO-ADVERSARIAL-REVIEW.md`
11. `FINAL-CONTRACT/OPEN-DECISIONS.md`

## Recompute the live candidate

Before writing, the Ultra root must locate the real Git root and record machine-readable output for current branch/HEAD, `git status --porcelain`, `git worktree list --porcelain`, submodules where present, candidate diff/base, untracked/ignored risks, relevant tool and Cargo metadata, current plugin/marketplace/skill/agent surfaces, CLI command inventory, generated authorities, host capabilities, permissions, and exposed runtime metadata. Snapshot values are comparison context only.

Resolve discrepancies in favor of live evidence and update the work graph. Do not silently rewrite this contract; propose and review a contract change when live facts invalidate a requirement or implementation assumption.

## Decompose work

Use the DAG topological order, but dynamically parallelize dependency-ready nodes with validated non-overlapping semantic write leases. Use read-only custom agents for reconnaissance and independent review. Use run-scoped implementation workers for `WS-CLI-CORE`, `WS-DISTRIBUTION`, `WS-FIT`, `WS-ROUTINE`, `WS-OBSERVE`, `WS-PLUGIN`, `WS-AGENTS`, `WS-EVAL`, and `WS-MIGRATION`. The root alone applies requested shared-authority changes, integrates branches/worktrees, and reconciles claims.

Every worker must return `WorkerResult-v1`. Reject stale context, out-of-scope writes, unreviewed material changes, missing artifacts, unsupported success claims, and workers that modify shared authority.

For host-protected `.codex/`, `.agents/`, and Git metadata, workers return exact `requested_root_changes`; only the root may apply them through exposed approval/capability boundaries.

## What Ultra must refuse to claim

Ultra must not claim a requested model/mode/configuration unless Codex exposes it; source/package/install/discovery/runtime/product truth must not be collapsed; tests, receipts, rows, docs, telemetry, signatures, provenance, or generated manifests must not substitute for behavior; a missing custom tool blocks dependent claims; a package or release does not prove completion; host goal status does not prove product state; and unresolved external/destructive decisions remain explicit.

Paste the exact content of `CODEX-ULTRA-GOAL.md` as the goal without rewriting.
