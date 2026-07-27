# ExecPlan Law

`PLANS.md` defines stable planning rules. Current project state belongs in one
self-contained file under `docs/exec-plans/active/`.

## One milestone, one active plan

Each goal has one observable product milestone and one active ExecPlan. The
plan names:

- user outcome and non-goals;
- current facts, assumptions, and claim ceiling;
- exact owned, forbidden, and shared surfaces;
- dependency order and one integration owner;
- finite gates and proof tiers;
- commands or oracles that distinguish correct from incorrect behavior;
- cost, repair, stop, recovery, and teardown rules; and
- Tree decisions with no safe default.

Overlapping active plans are an orchestration defect. Remove superseded plans
from the active tree; Git retains history.

## Progress and restartability

The active plan is the current state record. It keeps these sections current:

- Purpose and observable outcome
- Progress
- Surprises and discoveries
- Decision log
- Context and orientation
- Lane map and dependency graph
- Concrete steps
- Validation and acceptance
- Idempotence and recovery
- Artifacts and retention
- Outcomes and retrospective

A future agent must be able to resume from the plan without chat, a stale
registry, or a wall of receipts.

## Parallelism and exclusive ownership

Use one lane for coupled work. Use parallel lanes only when:

1. each lane can progress without another lane's uncommitted state;
2. path ownership and semantic decisions do not overlap;
3. shared interfaces are frozen before fan-out;
4. every lane has a local oracle, budget, stop condition, and concise handoff;
5. all lanes consume the same root-frozen base; and
6. one root owner has reserved integration and verification capacity.

Root owns shared schemas, dependency files, public grammar and application
programming interfaces, migrations, effect authority, claim promotion, and
final acceptance. Workers request shared changes in their handoff.

Every launched lane binds:

- exact base commit and tree;
- exact shared-interface fields consumed;
- exclusive path and semantic ownership;
- forbidden and root-owned surfaces;
- local oracle and proof tier;
- relevant dependency identities;
- repair budget, cancellation lineage, and stop condition; and
- one commit-bound return envelope.

Lanes never consume, merge, cherry-pick, or coordinate through another lane's
unintegrated work. If a shared interface changes, root cancels only declared
consumers and issues a new version.

Root alone moves a lane through `defined`, `ready`, `active`, `accepted`,
`no_change`, `blocked`, `cancelled`, `merged`, or `waived`.

Fan-in happens once, in a deterministic order. The integrator checks ancestry
and ownership, rejects peer merges and semantic conflicts, applies root-only
wiring, runs integrated checks, freezes one candidate, and closes or cancels
every lane. A merged patch or worker summary is not integration proof.

## Finite proof tiers

Use the lowest tier able to falsify the current claim:

- **T0 Context / micro:** provenance and explicit currentness limit.
- **T1 Lane commit / standard:** exact base/head commit and tree, exclusive
  owned diff, focused changed-behavior checks, and clean handoff.
- **T2 Integrated candidate / standard:** deterministic fan-in, shared wiring,
  integrated checks, and exact candidate freeze.
- **T3 Consequential boundary / elevated:** explicit failure model, relevant
  negative or fault evidence, recovery or rollback where touched, and one
  focused independent review.
- **T4 Product milestone / critical for authorized effects:** exact installed
  candidate, authorized representative same-surface journey, and concise
  quality-in-use outcome.
- **T5 Release / critical:** fresh release-candidate identity, distribution,
  install, discovery, runtime, security, migration, rollback, and Tree
  approval.

Do not run release or completion gates on ordinary work. Do not re-review
byte-identical authority solely to seek another result.

## Evidence economy and invalidation

Persist evidence only for a named current claim, cross-process custody,
irreproducible observation, recovery need, or authorized release. T1 and T2
normally use commits, the active plan, and a concise handoff.

Retained evidence declares owner, claim, exact candidate or environment,
surface, maximum statement, consumed dependencies, oracle, invalidation
trigger, and deletion boundary.

Evidence invalidates only when its candidate or a declared consumed dependency
changes, a relevant environment identity changes, its explicit freshness
window expires, or contradictory same-surface evidence appears. Unrelated
lanes, unconsumed docs, branch movement preserving the commit/tree, and age
alone do not force regeneration.

Stale evidence loses authority but does not block unrelated work. Delete
superseded reproducible evidence after unique recovery value is ruled out.

## Model and cost routing

Use Luna for narrow deterministic work, Terra for ordinary engineering, and
Sol for ambiguous or high-risk judgment. Use Ultra only for genuinely
independent streams with disjoint ownership and explicit root fan-in capacity.

Every lane has a budget. Stop and replan after two failed repair attempts,
budget exhaustion, or evidence that coordination costs more than the saved
time. Escalate reasoning only for a named risk the lower route did not retire.

## Completion

Lane completion means its owned commit and local oracle are ready for fan-in.
Goal completion means the single milestone passes its gate on the exact
integrated candidate. Release, repeated-use, daily-driver, and broad product
success claims require later authority and evidence.
