# ExecPlans, Worktrees, And Orchestration

## ExecPlans

Long-running work uses self-contained living ExecPlans. A future agent should
be able to restart from the plan alone.

An ExecPlan includes purpose, observable outcomes, current progress,
discoveries, decision log, concrete steps, validation, explicit blockers,
idempotence, recovery, and artifacts.

Revise plans as work proceeds. Do not leave stale plan claims behind because a
later chat message corrected them.

## Worktree And State Isolation

Concurrent agents work in isolated workspaces. Isolation includes branch,
working tree, state roots, artifact roots, ports, caches, scratch paths,
credentials, and mutable runtime state.

- Agents preserve user changes and never sweep unrelated files into their work.
- Dirty worktrees, foreign edits, and stale state roots are blockers or
  coordination points, not reasons to widen scope.
- Teardown preserves evidence before deleting scratch state.
- Codex app-managed worktree threads are the preferred owners for ExecPlan
  macro-lanes that should be visible, resumable, and independently operable in
  the Codex app.
- This preference is conditional. If the active goal, spine, parent contract,
  or phase order blocks worktrees, no standard here permits launching them.
- Future Codex app worktree lane owners default to `gpt-5.5` with `low`
  reasoning to control cost. Raise reasoning only when the lane contract names
  a concrete risk that requires it. Material reviewers remain separate and use
  `gpt-5.5` with `high`.
- Branch first, worktree second. The orchestrator must ensure the branch ref
  exists before requesting an app worktree from that branch.
- Repo-managed Codex app environments should create ignored per-worktree state
  under `.codex-worktree/`, write `.codex-worktree/env.sh`, and assign isolated
  state, scratch, home, temp, port, cache, and target directories.
- Runtime output belongs in ignored per-worktree state, not in repo-managed
  `.codex/` configuration.
- Durable identity belongs in repo-managed contracts and receipts. Ephemeral
  worktree, cache, debug, replay, and local environment state belongs in
  ignored per-worktree paths.
- One worktree per macro-lane is reused across that lane's repair and review
  loops.
- Worktrees are storage debt. Clean merged worktrees must be closed after
  verification; dirty, stale, unmerged, or abandoned worktrees require an audit
  record naming owner, branch, status, age, and next action.

## Orchestration Doctrine

Organize substantial work around deliverables and proof, not chat sessions.

- Successful orchestration means the parent can answer, from current artifacts,
  who owns each lane, which branch and worktree it uses, what it may edit, what
  it depends on, what proof it owes, whether its state is fresh, and what must
  happen next.
- The orchestrator owns the dependency graph, not the lane implementation. It
  defines lane contracts, creates or verifies workspaces, routes current
  context, watches readiness and blockers, reconciles dependencies, preserves
  proof anchors, and prevents stale or overlapping work from being treated as
  complete.
- Macro-lane owners should be app-visible Codex worktree threads when the work
  needs independent continuation, app sidebar visibility, or user handoff.
- The launch prompt or thread setup records the lane-owner model and reasoning;
  default lane owners use `gpt-5.5` with `low`, not the reviewer settings.
- Hidden subagents are not substitutes for app-visible macro-lane owners when
  the user expects to inspect or continue the lane in Codex.
- Lane completion is not root completion. A lane-ready claim must be joined to
  current root state, dependency state, proof anchors, clean worktree, teardown
  condition, and review gate before the parent may claim the larger deliverable
  is complete.
- When a dependency lane lands, the orchestrator merges it to the root
  integration branch, verifies the stated root gate, confirms the lane branch
  tip is reachable, and advances newly unblocked dependent worktree threads.
- If advancing a dependent worktree would overwrite dirty scoped work, create
  conflicts, or require semantic choices, classify it as lane-local
  reconciliation and steer the lane owner to resolve it in lane context.
- `PLANS.md` is stable ExecPlan law, not active project state. Worker ids,
  phase progress, backlog rows, receipt state, and completion claims belong in
  active ExecPlans, lane registries, verification backlogs, receipts, or
  completion manifests.

## Lane Completion Message

Lane agents notify the parent with a ready package, not a vague "done."

Required fields:

- lane id, branch, worktree, and current commit;
- changed owned paths and confirmation that forbidden/shared paths were not
  modified;
- commands run, exit codes, and artifact paths;
- ready receipt path and claim ceiling;
- dirty worktree status or explicit preserved uncommitted paths;
- blockers, withheld claims, and next recommended parent action.
