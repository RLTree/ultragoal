# Plugin And CLI Authority Integration

This is the active restart contract for the adopted Harness Ultragoal successor
implementation. It is operational state, not proof and not a second product
contract. Normative authority remains the adopted bundle rooted at
`docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/00-READ-ME-FIRST.md`.

## Purpose / Big Picture

Finish the plugin product and typed Rust CLI kernel from current repository
truth. First reconcile the intentionally committed self-law refactor into a
warning-free, behaviorally honest integration checkpoint. Then launch a small
number of dependency-closed Codex worktree sessions and integrate each frozen
increment through root-owned review, wiring, proof, and claim decisions.

## Durable Goal Binding

- Runtime goal id and thread id: `019f5f39-507b-78a2-a96a-0b566a5a2126`.
- Runtime status at binding: `active` via `create_goal`.
- Binding receipt:
  `docs/ultragoal-successor-live/root-decisions/HOST-GOAL-BINDING.json`.
- Binding candidate: branch `codex/successor-contract-v2-live-product`, commit
  `e60fc9897368a946bcdb56c8f5e0c52ab379c39e`, tree
  `1ec0ca72326068177da758fc61c078b63d100e94`.
- Host goal state does not raise any product claim. Rebind with `get_goal` on
  resume, handoff, replacement, and completion review.
- This is a continuation binding explicitly requested by the current user. It
  neither replaces the adopted contract nor reopens the initial bootstrap goal.

## Progress

- [x] Recovered repository root, branch, worktree list, status, history, and
  intentional three-commit candidate after `40982060a`.
- [x] Read the routed standards, architecture, security, reliability, product,
  contract, claim, graph, migration, and safe open-decision surfaces.
- [x] Bound the durable host goal with `create_goal`.
- [x] Rejected stale WorkerResult and board rows as current proof.
- [ ] Reconcile formatting, authored-file size, warning-free compilation,
  standards digests, and the routine false-pass defect.
- [ ] Run fresh focused candidate checks, coverage authority, public self-law,
  independent review, and record root acceptance for the checkpoint.
- [ ] Recompute the live graph and launch dependency-ready macro-sessions from
  the exact accepted checkpoint.
- [ ] Integrate accepted sessions one at a time and rerun dependency-closed
  root checks after each landing.
- [ ] Close installed/runtime journeys, migration/retirement, claims, release,
  two identical inventories, and the final requirement-to-evidence audit.

## Surprises & Discoveries

- The handoff snapshot was stale: the root was already clean at `e60fc9897`,
  and the three self-law commits were intentional.
- The old external `E0521` compile blocker is gone. Fresh library compilation
  instead fails on 657 candidate-owned diagnostics: 640 dead-code, 16 unused
  imports, and one private-interface error. Dormant candidates must be kept out
  of production compilation until a real caller exists; active product code
  must be wired or retired rather than hidden by lint allowances or visibility
  widening.
- Fresh direct formatting fails in 14 Rust files. One authored contract test is
  251 lines. `scripts/check-agent-standards` also detects a stale audit digest.
- Public routine success is self-certifying: the child script writes a marker
  and reports `behavior_observed=true` without running the declared repository
  behavior. This remains decisive REWORK.
- `CRITICAL-PATH-BOARD.md` and the plugin-delivery WorkerResult are anchored to
  `40982060a`; both require current-candidate reconciliation.

## Decision Log

- Preserve commits `1594ded64`, `a16753a66`, and `e60fc9897` as intentional
  development history. Repair forward; do not rewrite or squash them.
- Do not launch implementation worktrees until the self-law candidate has a
  clean compile, focused behavior proof, current evidence, and independent
  root acceptance.
- Keep incomplete product candidates test-only or otherwise outside the
  production module graph until a real public caller exists. Do not use
  `allow(dead_code)`, dummy references, or broad public visibility as proof.
- Replace routine outcome selection with a closed root-adopted behavior recipe.
  Repository data may select a behavior, never a pass/fail outcome.
- Keep OD-001 through OD-010 safe defaults: local, unpublished, unsigned,
  non-destructive, no optional export or executable extension, and claims only
  for independently exercised environments.

## Context And Orientation

- Product package: `.codex-plugin/`, `skills/`, `agents/`, `install/`,
  `plugin-manifest-draft.json`.
- CLI crate: `validator/`; public grammar under `validator/src/cli/successor/`
  and public dispatch under `validator/src/cli/successor_public/`.
- Domain behavior: `validator/src/{distribution,evaluation,inventory,migration,
  orchestration,package,plugin_product,repository_fit,routine_work}/`.
- Root-only authority: contracts, Cargo workspace/lockfile, public catalog and
  dispatcher, aggregate generated authority, migration/claim registries,
  candidate identity, integration order, release, and completion.
- Current evidence router: `docs/ultragoal-successor-live/`; records remain
  context until their candidate identity and behavior are freshly reconciled.

## Plan Of Work

### A. Recover The Integration Checkpoint

1. Repair rustfmt and the 250-line authored-file violation without semantic
   churn.
2. Classify each production warning as active wiring, safe deletion, or dormant
   candidate isolation. Preserve no fake production reachability.
3. Replace the routine marker/report false pass with a typed behavior recipe,
   trusted outcome derivation, causal failure, interruption, and reuse tests.
4. Regenerate root-owned standards and generated authority through their
   canonical projectors.
5. Run clean compile, focused domain suites, source laws, exact coverage, and
   the public recursively zero-write self-law command.
6. Freeze a current WorkerResult, run independent full-scope review, and commit
   root acceptance or return the exact defect to recovery.

### B. Launch Dependency-Closed Product Sessions

After checkpoint acceptance, recompute the graph and create descriptive
product-role branches before requesting Codex app worktrees. Candidate sections
are delivery/runtime, orchestration/goal, evaluation/research, then proof and
migration. Final ownership and paths come from the refreshed graph.

Pending session contracts are deliberately unlaunched until checkpoint
acceptance. Unknown runtime fields must be resolved by these exact probes:
`git rev-parse HEAD`, `git status --porcelain=v2`, `git worktree list
--porcelain`, `codex_app__list_projects({})`, branch creation from the accepted
HEAD, then `codex_app__create_thread` with `startingState.type=branch`. The
create-thread result supplies thread and worktree identity; model and reasoning
remain `unknown` unless the host exposes them.

- Delivery runtime: branch `codex/successor-delivery-runtime`; owns
  distribution, package, plugin-product, install, and matching fixtures; starts
  first; ready record `worker-results/DELIVERY-RUNTIME-WORKTREE.json`.
- Orchestration goal: branch `codex/successor-orchestration-goal`; owns
  orchestration product/recovery and matching fixtures; depends on delivery;
  ready record `worker-results/ORCHESTRATION-GOAL-WORKTREE.json`.
- Evaluation research: branch `codex/successor-eval-research`; owns evaluation,
  fixture-scheduler, and matching fixtures; depends on delivery and may run
  beside orchestration; ready record `worker-results/EVAL-RESEARCH-WORKTREE.json`.
- Proof migration: branch `codex/successor-proof-migration`; owns claims,
  migration implementation, and matching fixtures; depends on orchestration and
  evaluation; ready record `worker-results/PROOF-MIGRATION-WORKTREE.json`.

All sessions forbid contracts, Cargo files, `.codex-plugin`, `.codex/agents`,
public catalog/dispatcher, global generated authority, migration/claim registry,
candidate identity, integration, claims, and release. Before launch, each gets
a dedicated ExecPlan with exact base/head, launch prompt, owned/forbidden paths,
`.codex-worktree/env.sh`, state/scratch/home/temp/cache/target roots, dependency
digests, verification commands, review cadence, ready receipt, and teardown
condition. Absence of that file blocks `create_thread`.

Every session starts at the exact accepted commit, owns disjoint paths and
semantics, uses isolated `.codex-worktree` state and Cargo targets, and returns
a committed WorkerResult-v1 package without readiness, release, or completion
claims.

### C. Integrate Continuously

For each frozen session: inspect the commit and WorkerResult, run independent
falsification, return one material defect for REWORK, integrate one accepted
increment, apply root-owned wiring, rerun the dependency closure, refresh any
remaining worktrees after shared-interface changes, then archive and remove
the accepted session when no unique state remains.

### D. Close Product And Proof Surfaces

Run source, package, install, cache, marketplace, app-registry, discovery,
runtime, and product behavior proof separately. Exercise clean, dirty, partial,
conflict, failure, interruption, recovery, repeat-use, adversarial, and
fresh-agent journeys. Reconcile migration absence, exact claims, release proof,
two byte-identical inventory summaries, and every requirement-to-evidence row.

## Concrete Checkpoint Commands

- `cargo fmt --all -- --check`
- `RUSTFLAGS=-Dwarnings cargo check -p ultragoal --all-targets --offline --jobs 16`
- `python3 scripts/check-python-source-laws .`
- `python3 scripts/check-agent-standards .`
- `cargo test -p ultragoal --test plugin_agent_discovery_contract --offline --jobs 16 -- --test-threads 16`
- `cargo test -p ultragoal --test supported_package_product_contract --offline --jobs 16 -- --test-threads 16`
- `cargo test -p ultragoal --test migration_product_contract --offline --jobs 16 -- --test-threads 16`
- `cargo test -p ultragoal --test fixture_scheduler_contract --offline --jobs 16 -- --test-threads 16`
- `scripts/check .`
- Built `ultragoal --root . check strict --claim cli-self-law-compliance`
  with recursive filesystem and Git comparisons before and after.

Use an isolated Cargo target for each concurrent workspace. Broad checks run
only at dependency-closed freezes.

## Validation And Acceptance

Checkpoint acceptance requires current-candidate clean compilation, formatting,
source laws, standards projection, focused positive/negative behavior, exact
coverage or an explicit withheld material source ceiling, a recursively
zero-write public self-law run, refreshed WorkerResult, independent review, and
root acceptance. No source checkpoint proves package, install, discovery,
runtime, journey, release, or completion.

Final acceptance comes from the current user request interpreted through the
adopted contract. The host goal mirrors execution intent but is not product or
completion authority. Any open material requirement, migration, claim,
representative journey, release surface, stale generated authority, dirty
worktree, or final reconciliation gap keeps the goal active.

## Idempotence And Recovery

- Repair forward from the latest accepted commit; never reset or discard user
  work.
- Re-run projectors from canonical inputs; never hand-edit generated outputs.
- On interruption, rebind the goal, verify branch/HEAD/status/worktrees, read
  this plan, and rerun only the narrow highest-authority failing check.
- A dirty or semantically conflicted worktree is reconciled by its owner before
  parent integration. The parent does not force, stash, or reset it.

## Artifacts And Notes

- Durable goal receipt: `docs/ultragoal-successor-live/root-decisions/HOST-GOAL-BINDING.json`.
- Active state: this ExecPlan.
- Current stale router: `docs/ultragoal-successor-live/CRITICAL-PATH-BOARD.md`.
- Current blocked candidate record:
  `docs/ultragoal-successor-live/worker-results/PLUGIN-DELIVERY-SELF-LAW-REFACTOR.json`.
- Canonical repository check: `scripts/check`.

## Interfaces And Dependencies

The dependency graph in `IMPLEMENTATION_DEPENDENCY_GRAPH.json` remains the
normative ordering contract. This plan records live execution state only.
Worker sessions may request root changes but never edit or claim root-only
authority. Parent acceptance joins current source, dependency state, behavior,
evidence, worktree state, documentation freshness, and claim ceiling.

## Outcomes & Retrospective

Open. Update after every accepted integration checkpoint and at final claim
reconciliation. Until then, the highest honest ceiling is intentional source
history under REWORK.
