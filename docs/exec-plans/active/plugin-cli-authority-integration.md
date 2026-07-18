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
receipts. The user-authorized cleanup removed the obsolete review tree and
reproducible worktree home/tmp state, recovering about 10.1 GB in addition to
earlier cleanup; the three approved paths now exist only as empty roots. No
unique active-worktree state remains.

## Outcome

Complete the plugin-led daily-driver product and typed Rust CLI kernel from
current repository truth. Preserve accepted source work while proving source,
package, install, cache, marketplace, app registry, discovery, runtime,
product, migration, release, and completion claims on their own surfaces.

The current P0 candidate is always the clean containing `HEAD`/tree. The
literal checkpoint above must remain an ancestor with its exact tree; the
freeze derives live HEAD/tree, permitted root paths, payload digests,
projection chain, and clean status. The exact P0 source candidate
`ed318ce541684489d017dcfd3363501337290dab` / tree
`bc9354969b5fe47e539bdeccccec7918a2c0045b` passes the warning-free
production-library check, 121-row standards entrypoint, authored-Rust line cap,
and the current namespace strict route with zero findings and recursive
zero-write comparison. One exhaustive independent review returned ACCEPT after
the namespace inventory path was made fail-closed. This closes only the P0
compile/namespace/standards checkpoint and makes N00 the next root-only gate.
The full clippy wall and library-test compilation still have broad inherited
findings, so CL-STRICT and every package/install/runtime/product claim remain
withheld. Four-persona exposure remains reserved for its material boundary.

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
Sol/medium on the standard service tier; bounded repair uses the lowest-sufficient
standard-tier patch route, raising reasoning only for a named invariant. Cached
checks run during repair. Builds, strict checks, and broad audits run only at the
decision boundary they can support. Routine lane acceptance uses one
risk-matched specialist for one exhaustive invariant pass. The repo-defined
four-persona round is reserved for material or major root integration,
consequential cross-domain milestones, and product, release, or completion
signoff; until installed active-registry exposure is current, those milestone
claims stay withheld.

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

The root records prelaunch gates with owner, command, status, and the derived
clean candidate: compile (current at the accepted P0 source freeze), namespace
(`target/debug/ultragoal --root . check strict --claim namespace-progressive-disclosure`),
and standards (`scripts/check-agent-standards .`). All three are current at the
accepted P0 source freeze. The namespace adapter
evaluates current governed source and plugin-interface names without consuming
the N02-owned package manifest closure. Four-persona exposure
is a product/release/completion gate, not a prelaunch lane-selection gate. No
lane is selectable until every applicable prelaunch gate is current and a
run-scoped lease is issued.

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
| Worktree freeze | Exact identity; one risk-matched specialist completes the named invariant; focused failure, race, recovery, security, and false-pass controls | Dependency eligibility and integration |
| Root integration | Affected strict compile partition, cross-lane behavior, namespaces, generated authority, docs, and claim-relevant coverage | Integrated dependency and strict claims |
| Product freeze | Exact package/install/cache/registry/discovery/runtime identities; one canonical Product Fitness receipt; Product Success/Fitness/Cohesion; four-persona signoff; two real repositories when broad reuse is claimed | Installed, runtime, daily-driver, mastery |
| Release freeze | Full standards/coverage, clean-room distribution, migration/retirement, representative journeys, four-persona signoff, release proof | Readiness and release |
| Completion freeze | Final committed candidate, two byte-identical inventories, final falsification and requirement-to-evidence audit | Completion and goal completion |

Deferred audits remain in `VERIFICATION_BACKLOG.json` with trigger, owner,
candidate identity, evidence surface, and withheld claim. A delayed audit never
promotes a claim. Receipts and build artifacts are retained only while they
support an active claim, audit, or recovery need; reproducible caches and
superseded evidence are removed at integration boundaries.

Observability follows applicability rather than artifact volume. Every
law-bearing command emits a typed redacted diagnostic envelope; logs are
required for durable effects or reconstruction, metrics for aggregate rate or
capacity claims, traces for multi-step or cross-process causality, and evals
for model-behavior claims. All four join only at the observability capability
claim and representative product or release proof.

## Current state and next actions

- P0 canonical registry/projections retain literal checkpoint
  `97e24c9706e7b489bdbdc6184ff9520a7116c6fd` / tree
  `c1d0cc65ffce60e4d917e14cb8b9ac4664d71a3e`; the clean containing candidate
  must prove permitted paths and status live. P0 source acceptance is current
  only for compile, namespace, standards, line-cap, and zero-write enforcement;
  all 14 product claims remain withheld.
- The disconnected legacy red execution engine and target-repository audit are
  retired. The disconnected legacy claim, semantic-receipt, and material-review
  authority was then removed in `a11dcac01`, including its test-only dispatch
  and receipt machinery. Root reconciliation removed stale reader witnesses and
  public test dispatch. Follow-up commits `549770c96` and `7799d5973` retired
  disconnected validator-receipt writers, audit artifact aggregation, and
  authority-inventory projections. The affected production-library check now
  reports 64 deny-warning errors, down from 664, with no unresolved reference
  to the retired claim/review or audit-writer graphs. N09's 194-diagnostic
  discovery partition is now genuinely production-reachable through
  `inspect capabilities --package-root <host-path>`; exact package and `HOME`
  roots are validated before discovery, and invalid roots prove zero authority
  I/O. This raises only a source-local public-observation ceiling: host
  discovery, runtime exposure, package/install/cache identity, and claims remain
  withheld. Scoped OD-008 authority now covers exactly 12 dormant internal
  `claim_semantics/lane/**` paths plus `ready/mod.rs` and `ready/receipt.rs`.
  The private, unreachable cohort has no compatibility window: its 14 routes
  may demote to retained non-authoritative context only while the exact decision,
  production detachment, stable IDs, source digests, transition tuple, and
  HCT-CLAIMS target remain current. The five legacy-only helper mechanisms were
  retired with that production graph. This closes the 64-warning production
  frontier on the scoped source candidate without deleting archived bytes.
  Global OD-008 remains unresolved for every other route, OD-009 still forbids
  physical deletion or movement, and the remaining N14 route adoption,
  equivalence, host execution, and registry closure are withheld. Exact candidate
  `bd7eb339e` / tree `558f0f6dc` passed the bounded independent review after its
  root digest chain and pending-source binding were repaired. The surviving coverage-digest
  authority is now owned by `audit/coverage/scope/digests.rs`, outside the
  retirement graph; all Rust callers, both coverage manifests, the package
  manifest, and the five valid package fixtures were rebound to that path. The
  production diagnostic frontier remains exactly the same 64 errors, with no
  new source diagnostic. Focused coverage test execution is withheld because
  the library-test target currently fails compilation on 319 pre-existing
  retired-module/import errors. The safe audit cleanup separately
  retired the test-only clock formatter and redundant final-packet pass wrapper.
  The latest bounded cleanup removed four definition-only schema
  adapters and unreachable scheduler variants/projections while preserving the
  live schema store and deterministic pure-read scheduler. The next item-level
  cleanup removed an unused command-artifact dialect and an unread skill-link
  diagnostic field without changing the surviving artifact or skill-link
  validators. The latest coherent retirement removed 3,651 lines of orphaned
  observability-registry implementation and tests that were physically present
  but not compiled, plus the two remaining live wrappers that exposed that
  retired registry. The manifest and raw projection catalog now name only the
  surviving fail-closed registry surface. The test-only JSON value-projection
  variants of the text guards were then retired while preserving the four live
  file-backed self-law checks. Package schema validation now has one live
  scheduler-backed implementation instead of two parallel copies; unused
  targeted-schema reporting and scheduler-metric projections were retired. The
  audit surface then dropped an unconsumed semantic-receipt Rust model whose
  schema-owned validation remains live, plus test-only product and standards
  projection adapters that duplicated the file-backed production routes. The
  legacy help catalog now delegates to the typed successor catalog, and
  duplicate template, registry, Product Fitness, and bytecode projections were
  removed while their canonical file-backed checks remain live. The
  production-dead generic JSON/output mutation chain was then retired; generic
  JSON writes now exist only in the test fixture boundary, while the two
  governed claim-artifact path functions remain the sole production surface.
  Commit `a558dfaf9a58d4ddb9f9dcb818927acbd42bd7d3` then closed the process-mediator
  semantic-namespace debt by moving failure injection, sandbox profiles,
  termination status, observation digests, and observation failures into
  behavior-named leaves. The exact decision-boundary production check still
  reports the same 64 N14-blocked diagnostics and no new diagnostic from that
  move. This is not a compile pass; no suppression or dummy reachability is
  allowed.
- Root reconciliation revoked the stale P0 lease, removed retired
  authority-reconciliation paths from the protected checkpoint, rebound the
  surviving plan and claim-module digests, and settled the verification backlog
  and completion manifest through one explicit reproducible registry-preimage
  rule. All claims remain withheld, and no replacement tracker or receipt was
  created. The subsequent root-owned debt repair is accepted at
  `ed318ce541684489d017dcfd3363501337290dab` / tree
  `bc9354969b5fe47e539bdeccccec7918a2c0045b`: compile, namespace, standards,
  authored-file limits, and zero-write enforcement pass, including fail-closed
  namespace inventory. No P0 lease or receipt was manufactured after the fact.
  N00 adoption is accepted on exact clean root candidate `8f789de6a940b5a2b600d3021ac803b0b62ff341`
  / tree `3abfb13a11e529a8af8e96770b2e2392c8658ef1`. The new epoch binds the
  immutable contract bundle and graph, exact N00 source identity, scheduler
  eligibility, lane states, and source ceilings. N01 is the sole serial gate;
  N02-N11 remain blocked and require adopted-current reobservation.
- HCT-FIXTURES execution/parity, runtime, installed journey, readiness,
  release, and completion claims remain withheld.
- The calibrated review, observability, and Product Fitness doctrine is now
  projected through the canonical standards sources and templates. The
  disconnected legacy material-review validator is not treated as production
  enforcement; its bounded-invariant decision must be adopted through a live
  successor route or retired before an enforcement claim is raised.
- A single bounded synthesis of the July model-routing and product-craft source
  corpus, followed by one bounded Sol/Max adversarial crafting pass, found no
  monotonic contract change: `AMEND-001` already owns the doctrine. The existing
  front door, goal-run, maintenance, journey-review, and resource-map surfaces
  now calibrate the next value-bearing action, representative acceptance check,
  lowest sufficient exposed route, review scope, proof boundary, and persistence
  decision. This is ephemeral guidance over the existing registry and claim
  kernel, not scheduler or state authority. Actual `next` CLI behavior remains
  deferred to its existing N08/N09 dependency boundary. No guide, node, tracker,
  schema, receipt family, or observability channel was added. Installed cache
  guidance remains an unverified stale projection until the current source is
  packaged, installed, and observed on those separate surfaces.
- `scripts/check-agent-standards .` passes 121 rows after stale pass claims
  were downgraded to pending rather than manufacturing replacement evidence.
  The corresponding performance, Rust developer-loop, observability,
  improvement-loop, cohesion, and cache claims remain withheld until their
  real evidence surfaces are refreshed.
- The retired target-repository audit is removed from covered mandatory-law,
  trace, standards, schema, fixture, and package projections. Its only current
  authority is the blocked source-obligation row for the missing typed HCT-FIT
  and HCT-FIXTURES mapping; target capability and product claims remain
  withheld.
- No implementation worktree launches while the root-only N01-N03 serial
  authority chain is open; P0 and N00 are clean.
- Re-observe N01 now, then N02/N03 in dependency order. After that boundary,
  select the smallest dependency-closed source or live-product streams that
  move the installed journey without overlapping authority.

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
