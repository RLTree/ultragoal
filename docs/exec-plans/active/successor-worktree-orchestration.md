# Successor Worktree: Orchestration Authority Runtime

Operational lane contract only. The adopted successor contract remains
normative, and parent acceptance remains with the root integrator.

## Identity

- Node: `N10-ORCHESTRATION`.
- Thread: `019f5fa2-923f-7770-9a89-83ed713ac1f3`.
- Worktree: `/Users/terrynoblin/.codex/worktrees/cb63/harness-ultragoal-plugin-proposal`.
- Branch: `codex/n10-orchestration-authority`.
- Current integration base: `5cc3be5f860de0071837f478bbe3a9de0d6c0e47`;
  original launch base `20a11eb981c5db2d2136a555632be9c4e5ab7752` is historical.
- Live state: implementation is active and no handoff has been parent-reviewed.
- Contract manifest digest: `390138fa292fffa6975f501acd1570be6d32b8498f33651d44596009b900a130`.
- Dependency graph digest: `a779b96ccfeba031aa8565f44c808223fe3cf73847ce149eed8759c446905f2f`.

## Outcome And Lease

Implement the production root-authority issuer and durable owner-only
cross-process replay ledger behind the accepted adapter. Prove exact lease,
reservation, reconciliation, interruption, restart, ambiguity, and recovery
semantics without claiming sudden-power-loss durability unless exercised.

Owned paths are `validator/src/orchestration/**`, exclusive orchestration tests
and fixtures, and one WorkerResult-v1. The lane owns no public dispatch,
cross-domain context/state, Cargo surface, generated authority, migration or
claim registry, claim/release state, or shared documentation.

## Environment And Verification

Run the sanitized bootstrap in `.codex/environments/environment.toml`, then
use `.codex-worktree/run-command <command>` for lane commands; do not source
the compatibility `env.sh` projection. Exercise positive, negative, replay, race, mutation, security,
false-pass, multi-process restart, ambiguity, recovery, reconciliation, and
zero-write read behavior.

## Handoff And Teardown

Leave a clean committed branch with exact base/head, commits/paths, behavior,
checks/results, doc/generated impact, root requests, blockers, and
WorkerResult-v1. State explicitly that the lane claims no readiness, release,
completion, or parent acceptance. Root review/integration precedes archival and
worktree/cache removal.
