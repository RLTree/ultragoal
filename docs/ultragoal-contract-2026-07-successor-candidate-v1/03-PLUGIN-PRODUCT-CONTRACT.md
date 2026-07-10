# Plugin Product Contract

## Role

The plugin is the product’s human- and agent-facing operating system. It provides the front door, guidance, role selection, repository fitting, product journeys, and progressive disclosure. The CLI supports and enforces it; the plugin is not packaging around the CLI.

Plugin success is measured by whether an intended user or agent can complete real journeys accurately and recover from failure, not by the number of bundled resources.

## Component architecture

### Canonical components

| Component | Product role | Authority |
|---|---|---|
| Marketplace metadata | Acquisition and version selection | Distribution truth only |
| `.codex-plugin/plugin.json` | Package identity and declared resources | Source/package declaration |
| Front-door skill | Intent intake, state check, journey routing | Guidance, not claim authority |
| Specialized skills | Progressive task procedures | Guidance constrained by product laws |
| Agent roles | Read-only reconnaissance, review, or bounded implementation expertise | Role contract, not completion authority |
| Templates | Initial source material and schemas | Inputs requiring reconciliation after use |
| Rust CLI/library | Typed decisions, findings, repair, and claim guards | Local authority kernel |
| Optional MCP/app tools | External-system access that cannot be owned locally | Explicit permission and availability boundary |
| Research registry | Source provenance and law proposals | Source authority by class |
| Maintenance assets | Upgrade, deprecation, migration, and cleanup guidance | Planned mutation only |

The plugin manifest MUST be generated from canonical component metadata where possible. The generated resource list MUST be reproducible, diffable, and verified against the packaged archive.

## Progressive disclosure

The installed plugin MUST expose a concise initial description that answers:

- what Harness Ultragoal does;
- when to enter through it;
- what it may read or write;
- how to inspect current state;
- how to get the exact next action.

Only the selected skill’s complete instructions and directly required references should enter the active task context. Specialized material MUST be routed by explicit trigger and product surface. Duplicate instructions across skills are forbidden unless one is generated from the canonical source.

Initial plugin metadata and skill descriptions MUST remain within the host’s current context-budget guidance. A validator MUST measure both aggregate discovery size and route ambiguity.

## Skill topology

The successor SHOULD consolidate current skills into these product-semantic capabilities:

| Capability | Replaces or absorbs | Primary result |
|---|---|---|
| `ultragoal` | General entry and historical macro-lane routing | Select a journey from current state |
| `fit-repository` | Init, retrofit, fit-repo, setup variants | Inspect, plan, apply, and verify fitting |
| `work-with-ultragoal` | Exec-plan and routine-loop guidance | Fast routine development and exact repair |
| `prove-with-ultragoal` | Proof gate, final packet, claim workflows | Strict surface-appropriate proof |
| `observe-and-diagnose` | Runtime legibility and observability guidance | Query, explain, and repair observed failures |
| `improve-ultragoal` | Standards gardening, research, product cohesion | Evidence-to-evaluation improvement loop |
| `maintain-ultragoal` | Garbage, migration, upgrade, retirement guidance | Safe, measured maintenance |

Names may change during implementation review, but there MUST be one canonical owner for each capability and no skill may advertise a stronger claim than its authority surface.

## Agent topology

Agent roles MUST be generated or validated from one canonical role definition. The minimum semantic roles are:

- `reconnaissance-reader` — read-only repository and runtime mapping;
- `product-journey-reviewer` — read-only usability and closure review;
- `claim-falsifier` — read-only/adversarial false-pass testing;
- `security-reviewer` — read-only authority, confinement, secret, and supply-chain analysis;
- `implementation-worker` — write-owning, bounded product package implementation;
- `recovery-reviewer` — read-only stale, drifting, blocked, and partial-worker diagnosis.

Roles may be specialized by work package, but role files MUST NOT pin a historical model name as product law. Runtime selection belongs to the orchestration environment and must be recorded as observed metadata when exposed.

## Plugin lifecycle journeys

### PJ-ACQUIRE-001 — Find the intended plugin

**Entry:** user has an approved marketplace, local package, or repository source.  
**Behavior:** show plugin name, semantic version, origin, compatibility, permissions, release integrity, and install route.  
**Proof:** reconcile marketplace entry to package identity and source/release provenance.  
**Failure:** ambiguous origin, version collision, missing compatibility, or unverifiable package.  
**Repair:** select an explicit origin/version and re-run acquisition verification.

### PJ-INSTALL-001 — Install and verify bytes

**Entry:** verified package snapshot exists.  
**Behavior:** install into the selected plugin home without lifecycle-script surprise, then compare installed bytes and metadata to the package snapshot.  
**Proof:** independent installed-tree snapshot plus host-recognized plugin identity.  
**Failure:** partial install, byte mismatch, unexpected files, wrong home, or stale cache.  
**Repair:** remove only the identified failed install through an authorized plan and reinstall the verified package.

### PJ-DISCOVER-001 — Discover the plugin in the application

**Entry:** installed bytes are reconciled.  
**Behavior:** the host registers the plugin, exposes its front-door skill, and routes a new task to the intended capability.  
**Proof:** application registry observation plus an independent real-task discovery probe.  
**Failure:** installed but unregistered, registered but hidden, stale app cache, route ambiguity, or source-only evidence.  
**Repair:** refresh the smallest relevant host surface and repeat the new-task probe.

### PJ-ENTER-001 — Understand the first action

**Entry:** plugin is discoverable.  
**Behavior:** a user can ask a normal product question without knowing internal gate or command names; the plugin identifies the repository, reads current state, states intended effects, and recommends one next action.  
**Proof:** scored journey replay with no hidden prerequisite knowledge.  
**Failure:** internal jargon, multiple conflicting commands, missing effect notice, or receipt scavenger hunt.

### PJ-FIT-FRESH-001 — Fit a new repository

**Entry:** repository has no active Harness Ultragoal contract.  
**Behavior:** inspect stack and constraints, propose minimal files, request authority for material choices, apply through explicit mutation, and verify loading plus routine behavior.  
**Proof:** before/after repository snapshot, generated-source reconciliation, host load, and routine journey.  
**Failure:** generic scaffold, destructive overwrite, environment-specific paths, or receipt-only success.

### PJ-FIT-RETROFIT-001 — Fit an existing repository

**Entry:** repository has existing instructions, build/test systems, local edits, or a prior harness.  
**Behavior:** classify ownership, preserve local policy, map conflicts, propose migration, make bounded edits, and prove compatibility.  
**Proof:** user-change preservation, contract merge decision, representative existing workflow, and rollback path.  
**Failure:** overwriting instructions, duplicating authorities, importing fixed lanes, or claiming success from file presence.

### PJ-ROUTINE-001 — Perform ordinary work quickly

**Entry:** repository is fitted.  
**Behavior:** compute affected surfaces from the current dirty tree, run the smallest legal checks, use only verified reuse, and return actionable findings quickly. Optional proof surfaces may be unavailable without breaking routine repair.  
**Proof:** changed-surface experiments, dirty-tree cases, cache hit/miss equivalence, and bounded latency distribution.  
**Failure:** accidental global audit, hidden artifact writes, stale cache pass, or optional service dependency.

### PJ-DIAGNOSE-001 — Explain a failure

**Entry:** command, agent, test, journey, or runtime failure exists.  
**Behavior:** query local semantic events and evidence references, explain the causal chain, distinguish missing from failing surfaces, and compile the smallest repair.  
**Proof:** known-fault injections and operator comprehension review.  
**Failure:** raw receipt dump, guessed cause, private-path leak, or repair that changes unrelated state.

### PJ-PROVE-001 — Run strict proof

**Entry:** a named claim is requested.  
**Behavior:** resolve its required surfaces, freeze an AuditContext, execute or verify same-candidate work, independently reconcile results, and return a ceiling.  
**Proof:** false-pass, tamper, stale, wrong-surface, no-cache, and interrupted-run cases.  
**Failure:** receipt existence, generated checklist pass, source proof substituted for runtime, or narrow subset labeled complete.

### PJ-ORCHESTRATE-001 — Complete multi-package work

**Entry:** current state reveals multiple dependent work packages.  
**Behavior:** the root Ultra agent builds an adaptive graph, separates read-only and write ownership, serializes shared authority, reviews and reconciles worker output, and recovers partial work.  
**Proof:** dependency, overlap, stale-worker, blocked-worker, drift, and interrupted-root simulations.  
**Failure:** fixed historical lanes, overlapping writes, worker self-acceptance, or split completion authority.

### PJ-IMPROVE-001 — Learn from real failure

**Entry:** reconciled trace, review, or user journey exposes a product failure.  
**Behavior:** harvest a proposed finding, validate evaluation data, scope a change, compare candidate behavior, independently review, and promote or reject.  
**Proof:** replayable failure-to-eval chain and a negative-control case.  
**Failure:** automatically converting every trace anomaly into law or using the candidate’s own score as proof.

### PJ-RESEARCH-001 — Refresh research and propose product law

**Entry:** a drift-prone source, host/model/tool change, unexplained product failure, or proposed product practice needs external authority.  
**Behavior:** search current primary sources, classify facts/advice/hypotheses, preserve canonical provenance, translate only relevant facts into a product principle and proposed law change, and route it through compatibility, validator, fixture, proof, claim-guard, and review work.  
**Proof:** an independent reader reaches the cited primary fact, the requirement trace closes, and no local synthesis or vendor assertion is promoted beyond its class.  
**Failure:** stale documentation, broken link, source card treated as primary evidence, advice silently made binding, or law changed without migration/product proof.  
**Repair:** refresh or reclassify the first unsupported source fact and lower every dependent claim until review/adoption.

### PJ-MAINTAIN-001 — Upgrade and retire safely

**Entry:** plugin or CLI version changes, cache grows, a command is deprecated, or authority is consolidated.  
**Behavior:** inventory affected state, propose migration, preserve rollback and user changes, verify new journeys, then retire obsolete surfaces.  
**Proof:** upgrade, downgrade/rollback, deprecated alias, cache cleanup, and partial-migration cases.  
**Failure:** deleting before replacement proof or leaving two claim authorities active.

### PJ-COMPLETE-001 — Complete the product goal honestly

**Entry:** requested goal and repository contract have explicit completion conditions.  
**Behavior:** root authority reconciles all required journeys, independent review, cleanup, unresolved risks, and exact candidate identity; it either emits the allowed completion candidate or stops with the next blocker.  
**Proof:** one real end-to-end journey suite plus adversarial incomplete candidates.  
**Failure:** green checklist, worker consensus, or historical receipt history used as completion.

## Repository-fitting contract

Fitting has four explicit phases:

1. `inspect` — read-only map of repository root, instructions, stack, build/test surfaces, current edits, existing harness assets, and conflicts.
2. `plan` — deterministic proposed operations with ownership, preconditions, effect class, rollback, and expected verification.
3. `apply` — explicit authorized writes only; never implicit in inspect/status/next/check.
4. `verify` — reload from disk and host context, exercise representative routine behavior, and reconcile generated material.

The fit engine MUST support partial adoption and MUST state the resulting claim ceiling. It MUST not require a clean worktree. It MUST distinguish files it owns, generates, shares, or merely observes.

## Installation and discovery proof ladder

1. source manifest valid;
2. package contents match canonical snapshot;
3. archive/release integrity verified;
4. installed tree matches package;
5. host cache references installed identity;
6. application registry exposes the plugin;
7. new task discovers the intended front door;
8. representative skill loads complete instructions;
9. representative journey executes against the fitted repository.

Each rung may support only its named claim. Higher rungs depend on, but are not implied by, lower rungs.

## MCP and external-tool boundary

Harness-owned semantics remain in the local Rust kernel. MCP or app tools are appropriate only when the product must access an external system, application registry, hosted service, or remote evidence that cannot be obtained locally.

Every external tool use MUST declare:

- why local substrate is insufficient;
- permission and data boundary;
- availability and timeout behavior;
- redaction and retention behavior;
- deterministic fallback or honest unavailability;
- which claim, if any, the result can support;
- independent reconciliation when completion-bearing.

An unavailable optional connector lowers only the claims that require it. It must not block unrelated routine work.

## Product guidance contract

Guidance MUST:

- lead with current truth and one next move;
- define unfamiliar terms at first use;
- expose command effects before writes;
- use product semantics, not historical gate numbers;
- distinguish required, advisory, experimental, and rejected ideas;
- state evidence and claim ceiling without forcing users to read raw JSON;
- provide machine-readable detail for agents and concise prose for operators;
- remain accurate when optional services are absent.

## Plugin observability

The plugin MUST emit or request events for journey selection, skill loading, external-tool boundary, fitting plan/apply/verify, routine checks, strict proof, worker handoff, review, recovery, improvement, and completion decision. Events MUST be correlated to a candidate and operation without recording raw prompts, secrets, or unnecessary repository paths.

Plugin guidance itself is an observable product surface: route chosen, instructions loaded, effect communicated, command proposed, result understood, and repair completed.

## Plugin maintenance and compatibility

- Component metadata, generated manifest, package, installed bytes, and host registry MUST share a reconciled version identity.
- Deprecated skills and commands MUST provide one versioned semantic alias and exact migration guidance, then be removed.
- Templates MUST declare schema/version and ownership.
- Generated catalogs MUST be reproducible and never hand-edited.
- Platform capability changes MUST be researched and tested before product guidance changes.
- Real-use journeys MUST be rerun for material plugin, host, or model behavior changes.

## Plugin completion blockers

The integrated product cannot be complete while any of these are true:

- the plugin is proven only at source or package level;
- the host cannot discover the intended front door in a new task;
- setup or retrofit overwrites user-owned decisions;
- routine work requires strict/global proof or an optional service;
- a required journey lacks behavioral red/green/tamper coverage;
- research-backed product guidance depends on an unrefreshed, misclassified, or unmapped source;
- plugin skills expose competing authority or obsolete lane/model guidance;
- package, install, cache, application, and runtime identities are conflated;
- the plugin cannot explain and recover from a representative failure;
- an end-to-end real-use journey has not been independently reconciled.
