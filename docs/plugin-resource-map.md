# Harness Ultragoal Plugin Resource Map

This is the source-level product map. It defines one preferred entry and seven
specialized workflows so an operator does not need to remember an internal
sequence of lanes, gates, receipts, or helper scripts.

## Canonical skill topology

`$harness-ultragoal:harness-ultragoal` is the only front door. It selects
exactly one primary route:

| Immediate outcome | Canonical skill | First capability probe | Effect ceiling |
| --- | --- | --- | --- |
| Classify an unknown request | `harness-ultragoal` | `ultragoal --json inspect capabilities` | Read |
| Set up a fresh repository or retrofit an existing one | `repository-fit` | `ultragoal --json fit inspect --target <relative-path>` | Read until an accepted apply plan |
| Run affected checks while preserving a dirty tree | `routine-work` | `ultragoal --json check routine --target <relative-path>` | Declared local `WorkspaceWrite` |
| Explain a failure, query local events, or choose the next action | `diagnose-and-observe` | `ultragoal --json inspect findings` | Read; export is separately approved `ExternalWrite` |
| Coordinate durable multi-scope work and recover it after interruption | `goal-run` | `ultragoal --json inspect context` | Per-work-package effects only |
| Prove one named claim at its exact truth surface | `prove` | `ultragoal --json inspect claims` | Declared proof output `WorkspaceWrite` |
| Evaluate, research, migrate, preserve compatibility, or retire | `improve-and-maintain` | `ultragoal --json eval audit --spec <relative-path>` or `migrate plan` | Read until an accepted bounded operation |
| Independently falsify a real operator journey | `product-journey-review` | Probe every required command and host surface | Read-only reviewer |

The old skill names may remain as non-authoritative compatibility sources until
the root-owned migration registry retires them. They are not preferred product
routes and must never be selected as a fallback when a canonical capability is
missing.

## Deterministic selection

Select the skill that owns the requested immediate outcome:

1. A request to independently review or falsify an existing journey selects
   `product-journey-review`.
2. A request to prove a named claim selects `prove`.
3. A symptom, finding, failure, query, repair plan, or next-action request
   selects `diagnose-and-observe`.
4. Fresh setup, retrofit, partial fitting, or an ownership conflict selects
   `repository-fit`.
5. Affected tests, changed-impact validation, or safe reuse selects
   `routine-work`.
6. Coordinated execution across multiple dependency-bound scopes selects
   `goal-run`.
7. Evaluation, research refresh, compatibility, migration, or retirement
   selects `improve-and-maintain`.

When one prompt contains multiple outcomes, choose the earliest outcome the
operator must complete and name the others as follow-ons. Do not merge skill
authorities. If two outcomes are truly simultaneous and require coordination,
select `goal-run`, then create disjoint work packages that invoke the owning
specialized workflows. If a repository-dependent route lacks a target, or a
write route lacks required authority, stop before execution and ask one focused
question. Unknown intents produce no route.

## Representative journeys

### Fresh repository

1. Enter through `harness-ultragoal` and select `repository-fit`.
2. Run `fit inspect` and `fit plan` without writes.
3. Present every mutation, preservation rule, conflict, rollback, and effect.
4. Run `fit apply` only after acceptance of the unchanged current plan.
5. Run `fit verify`, then route actual routine work to `routine-work`.

### Partial retrofit or conflicting authority

Use `repository-fit`. Classify existing owners and preserve all unrelated
modified, staged, untracked, and worktree state. Conflicts remain findings;
they are never resolved by preference or hidden behind generated output.

### Routine repeat use

Use `routine-work`. Recompute changed impact, verify every reuse key against the
current candidate and environment, run only the dependency-closed affected set,
and compare the full declared workspace boundary before and after. A strict
claim boundary follows through `prove`; routine work does not become release
ceremony.

### Failure and diagnosis

Use `diagnose-and-observe`. Inspect the current candidate and findings, query
bounded local events when available, derive one causal explanation, and return
one legal next action. The selected repair executes only through the workflow
that owns its effect.

### Interrupted orchestration

Use `goal-run`. Recompute candidate identity, leases, worker results, reviews,
and preserved state. Reconcile accepted work at the root, reissue stale work,
and advance independent ready work. Parallel activity does not prove recovery.

### Strict proof

Use `prove` for one claim. Bind prerequisites, positive behavior checks,
false-pass controls, outputs, and an independent reconciler to the current
candidate. Root authority alone decides claim promotion.

### Improvement and migration

Use `improve-and-maintain`. Audit tasks and scorers before evaluation. Treat
research and metric gains as candidates. Inventory active readers and writers,
verify replacement behavior, preserve explicitly adopted compatibility, and
require destructive approval before retirement.

## Current project-scoped agents

The six current read-only roles live under `.codex/agents/`. Their presence in
source does not prove package inclusion or host discovery.

| Agent | Use |
| --- | --- |
| `repo-recon` | Recompute repository, worktree, command, component, and candidate truth. |
| `research-verifier` | Recheck mutable primary-source and capability facts. |
| `product-journey-reviewer` | Falsify acquisition-through-completion and quality-in-use journeys. |
| `claim-falsifier` | Attack prerequisites, wrong surfaces, stale evidence, guards, and ceilings. |
| `security-reviewer` | Attack confinement, effects, secrets, permissions, and supply chain. |
| `orchestration-recovery-reviewer` | Attack leases, dependency closure, interruption, reconciliation, and recovery. |

Do not copy these files into a global agent directory as part of source setup.
Use project-scoped discovery when the current host exposes it. If the host does
not expose the named role in the current task, record discovery as unsupported
and lower the dependent review ceiling.

## Human attention policy

The plugin handles deterministic routing, capability probes, read-only
inspection, causal diagnosis, safe retry planning, and independent-work
advancement. Interrupt the operator only for a missing choice that changes the
product outcome or authority, an external write, an unavailable required
access, a secret boundary, or a destructive decision. The interruption names
the exhausted safe routes, the exact blocker, preserved state, and the exact
next action.

## Truth surfaces

Keep these surfaces separate and candidate-bound:

1. source coherence;
2. deterministic package bytes and package inventory;
3. repository or personal marketplace catalog observation;
4. installed bytes;
5. cache identity;
6. app registry and Plugins UI observation;
7. new-task discovery;
8. representative runtime behavior;
9. repository and product journeys;
10. release and completion.

A lower surface never proves a higher one. Help, parse, inspect, query,
diagnose, and next-action selection must also prove zero hidden writes at the
same recursive tree and Git-status scope as the command.

The source-candidate freeze includes these docs, the eight canonical skills,
the route and journey fixtures, their semantic tests, the plugin descriptor,
the package manifest, and `validator/src/cli/successor/catalog.rs`. Root-only
agent descriptors and marketplace absence enter only through root-rederived
metadata. Either kind of input invalidates the full extension when it changes;
the issued lease candidate identity and the worker artifact aggregate remain
separate fields.
