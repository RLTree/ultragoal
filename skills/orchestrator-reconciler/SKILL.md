---
name: orchestrator-reconciler
description: Parent-session orchestration and reconciliation for ultragoal lanes.
---

# Orchestrator Reconciler

Use this skill in the parent session of a multi-lane goal run.

## Role Boundary

The parent is not a general implementation worker. It coordinates, monitors, reconciles, validates, and closes gaps. If it must implement, it creates a parent-owned lane entry first.

## Duties

- Keep the goal contract current.
- Keep `LANE_REGISTRY.json` current as the canonical lane, dependency, and root-verification authority.
- Keep any markdown lane registry view reconciled to the JSON authority.
- Ensure each lane is goal-bound when tools allow.
- Ensure each lane has an ExecPlan-scale contract.
- Prefer Codex app-managed worktree threads for substantial macro-lanes; create
  or verify the branch first, then create the app worktree thread.
- Launch future worktree lane owners with `gpt-5.5` and `low` reasoning by
  default. Reviewer agents are separate and still use `gpt-5.5` with `high`.
- Ensure dependencies do not consume unverified claims.
- Inspect exact receipts and logs, not broad scans.
- Send lane-owed work back to the owning lane.
- Classify root-owed, externally-blocked, and withheld claims.
- Merge only after sufficient evidence.
- Use explicit path staging if committing.
- Tear down stale worktrees and stale sessions.
- Preserve user changes.
- Launch or resume newly unblocked macro-lane worktree threads after dependency
  lanes land, root verification passes, and no other blockers remain.
- After active setup and first-wave lane launch, create or verify the
  Ultragoal orchestrator automation from
  `.codex/automations/ultragoal-orchestrator/automation.toml`, record the
  transition receipt, and let heartbeat wakeups handle idle monitoring instead
  of burning a continuous goal-bound session.

## Polling Discipline

Use low-frequency checks appropriate to lane size. Do not burn context on baby waits. If automation exists, verify the real automation artifact.
The automation must be self-contained, bound to the orchestrator thread, goal,
repo root, lane registry, active ExecPlan directory, backlog, completion
manifest, tick receipt, output contract, tools, skills, evidence cursors, and
claim ceiling.

## Merge Discipline

Before merging a lane:

- read ready receipt;
- read final lane report;
- inspect diff and changed files;
- inspect worktree status;
- compare base commit, target head at launch, target head at validation, and merge base;
- verify command evidence;
- verify claim ceiling;
- verify Product Fitness receipt when a lane claims product success, readiness,
  daily-driver fitness, release, or material product sign-off;
- update verification backlog;
- confirm no unrelated changes are staged.

After merging:

- run integrated checks appropriate to the lane;
- preserve or update verification backlog;
- identify dependent lanes whose blockers are now lifted;
- create or resume their Codex app worktree threads with the lane ExecPlan
  contract and current root proof anchors;
- remove stale worktree if complete;
- record merge commit and final claim status.

## Blocker Discipline

Do not wait for a blocker to resolve itself. Classify and act:

- lane can run it: send back or relaunch lane;
- root-only: record root-owed and schedule root proof;
- external: record exact missing auth, artifact, product decision, or state;
- invalid claim: withhold it.
- missing Product Fitness proof for a product-impacting claim: withhold product
  success/readiness claims and route repair to the lane or root owner.
