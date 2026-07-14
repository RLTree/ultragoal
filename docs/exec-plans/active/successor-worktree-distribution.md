# Successor Worktree: Distribution Runtime

Operational lane contract only. The adopted successor contract remains
normative, and parent acceptance remains with the root integrator.

## Identity

- Node: `N04-DISTRIBUTION`.
- Thread: `019f5fa2-0bc3-7a50-9c2f-c377d88c60e4`.
- Worktree: `/Users/terrynoblin/.codex/worktrees/45df/harness-ultragoal-plugin-proposal`.
- Branch: `codex/n04-distribution-runtime`.
- Current integration base: `5cc3be5f860de0071837f478bbe3a9de0d6c0e47`;
  original launch base `20a11eb981c5db2d2136a555632be9c4e5ab7752` is historical.
- Live state: decisive parent REWORK; repair is active and no handoff is accepted.
- Contract manifest digest: `390138fa292fffa6975f501acd1570be6d32b8498f33651d44596009b900a130`.
- Dependency graph digest: `a779b96ccfeba031aa8565f44c808223fe3cf73847ce149eed8759c446905f2f`.

## Outcome And Lease

Implement the supported macOS distribution transaction behind the accepted
deterministic source package: repository package output, install, cache,
marketplace, app-registry, discovery, and runtime identities, each observed
separately.

Owned paths are `validator/src/distribution/**`, `validator/src/package/**`,
exclusive distribution/package tests and fixtures, and one WorkerResult-v1.
The lane owns no plugin descriptor, marketplace manifest, version adoption,
public CLI catalog/dispatcher, Cargo surface, generated authority, registry,
claim, release state, or shared documentation.

## Environment And Verification

Run `.codex/setup-worktree-env.sh`, source `.codex-worktree/env.sh`, and use its
isolated Cargo, home, state, scratch, and temporary roots. Verify positive,
negative, race, mutation, confinement, false-pass, interruption/recovery,
repeat-use, and zero-write behavior. Source tests cannot prove installed or
runtime claims.

## Handoff And Teardown

Leave a clean committed branch with exact base/head, commit list, changed paths,
behavior and assumptions, exact checks/results, generated/doc impact, root
requests, blockers, and WorkerResult-v1. State explicitly that the lane claims
no readiness, release, completion, or parent acceptance. Root reviews and
integrates one frozen increment; only then may this thread be archived and its
worktree/cache removed.
