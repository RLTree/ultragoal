# Successor Worktree: Distribution Runtime

Operational lane contract only. The adopted successor contract remains
normative, and parent acceptance remains with the root integrator.

## Identity

- Node: `N04-DISTRIBUTION`.
- Thread: `019f5fa2-0bc3-7a50-9c2f-c377d88c60e4`.
- Worktree: `/Users/terrynoblin/.codex/worktrees/45df/harness-ultragoal-plugin-proposal`.
- Branch: `codex/n04-distribution-runtime`.
- Accepted integration base: `5cc3be5f860de0071837f478bbe3a9de0d6c0e47`;
  original launch base `20a11eb981c5db2d2136a555632be9c4e5ab7752` is historical.
- Accepted source: `125f54bcd4820497d06ff36d1a90405a39bc1f5a` / tree
  `b53918ab8ec00c7e84672181693607eb5253841c`.
- Accepted corrected receipt: `75bced89f4fa29d26edbe1b5824cd52c80ddb58d` /
  tree `f57dc811752f4e54275e03764a509e0d37e667e9`; WorkerResult SHA-256
  `47c6aa022f3ad0be86ab6d179400411b61ab2be3153adcdf91bafa19a400d549`.
- Root integration basis: `665140d11c00b0706a60c94508efdbb1a2093667` /
  tree `8dd8a461a93652d8ed9e5ac01984e7323529e922`.
- Live state: source transaction and receipt accepted, root-integrated, and
  retired. The Codex task is archived and the clean worktree/cache is removed.
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

The frozen handoff satisfied that protocol. Independent source review accepted
the exact 129-path set, and independent receipt review accepted the corrected
self-excluding WorkerResult. Root integration passed distribution 206/206 with
one ignored, adapter 14/14, host-effect executor 7/7, package identity 4/4,
rollback retry 1/1, and rustfmt. Root strict compilation remains withheld at 608
warning-as-error diagnostics. No repository package, install, cache,
marketplace, app-registry, discovery, runtime, readiness, release, node closure,
or completion claim is accepted.
