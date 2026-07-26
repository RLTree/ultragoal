# Harness Ultragoal Usable Product Goal

## Authority

This contract is the current product-delivery authority for Harness Ultragoal.
Tree authorized the reset on 2026-07-25 to replace the recursive
`harness-ultragoal-successor-contract-v2` operating loop with a finite path to
one usable product milestone.

The v2 handoff bundle, `LANE_REGISTRY.json`, `VERIFICATION_BACKLOG.json`,
`COMPLETION_MANIFEST.json`, `docs/ultragoal-successor-live/`, and the legacy
mandatory-law receipt projections under `agent-standards/policy/`,
`agent-standards/enforcement.*`, and `docs/mandatory-law-surfaces.*` are frozen
compatibility and migration inputs. They do not schedule work, require refresh,
promote claims, or block ordinary delivery. Their self-declared authority is
superseded by this contract, the scope reset in `AMEND-002`, and the strict
lane/proof decomposition in `AMEND-003`. Current product behavior and protected
invariants still require direct baseline inspection. Remove legacy inputs only
after the retirement lane proves that no current reader, compatibility promise,
unique recovery state, or active claim needs them.

The only active plan is
`docs/exec-plans/active/usable-product-milestone.md`. If another active plan
conflicts with it, stop and resolve the duplicate rather than combining both.

## Outcome contract

- **User:** a repository operator using the Harness Ultragoal Codex plugin and
  typed Rust command-line interface (CLI).
- **Job:** safely fit a repository, run useful affected work on a dirty tree,
  diagnose a representative failure, recover, and continue without losing
  unrelated work.
- **Product outcome:** the operator reaches a useful verified result through
  one understandable flow with bounded supervision.
- **Mission outcome:** reduce operator attention and delivery risk without
  replacing product work with governance or evidence maintenance.
- **Current decision:** determine and close only the gaps that prevent the
  usable product milestone below.

## Single milestone

`CL-USABLE-LOOP` is the only claim this goal advances.

The claim passes only when one exact integrated candidate completes this
sequence on a Tree-selected, representative, non-toy repository:

1. build the current-source plugin package;
2. install it in an explicitly authorized local scope;
3. observe the same bytes through supported host discovery;
4. enter through the documented front door;
5. inspect and fit the repository without changing unrelated state;
6. run a useful dirty-tree affected check;
7. encounter one representative failure;
8. receive a causal diagnosis and exact safe next action;
9. recover or refuse safely with state preserved; and
10. repeat useful work without stale custody, hidden writes, or a new
    governance cycle.

Acceptance also requires:

- no unapproved external, destructive, credential, publishing, or production
  effect;
- unchanged unrelated tracked and untracked repository state;
- one current candidate identity across source, package, install, discovery,
  and runtime observations;
- the operator can identify what ran, what did not run, why, and what remains;
- time to verified value, human interventions, recovery outcome, and retained
  artifact cost are recorded in the milestone outcome, not a telemetry system;
- all required evidence remains within the proof-tier ceiling defined below.

This milestone does not prove public release, marketplace availability,
universal host support, repeated adoption, daily-driver status, or completion
of every historical v2 claim.

## Observed facts, assumptions, and decisions

### Observed facts

- The repository contains a Codex plugin surface and a typed Rust CLI.
- The public architecture already separates source, package, install,
  discovery, runtime, journey, and release truth surfaces.
- Historical source work exists for repository fit, routine work, distribution,
  diagnosis, recovery, and orchestration, but its receipts are not current
  proof for this candidate.
- The prior operating model has 18 blocked lanes, 14 withheld claim rows,
  multiple active plans, recurring receipt refresh, and reproof cycles that can
  run without advancing a user outcome.
- Current installed, host-discovery, representative-runtime, product-journey,
  and release state is unknown until freshly observed.

### Assumptions to test first

- Some milestone behavior may already exist and need only integration or
  routing. The baseline may return `no_change`, `partial_change`,
  `change_required`, or `blocked` per surface.
- Four disjoint implementation lanes are enough after root freezes their shared
  interfaces. More lanes are not justified unless the integration owner records
  a concrete coupling reduction.
- One representative repository is sufficient for this milestone. Broader
  reuse or repeated-use claims require later evidence.

### Product decisions

- One milestone replaces the 14-claim completion graph for current delivery.
- One active ExecPlan replaces the eight overlapping active plans.
- Ordinary implementation uses focused ephemeral checks. High-risk boundaries
  and release claims use stronger proof only when those claims are current.
- There is one root-owned fan-in. No worker writes shared public grammar,
  manifests, shared schemas, dependency files, or claim decisions.
- Every launched lane consumes the same root-frozen `BASE-0` commit and
  `IFACE-0` interface contract. Lanes never consume or merge one another's
  unintegrated work.
- Lane proof is bound to exact base/head commits and trees, owned paths,
  consumed dependencies, and a local oracle. The commit plus concise handoff is
  sufficient unless the observation is irreproducible or must cross a custody
  boundary.
- Stale evidence loses authority; it does not trigger regeneration or block an
  unrelated change.

## Protected invariants

These invariants apply at every assurance level:

- authorize before effects;
- parse untrusted input before product behavior;
- preserve unrelated user work;
- keep read, help, inspect, diagnose, and next-action routes free of hidden
  writes;
- bind effectful work to explicit repository, candidate, and authority;
- fail closed on path escape, ambiguous ownership, stale custody, or possible
  post-effect ambiguity;
- make interruption, cancellation, retry, and recovery explicit where touched;
- never let a worker, model output, receipt, generated row, or reviewer mint
  root authority or raise a claim ceiling;
- keep private paths, secrets, prompts, transcripts, credentials, and raw host
  output out of durable evidence.

## Seven-layer ownership

| Layer | Durable owner for this goal |
| --- | --- |
| Intent and specification | This contract and the single active ExecPlan |
| Context | `AGENTS.md`, routed standards, `ARCHITECTURE.md`, and current source |
| Harness | Typed tool, permission, sandbox, run-state, and recovery boundaries |
| Loop | A bounded act-check-repair loop with the repair budgets below |
| Graph | The lane map and Mermaid dependency graph in the active ExecPlan |
| System | Root integration owner, Tree approvals, host, repository, and tools |
| Agent-first delivery | Disjoint worktrees, focused tests, one fan-in, and one milestone outcome |

No separate receipt graph owns a layer.

## Finite gate hierarchy

| Gate | Applies when | Required result | Not required |
| --- | --- | --- | --- |
| G0 Current-behavior baseline | Once, before implementation lanes | Surface-by-surface `no_change`, `partial_change`, `change_required`, or `blocked`; shared interface freeze | Full audit, material review, durable receipt |
| G1 Lane acceptance | Every implementation lane | Exact base/head commit and tree, exclusive owned diff, focused changed-behavior checks, failure-path checks where touched, clean concise handoff | Full repository gate, four-persona review, package or runtime proof |
| G2 Boundary proof | A lane changes security, authority, custody, concurrency, recovery, migration, or external effects | Explicit failure model, relevant negative/fault evidence, one independent focused review | Unrelated claim refresh or broad receipt regeneration |
| G3 Root fan-in | Once after required lanes are accepted | Dependency-order merge, shared wiring by root, integrated checks, conflict and completeness review | Worker-result aggregation as proof |
| G4 Usable product milestone | Once on the exact integrated candidate | Authorized install/discovery/runtime journey and `CL-USABLE-LOOP` outcome | Release, publication, repeated-use, or mastery proof |
| G5 Release | Only after Tree explicitly chooses release | Fresh release-grade package, install, discovery, runtime, migration, security, and rollback evidence | Automatic continuation from G4 |

A failed gate repairs the smallest owning boundary and reruns that gate. It does
not reopen earlier accepted work unless the failed change intersects that work's
declared inputs or contract.

## Proof tiers

| Tier | Assurance profile | Maximum supported statement |
| --- | --- | --- |
| T0 Context | Micro | A document, historical artifact, or unverified observation exists |
| T1 Lane commit | Standard | One exact owned commit satisfies its frozen local contract and focused oracle |
| T2 Integrated candidate | Standard | Required accepted or waived lanes compose at the single root fan-in and integrated checks pass |
| T3 Consequential boundary | Elevated | The named security, authority, custody, concurrency, recovery, migration, install, host, or external-effect boundary is supported by focused negative/fault evidence and independent review |
| T4 Same-surface product | Critical for authorized effects | The exact installed candidate completes the authorized representative journey and supports `CL-USABLE-LOOP` only |
| T5 Release | Critical | The exact release candidate passes release-specific evidence and Tree approval for the explicitly authorized release claim |

Higher tiers do not erase a failure at a lower authoritative boundary. Test
volume, coverage percentage, receipt existence, or reviewer agreement cannot
substitute for the named surface.

## Evidence retention and invalidation

Keep observations ephemeral by default. Persist evidence only for:

1. the current `CL-USABLE-LOOP` decision;
2. an independent cross-process handoff that cannot be reconstructed cheaply;
3. an irreproducible external or host observation;
4. security, custody, or recovery state needed to resume safely; or
5. a Tree-authorized release decision.

Each retained item names its claim, owner, candidate or environment binding,
invalidation rule, and deletion boundary. Use one item per claim and proof
surface; do not create receipts of receipts, per-command mirrors, or periodic
refresh copies.

Every accepted proof, retained or ephemeral, names the candidate commit and
tree, proof surface, maximum claim, consumed authority/dependency set, oracle,
and environment identity only when the behavior depends on it.

Evidence invalidates only when:

- its candidate commit/tree, an authority-bearing byte, or a declared consumed
  dependency changes;
- the relevant installed/runtime environment identity changes;
- an external fact with a stated freshness window expires; or
- a contradictory same-surface observation appears.

Unrelated lanes, unconsumed documentation, timestamps, branch movement that
preserves the exact candidate, or the mere age of reproducible local output do
not invalidate evidence. Rerun only proofs that declare the changed dependency.
Superseded or detached artifacts lose authority immediately and are deleted at
the next safe integration or teardown boundary after unique recovery value is
ruled out. Git history is the archive.

## Model and orchestration cost policy

- **Luna:** narrow, repeatable, easy-to-grade transformations with one check.
- **Terra:** ordinary implementation, focused repair, and routine analysis.
- **Sol:** ambiguous cross-boundary design, security/authority judgment, root
  integration, or independent falsification.
- **Sol with Ultra:** only when two or more ready work packages are genuinely
  independent, path ownership is disjoint, and the root has reserved fan-in
  capacity.

Use the lowest reasoning effort that passes a representative check. Each lane
sets a wall-time or token budget and one repair budget before launch. Stop
parallel work when coordination, duplicated investigation, or integration cost
exceeds the saved critical-path time. Do not encode model prestige into policy.

## Stop conditions

Stop the affected boundary for:

- missing Tree authority for local install, target-repository mutation,
  destructive action, external write, publication, credential use, or release;
- an unresolved shared-interface or ownership conflict;
- possible secret or private-data exposure;
- ambiguous post-effect state that cannot be reconciled safely;
- two failed repair attempts without a new causal hypothesis;
- a lane exceeding its budget without evidence that the next attempt changes
  the failure mode; or
- a material product, scope, or risk-acceptance decision with no safe default.

Do not stop independent legal work because another lane is blocked. Do not
continue a self-governance or evidence-refresh loop merely because an old
artifact says it is due.

## Tree decisions

No Tree decision is required for the planning reset or read-only baseline. If
the baseline exposes a product-value, scope, risk, or shared-interface choice
with no safe default, only the affected lanes stop for Tree. Before G4, Tree
must select the representative repository and authorize the exact local install
and repository-write scope. After G4, Tree decides whether to stop at the
usable-product milestone, run a second materially different journey, or
authorize G5 release work. Publishing, marketplace changes, credentials, and
destructive retirement always require separate explicit authority.

## Current claim ceiling

This planning reset supports only a current contract and executable delivery
plan. It does not establish source correctness, package identity, installation,
host discovery, runtime behavior, product fitness, release readiness, or
`CL-USABLE-LOOP`. Those claims remain unavailable until their finite gate
passes on the exact candidate.
