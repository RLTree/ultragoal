# Migration and Retirement

## Migration posture

The active `docs/ultragoal-contract-2026-07/` remains authoritative until this candidate is independently reviewed, revised, and explicitly adopted. Migration MUST be incremental and evidence-bearing. Old authority is not deleted because the successor document exists.

Historical gate IDs remain searchable aliases for one compatibility version. They MUST NOT appear in new work-package IDs, public primary command names, or completion language.

## High-level decisions

| Current surface | Decision | Successor owner |
|---|---|---|
| Product/plugin lifecycle doctrine | Retain and rewrite | HUL-PRODUCT-001, HUL-PLUGIN-001 |
| Source/install/cache/app/runtime separation | Retain and formalize | HUL-DISCOVERY-001, HUL-PROOF-001 |
| Mandatory fit/retrofit entry | Retain and reconcile | HUL-FIT-001 |
| Fixed lane registry and launch state | Retire as architecture; archive as history | HUL-ORCHESTRATION-001 |
| Gate-specific commands | Consolidate | HUL-AUTHORITY-001, HUL-CLI-001 |
| Rust authority kernel | Retain and restructure | HUL-STATE-001 through HUL-CLI-001 |
| Research corpus | Retain provenance; refresh primary sources | HUL-EVAL-001, research registry |
| Source-law parallel matrices | Consolidate into one requirement graph | HUL-AUTHORITY-001 |
| Red/green/tamper fixture concept | Retain; rewrite behavioral coverage | HUL-PROOF-001 |
| Boolean-only law fixtures | Replace then retire | behavioral fixture metadata |
| Heavy local observability stack | Retain as optional integration substrate | HUL-OBSERVE-001 |
| Local structured spool | Retain and make zero-dependency authority input | HUL-OBSERVE-001 |
| Embedded Python policy in scripts | Replace; scripts become thin adapters | Rust product tools |
| Mutable checklist and progress prose | Retire from contract | current-state read model |
| Agent role pairs in Markdown/TOML | Consolidate canonical definition; generate views | HUL-WORKER-001 |
| Historical model pins | Retire | runtime-observed orchestration config |
| HALO/promptfoo/vendor adapter mandates | Demote to optional adapters | HUL-EVAL-001, HUL-IMPROVEMENT-001 |
| Generated package/resource inventories | Generate from canonical metadata | CT-PACKAGE-SNAPSHOT |

## Legacy gate migration map

### Gates 000-088

The pre-89 gate set is a cumulative builder-history contract whose individual requirements are already represented in the active foundational and source-obligation matrices. It migrates by semantic family, not by preserving 89 implementation phases:

| Legacy aliases | Successor laws | Disposition |
|---|---|---|
| GATE-000 through GATE-007 | HUL-PRODUCT-001, HUL-COMPLETION-001 | Consolidate base product/completion doctrine |
| GATE-008 through GATE-015 | HUL-AUTHORITY-001, HUL-CLI-001, HUL-REPAIR-001 | Consolidate command, naming, and operator authority |
| GATE-016 through GATE-023 | HUL-FIT-001, HUL-PLUGIN-001 | Consolidate setup, instructions, templates, and plugin entry |
| GATE-024 through GATE-031 | HUL-STATE-001, HUL-INCREMENTAL-001, HUL-MAINTENANCE-001 | Consolidate architecture, dependency, size, and cleanup rules |
| GATE-032 through GATE-039 | HUL-COVERAGE-001, HUL-PROOF-001 | Consolidate tests, coverage, fixtures, and validation truth |
| GATE-040 through GATE-047 | HUL-DISCOVERY-001, HUL-SUPPLY-001 | Consolidate package, install, cache, registry, version, and release truth |
| GATE-048 through GATE-055 | HUL-REVIEW-001, HUL-COMPLETION-001 | Consolidate review, final packet, readiness, and claim ceilings |
| GATE-056 through GATE-063 | HUL-SECURITY-001, HUL-PRIVACY-001 | Consolidate security, confinement, secret, and data boundaries |
| GATE-064 through GATE-071 | HUL-PERFORMANCE-001, HUL-INCREMENTAL-001 | Consolidate speed, routine/strict, reuse, and resource rules |
| GATE-072 through GATE-079 | HUL-PLUGIN-001, HUL-PRODUCT-001, HUL-FIT-001 | Consolidate product cohesion, usage, guidance, and repository journeys |
| GATE-080 through GATE-088 | HUL-ORCHESTRATION-001, HUL-WORKER-001, HUL-COMPLETION-001 | Consolidate worktrees, lanes, orchestration, stop, and integration rules |

Before adoption, an implementation task MUST use the existing `docs/foundational-law-traceability.json`, `docs/source-obligation-matrix.json`, and `docs/mandatory-law-surfaces.json` to assign every individual legacy law/gate row to a concrete successor requirement entry. The range table is an architecture-level alias map, not permission to drop a row. Unmapped rows block retirement.

### Gates 089-105

| Legacy gate | Legacy concern | Successor law(s) | Decision |
|---|---|---|---|
| GATE-089 | CLI control plane and non-bypassable execution | HUL-AUTHORITY-001, HUL-CLI-001, HUL-PROOF-001 | Retain semantics; consolidate command surface |
| GATE-090 | Semantic namespace and purpose-backed surfaces | HUL-AUTHORITY-001, HUL-CLI-001, HUL-MAINTENANCE-001 | Retain semantic naming; retire goal-history names |
| GATE-091 | Rust developer experience | HUL-CLI-001, HUL-PERFORMANCE-001, HUL-MAINTENANCE-001 | Retain useful Rust practices; remove gate-specific score authority |
| GATE-092 | Observability, current state, repair, and fast loop | HUL-OBSERVE-001, HUL-REPAIR-001, HUL-STATE-001, HUL-INCREMENTAL-001, HUL-PERFORMANCE-001 | Split by product owner; external stack becomes optional substrate |
| GATE-093 | Research source authority and article-to-law trace | HUL-PROOF-001 plus research-law process | Retain provenance; consolidate duplicate law matrices |
| GATE-094 | Trace feedback and improvement loop | HUL-IMPROVEMENT-001, HUL-EVAL-001 | Retain; require valid eval and independent promotion |
| GATE-095 | OpenAI API/model/cost/external AI boundary | HUL-SECURITY-001, HUL-PRIVACY-001, HUL-EVAL-001 | Generalize to external-tool boundary; no vendor output authority |
| GATE-096 | Promptfoo eval and red-team adapter | HUL-EVAL-001 | Demote Promptfoo to optional adapter; retain evaluation semantics |
| GATE-097 | HALO ranked changes | HUL-IMPROVEMENT-001 | Demote HALO to optional ranker; retire mandatory product dependency |
| GATE-098 | Self-improving domain-agent pattern | HUL-EVAL-001, HUL-IMPROVEMENT-001 | Retain evidence-to-eval pattern; reject self-certification |
| GATE-099 | Setup and retrofit integration | HUL-FIT-001, HUL-PLUGIN-001 | Retain and make one fit engine |
| GATE-100 | Cross-repository rollout | HUL-DISCOVERY-001, HUL-FIT-001, HUL-MAINTENANCE-001 | Retain as per-repository reconciled journey, never parent-memory propagation |
| GATE-101 | Rust/TypeScript developer experience | HUL-CLI-001, HUL-PERFORMANCE-001, HUL-FIT-001 | Generalize to detected-stack fitting; no language guide as universal law |
| GATE-102 | Feedback/telemetry privacy and retention | HUL-PRIVACY-001, HUL-SECURITY-001, HUL-OBSERVE-001 | Retain and centralize data classification |
| GATE-103 | Improvement-surface separation | HUL-DISCOVERY-001, HUL-PROOF-001 | Retain as core wrong-surface rejection |
| GATE-104 | Research-to-law closure across surfaces | HUL-AUTHORITY-001, HUL-PROOF-001 | Replace parallel matrices with canonical requirement graph and generated views |
| GATE-105 | Measured improvement and evolution | HUL-EVAL-001, HUL-IMPROVEMENT-001, HUL-MAINTENANCE-001 | Retain outcome measurement; reject anecdote/score-only proof |

## Command migration

| Current family | Successor | Compatibility behavior |
|---|---|---|
| `audit`, `source audit`, law/source-obligation/foundational validators | `check` or `prove --surface source` | filtered views over one findings graph |
| product, standards, review, final-packet readiness | `prove --surface <claim>` | no independent ceiling computation |
| package inventory/digest/cohesion | `package snapshot|verify|diff` | old commands read-only aliases; never hidden writes |
| current-state variants | `status` | one read model |
| next-action variants | `next` | one deterministic read-only selector |
| observe query/explain/next variants | `observe query|explain`; `next` | repair compiler shared with findings graph |
| routine/live-loop/check scripts | `check --changed` | routine scope explicit |
| full/strict coverage commands | `prove --surface strict-coverage` | strict scope and cache state explicit |
| fixture schedule/red report | internal selector under check/prove | public only if a user journey requires it |
| setup/init/retrofit agents and scripts | `fit inspect|plan|apply|verify` | agents become UX roles over one engine |
| HALO/OpenAI/promptfoo commands | optional eval/improvement adapters | never primary claim commands |
| garbage/workspace cleanup | `maintain plan|apply|verify` | destructive effect explicit |

## File and directory migration

### Retain as canonical or candidate inputs

- Rust source that owns validated product semantics after restructuring;
- concise plugin manifest identity and canonical component metadata;
- high-value behavioral fixtures and fixture scheduler isolation primitives;
- primary-source registry and trace provenance;
- repository-fitting templates after ownership/version review;
- optional observability development environment as an adapter testbed.

### Generate rather than hand-maintain

- plugin resource list;
- component/cohesion inventory;
- law-to-owner/surface/validator/fixture views;
- agent Markdown and custom-agent TOML presentations where both are required;
- fixture catalogs;
- public command inventory;
- package include manifest;
- concise current status.

### Move to history/archive outside active authority

- `LANE_REGISTRY.json` and `.codex/lane-registry/**`;
- mutable checklist progress narratives;
- old final packets and review launch status;
- gate-specific migration notes after semantic alias adoption;
- superseded research summaries, retaining provenance links.

### Remove after replacement proof

- duplicate claim engines and top-level command paths;
- embedded Python product-policy blocks;
- dead fallbacks and generic helpers with no product role;
- boolean-only law fixtures whose behavioral replacements pass;
- obsolete model pins and static lane instructions;
- generated catalogs that are not reproducible from canonical metadata;
- receipt writers invoked by read-only commands;
- external-tool mandates without measured justification.

## Retirement protocol

No surface is retired until:

1. successor owner and behavior are implemented;
2. legacy inputs and consumers are inventoried;
3. equivalence or intentional-difference tests pass;
4. user and agent migration guidance is discoverable;
5. package/install/app/runtime references are checked where applicable;
6. deprecation diagnostics have run for one compatibility version unless a security issue requires immediate removal;
7. a cleanup plan lists exact effects and rollback/irreversibility;
8. independent review confirms no claim or recovery gap;
9. root claim authority records retirement without raising unrelated ceilings.

## Generated and mutable evidence migration

Mutable receipts and progress histories do not enter the successor contract. During transition:

- legacy readers may dereference old receipts through typed adapters;
- old receipts remain observation-only with original context and schema;
- new commands write only explicitly requested artifacts;
- current-state views derive from canonical inputs and events, not copied status rows;
- compatibility adapters are versioned, measured, and have a removal condition.

## Package/version migration

The current plugin version and Rust package version are not synchronized product identity today. The successor MUST define:

- one product release version;
- component implementation versions only where technically required;
- package schema version independent of product version;
- compatibility range for host, plugin package, CLI, and repository contract;
- upgrade/downgrade behavior;
- release provenance and installed identity reconciliation.

Version alignment is necessary but not sufficient for byte or behavior identity.

## Migration completion blockers

- any individual active legacy law lacks a successor mapping;
- an old and new command compute different claim authority;
- a retired fixture was not replaced by behavioral coverage or explicitly judged obsolete;
- package/install/app/runtime still references removed resources;
- historical lane state is required to reconstruct current work;
- user changes or active repository instructions are lost;
- a compatibility adapter has no owner or removal condition;
- mutable receipts or progress prose remain binding contract inputs;
- cleanup precedes independent replacement proof.
