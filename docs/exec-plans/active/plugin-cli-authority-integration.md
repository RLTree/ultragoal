# Plugin and CLI Authority Integration

This is the active non-authoritative projection plan for the Harness Ultragoal
successor. It is not contract, queue, lease, or proof authority. Normative scope remains the bundle rooted
at `docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/00-READ-ME-FIRST.md`.
`LANE_REGISTRY.json` is the sole current operational authority;
`VERIFICATION_BACKLOG.json` and `COMPLETION_MANIFEST.json` are root-written
projections. This plan explains how the root advances that state. Its exact
digest is bound by `LANE_REGISTRY.json.source_context.operational_plan`; the
historical duplicate-plan digest remains a tombstone with commit/tree provenance.

## Stage B projection freeze (routing only)

The literal protected checkpoint is commit
`97e24c9706e7b489bdbdc6184ff9520a7116c6fd` / tree
`c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e`. It is provenance, not the
candidate: every candidate is the clean containing `HEAD`/tree derived live
from that checkpoint. Stage A is accepted for the registry, closed lease
contract, template, gates, scopes, graph, and review anchors; this plan and
the board remain projections and cannot promote claims.
The exact scheduler frontier is N04-N07 after N03, N09 after N02+N05, N08 after
N04-N07, N10 after N03+N06+N09, N11 after N06+N07+N08, and N12 after N10+N11.
After N12 the route is N12-A -> N14 -> N14-proof -> N12-B-invalidate-and-reproof
-> parallel N13/N15 -> N16 -> N17. P0 is available only to root for compile,
namespace, standards, and retention-aware cleanup; product lanes remain blocked.
All fourteen claims remain withheld with empty evidence and no validator
receipts. Cleanup removed 12.8 GB of reproducible state; 949 MB of unique
home/tmp evidence remains retained. Retention-aware cleanup is still blocked
until unique evidence is reviewed and the root handoff is current.

## Outcome

Complete the plugin-led daily-driver product and typed Rust CLI kernel from
current repository truth. Preserve accepted source work while proving source,
package, install, cache, marketplace, app registry, discovery, runtime,
product, migration, release, and completion claims on their own surfaces.

The current P0 candidate is always the clean containing `HEAD`/tree. The
literal checkpoint above must remain an ancestor with its exact tree; the
freeze derives live HEAD/tree, permitted root paths, payload digests,
projection chain, and clean status. Prelaunch compile (569 lib / 614 test), namespace runtime
verification, and four-persona exposure are blocked; standards rows are 122
current. No lane is selectable and no lease is issued. All claims remain
withheld. Namespace and standards are blocked/unavailable until their real
validator commands produce candidate-bound evidence.

## Durable binding and contract lineage

- Active host goal: `019f5f39-507b-78a2-a96a-0b566a5a2126`.
- Branch: `codex/successor-contract-v2-live-product`.
- `AMEND-001` strengthens all 14 required `CL-*` claims without removing or
  promoting one.
- No `AMEND-002` is required for operational path or worktree-topology changes
  that preserve requirements, graph edges, and claim ceilings. Any normative
  change still requires the append-only amendment mechanism.
- Chat, plans, boards, tests, and receipts do not promote claims. Exact behavior
  on the required surface plus root reconciliation does.

## Controller policy

Use this selection order:

1. Integrate an accepted exact candidate.
2. Repair the rejected critical-path candidate at the shared invariant.
3. Unblock the nearest installed daily-driver journey.
4. Investigate only when uncertainty prevents one of those actions.
5. Launch another stream only when root integration capacity is free and its
   ownership is dependency-independent.

One candidate may await review or integration while one genuinely independent
repair stream runs. Accepted work lands before replacement work launches.
One material defect returns the candidate to its owner. Repeated same-class
defects require one invariant-level repair across sibling, descendant,
rollback, recovery, reconciliation, cleanup, and replay transitions.

Reviewer/fixer loops use direct communication. Advisory development review is
Sol/high; bounded repair is Luna/xhigh. Cached checks run during repair. Builds,
strict checks, and broad audits run only at the decision boundary they can
support. The repo-defined four-persona round is required for material signoff;
until installed active-registry exposure is current, generic reviewers remain
advisory and material signoff stays withheld.

Each freeze collects one exhaustive meaningful issue set. Only authority or
security unsafety, false-pass enablement, integration invalidity,
operator-journey breakage, or high-compounding architectural debt returns the
candidate for REWORK. Other findings go to `VERIFICATION_BACKLOG.json` with a
named trigger, owner, and withheld-claim impact.

## Dependency and integration path

The accepted execution path is V8.1:

1. P0 canonical state and authority: sanitize the worktree environment, replace
   stale operational state with the N00-N17 registry, keep all historical
   acceptance refs nonselectable, and record every required claim as withheld.
2. P1 N02/N03 current source and public-interface reconciliation.
3. Run residual N04-N07 source work only for an exact changed invariant. Do not
   advance a node while N02 inventory closure is open.
4. N09 and N08 may proceed in parallel after their own authored dependencies;
   there is no invented N09-to-N08 edge.
5. N10 and N11 may proceed in parallel after their own dependencies; refresh
   N11 only when N10 changes its exact consumed-set digest.
6. N00 adoption is the first dependency gate after P0. It starts a new epoch,
   quarantines prior source-only evidence, and requires N01-N11 rebind and
   re-observation before any scheduler or claim eligibility.
7. N12 is the sole HCT-CLAIMS decision writer. N13, N15, N16, and N17 are
   read-only claim-reconciliation consumers.
8. Advance N12-A, then N14 and N14-proof, then N12-B-invalidate-and-reproof.
   Run N13 and N15 in parallel after that reproof, then N16 and N17.

### P0 freeze gates and lease protocol

`PRE-ADOPTION-SOURCE` is a root precondition, not a DAG node. It binds the
current FINAL-CONTRACT graph and the historical DEFER-052 decision. Only
source-local candidate preparation is allowed; live, product, claim, and
inventory-adoption ceilings stay withheld. N00 adoption quarantines, rebinds,
and reruns N01-N11.

The root records four prelaunch gates with owner, command, status, and the
derived clean candidate: compile (blocked), namespace
(`target/debug/ultragoal --root . namespace check --strict --no-write --jobs 8`),
standards (`scripts/check-agent-standards --root .`), and four-persona exposure
for product/signoff/release/completion (blocked). No lane is selectable until every
gate is current and a run-scoped lease is issued.

The lease record binds lease id, the exact clean current base commit/tree,
branch, worktree, upstream identities, consumed files/symbols/generated
outputs/fixtures/effects, owned
files/symbols, generated outputs, fixtures, effects, forbidden roots (including
the shared root plus exact `.codex`, `.agents`, and linked-worktree `.git`
metadata files, resolved gitdir targets, and Git refs only), protected-root
patterns, normalization/symlink resolution, positive/negative overlap examples,
and a fail-closed overlap rule. It also binds isolated
HOME/CARGO/RUSTUP/target/tmp/cache/port roots, clean committed handoff,
reachable tip, ready receipt, and teardown/cache/evidence retention. Creation is
branch-first/worktree-second.

N01, N02, and N03 are serial; N02 requires a fresh same-session rebuild, zero
blockers, and two byte-identical inventories, and no inventory authority or
claim promotion may occur while it is open. N04-N07 then use four separate
worktrees; N08/N09 and N10/N11 may run in parallel. N12-N17 are root-serialized.
N14 invalidates N12/N13 and affected N02/N04/N08/N09 package, discovery, and
inventory evidence; rerun affected surfaces, then N12 B and N13 B.

N08, N09, and N11 consumed sets include files, symbols, generated output,
fixtures, effects, forbidden surfaces, tools, contract, and context. N09 live
proof is root-owned and must separately bind package, install, cache, app
registry, new-session discovery, tool execution, and host invalidation.

## Worktree protocol after P0

Use Codex-managed worktree tasks for substantial write streams. Start every
task from the exact integrated root commit, use a unique `codex/` branch, and
assign disjoint paths, semantic authority, generated output, fixtures, and
effects. Worktree tasks may use their own reviewer/fixer subagents, but the
worktree session remains local integrator and cannot claim acceptance.

Every handoff is a clean committed freeze containing base, head, tree, changed
paths, behavior, dependency assumptions, focused positive/negative/race/
recovery/security/false-pass evidence, unsupported scope, root requests, and a
single WorkerResult only after source acceptance. It explicitly disclaims
readiness, release, and completion.

Root reviews and integrates one authority-bearing increment at a time, applies
shared wiring, reruns the affected closure, refreshes dependents when consumed
interfaces change, and removes accepted worktrees and disposable caches after
unique state is gone. Never rebase or repair a dirty worktree externally.

## Audit cadence

| Boundary | Required evidence | Claims withheld before pass |
| --- | --- | --- |
| Implementation | Cheap deterministic checks and focused changed behavior | Source acceptance |
| Worktree freeze | Exact identity; complete named invariant review; focused failure, race, recovery, security, and false-pass controls | Dependency eligibility and integration |
| Root integration | Affected strict compile partition, cross-lane behavior, namespaces, generated authority, docs, and claim-relevant coverage | Integrated dependency and strict claims |
| Product freeze | Exact package/install/cache/registry/discovery/runtime identities; Product Success/Fitness/Cohesion; two real repositories when broad reuse is claimed | Installed, runtime, daily-driver, mastery |
| Release freeze | Full standards/coverage, clean-room distribution, migration/retirement, representative journeys, four-persona signoff, release proof | Readiness and release |
| Completion freeze | Final committed candidate, two byte-identical inventories, final falsification and requirement-to-evidence audit | Completion and goal completion |

Deferred audits remain in `VERIFICATION_BACKLOG.json` with trigger, owner,
candidate identity, evidence surface, and withheld claim. A delayed audit never
promotes a claim. Receipts and build artifacts are retained only while they
support an active claim, audit, or recovery need; reproducible caches and
superseded evidence are removed at integration boundaries.

## Current state and next actions

- P0 canonical registry/projections retain literal checkpoint
  `97e24c9706e7b489bdbdc6184ff9520a7116c6fd` / tree
  `c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e`; the clean containing candidate
  must prove permitted paths and status live. All acceptance evidence is
  historical/nonselectable and all 14 claims remain withheld.
- Global strict compilation still fails on the inherited warning wall; no
  suppression or dummy reachability is allowed.
- No implementation worktree launches until the canonical P0 checkpoint is
  committed and the semantic-namespace/standards debt checkpoint is clean.
- After P0, re-observe N02/N03 and select the smallest dependency-closed source
  or live-product stream that moves the installed journey.

The nearest product milestone is one exact current-source journey:
source -> package -> install -> discovery -> repository fit -> dirty routine
work -> interruption/diagnosis/recovery -> repeat use. Each truth surface keeps
its own identity and claim ceiling. Product Fitness records only time to value,
human interventions, review rounds, recovery outcome, false pass/rejection when
observed, and retained artifact/cache cost; no metrics subsystem is added.

## Stop and escalation rules

Continue through ordinary defects, stale candidates, missing narrow evidence,
and environmental retryable failures. Stop only for destructive action without
authority, external writes or publication, secrets, unavailable required
access, contract-amendment conflict, or a material product/authority decision
with no safe default.

Completion requires every N00-N17 dependency, migration, retirement, claim,
journey, release surface, final inventory, and exact candidate reconciliation.
The active host goal remains active until that terminal condition is proved.
