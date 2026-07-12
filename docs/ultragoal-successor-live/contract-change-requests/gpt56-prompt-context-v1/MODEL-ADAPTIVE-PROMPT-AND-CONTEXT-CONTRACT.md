# Model-Adaptive Prompt And Context Contract

Status: proposed normative addition; root adoption required

## Purpose

Harness Ultragoal uses one stable outcome contract across supported model configurations. Prompt variants exist only when task inputs, output schemas, authority, or verification semantics differ. Model names, modes, reasoning levels, and orchestration capabilities are runtime configuration, never prompt-derived truth.

This contract governs goal prompts, skills, project agents, run-scoped worker instructions, review packets, automations, CLI prompt validation, and context loading. It refines workflow shape without weakening any adopted product, security, migration, proof, or completion law.

## Prompt contract

Every material prompt or task envelope answers six questions:

1. **Goal:** What observable outcome is required?
2. **Success:** Which current facts make the task complete?
3. **Context:** Which supplied or live facts can change the result?
4. **Constraints:** Which invariants, permissions, effects, and preserved behavior bound the work?
5. **Output:** Which artifact or typed result must be returned?
6. **Verification:** Which behavior and false-pass controls must run before acceptance?

One statement owns each rule. Other prompt surfaces link to that authority or narrow it for a task; they do not restate the law as a competing definition.

Prompts state outcomes and evidence more strongly than procedure. A procedural step is included only when it protects authority, safety, determinism, reproducibility, a dangerous boundary, or a required interface. Exhortations that do not change observable behavior are excluded.

## Runtime configuration

Use task-shape profiles rather than hard-coded model assumptions:

| Task-shape profile | Appropriate work | Prompt adjustment |
| --- | --- | --- |
| typed-transform | extraction, classification, registry comparison, schema-bound transformation | explicit input, labels, output schema, missing-data behavior, and one exact completeness check |
| routine-engineering | ordinary implementation, bounded debugging, documentation, and tool use | concrete outcome, relevant reproduction/context, material constraints, focused verification, and concise report |
| deep-judgment | ambiguous architecture, causal diagnosis, security, product quality, or claim falsification | richer domain context, stronger success and evidence bar, fewer prescribed investigative steps |
| parallel-orchestration | large work with genuinely independent dependency-ready packages | shared authority, disjoint ownership, worker output, review, integration, recovery, and terminal condition |

When the runtime exposes Luna, Terra, Sol, Sol Ultra, Max, Pro, or successor labels, the root may map them to these task shapes. When it does not, the labels remain unknown and must not be inferred from prompt text, configuration intent, or source files.

Use the lowest exposed configuration that passes representative evaluations for the task shape. Raise reasoning or orchestration configuration only when failure evidence shows the lower configuration is insufficient. Do not compensate for missing evidence, an ambiguous success criterion, or a weak verification loop by adding more prompt ceremony.

## Progressive context disclosure

The full adopted contract remains authoritative without being loaded wholesale for every task.

### Tier 0: lean goal

The goal contains the product outcome, live-proof boundary, root authority, orchestration contract, material invariants, legitimate stops, reporting filter, and exact terminal condition. It references durable authority instead of copying registries.

### Tier 1: startup authority

Load only:

- `00-READ-ME-FIRST.md`;
- `CONTRACT_MANIFEST.json` metadata and contract-entry rows;
- `01-AUTHORITY-AND-PRODUCT-LAWS.md`;
- `MODEL-ADAPTIVE-PROMPT-AND-CONTEXT-CONTRACT.md`;
- root-authority and active-node rows from `IMPLEMENTATION_DEPENDENCY_GRAPH.json`;
- unresolved rows from `OPEN-DECISIONS.md` that affect active nodes;
- the current live critical-path board, accepted evidence, open findings, and active WorkerResults.

### Tier 2: node context

For each active node, use `CONTEXT-ROUTING-MAP.json` to load:

- the node row and dependency closure;
- exact requirements selected by stable ID, owner, surface, tool, or claim intersection;
- exact product-surface rows named by the node;
- exact tool rows named by the node and recursive tool dependencies;
- exact claim rows intersecting selected requirements, tools, or surfaces, plus recursive claim prerequisites;
- only the prose modules, migration rows, research rows, and decision rows routed for that node.

Unknown, duplicate, ambiguous, or conflicting IDs fail closed. Prose summaries never override registry rows.

### Tier 3: strict boundary

Load the complete dependency-closed claim evidence and all affected registry rows when a named strict claim, migration retirement, release, or completion boundary is evaluated. Load entire registries only when global reconciliation or a cross-registry invariant requires them.

The root records which tiers and stable IDs were loaded. Context presence is not proof of behavior.

## Ultra orchestration

The root selects the largest safe dependency-closed work package that has one semantic owner and can be independently reviewed. It does not organize the program around tiny slices unless a slice isolates a dangerous boundary, establishes a shared typed interface, unblocks multiple nodes, or resolves a falsified assumption.

Only one implementation lane owns a node at a time. Independent review of a frozen candidate does not count as a second implementation lane. The root may keep other dependency-ready nodes active concurrently and rebalances lanes as the graph changes.

The root remains an active integrator. It verifies and applies accepted root-only wiring promptly, recomputes affected identity and evidence, and does not leave reviewed candidates idle behind administrative integration.

Each implementation lease names exact paths, semantic symbols, generated outputs, fixtures, effects, dependencies, interfaces, positive and negative tests, race/mutation/security/false-pass controls, required `WorkerResult-v1`, and an explicit no-claim statement.

## Agent role and task separation

Static project-agent manifests contain only stable role identity, purpose, least-privilege sandbox, and invariant prohibitions. They do not copy node requirements, claim registries, review cadence, model assumptions, or implementation conclusions.

Each invocation receives `AgentTaskEnvelope-v1` with:

- `task_id` and `role`;
- `goal` and observable `success`;
- current `candidate_identity` and `context_id`;
- exact `artifact_paths`, `semantic_scope`, fixtures, and external effects;
- relevant requirement, surface, tool, claim, migration, and decision IDs;
- primary evidence references and unsupported truth surfaces;
- allowed and forbidden effects;
- expected typed output;
- verification and false-pass controls;
- stop rule and no-claim statement.

An empty, generic, or opaque scope such as `full_current_scope` is insufficient without a bound expansion that resolves to exact current artifacts and semantics.

Envelope acceptance is a two-stage boundary. Draft 2020-12 schema validation proves only closed structure, bounded values, digest syntax, relative-path syntax, the single-valued effect matrix, and required control categories. A same-session semantic resolver must then independently:

- recompute the canonical repository, HEAD/tree, dirty-set or exact artifact-set membership, candidate ID, and context ID;
- resolve the bound role and every stable node, requirement, surface, tool, claim, migration, and decision ID exactly once against immutable source digests;
- reject unknown, duplicate, ambiguous, conflicting, stale, or root-spoofed authority;
- walk every candidate path without following symlinks, reject special files and traversal, keep candidate-source and logical `scratch://` authority separate, and revalidate after execution;
- prove every external effect is a named approved effect and that the fixed effect matrix is consistent with scoped paths and effects;
- resolve the typed output schema by exact path and digest; and
- revalidate the candidate, role, authority sources, output schema, and selected set before accepting a result.

Schema acceptance, self-reported zero conflict counts, registry prose, or a supplied digest never substitutes for that resolver. The deterministic `AGENT-TASK-ENVELOPE-RED-FIXTURES.json` corpus and `verify-agent-task-envelope.py` harness must reject all named adversarial classes before any envelope is used. Production adoption requires equivalent behavior in the canonical Rust boundary; the proposal harness is review evidence, not live product authority.

## Independent verification and review

No implementer accepts their own work.

Rolling artifact-set review proceeds as follows:

1. Freeze exact artifacts and candidate identity.
2. Assign an independent reviewer or verification executor.
3. Continue legal work on other nodes.
4. Return `REWORK` immediately after the first decisive material defect.
5. Correct and re-review the exact new set.
6. Root accepts and integrates only the reviewed set.

Comprehensive multi-role, full-scope review is reserved for the exact claim boundary that names it, including final integrated falsification. A failed required role blocks that boundary, but does not require unaffected reviewers to continue after a decisive defect has already invalidated the round.

Read-only candidate authority and executable scratch authority are separate:

- project reviewers remain unable to modify candidate source, shared state, Git metadata, manifests, claims, or external systems;
- an independent verification executor may receive an ephemeral, path-confined scratch lease for build outputs, temporary repositories, caches, or an isolated `CARGO_TARGET_DIR`;
- scratch outputs bind to the frozen candidate and are destroyed or retained only under the evidence policy;
- when scratch execution is unavailable, the reviewer labels executable evidence supplied-only and the root lowers the affected ceiling.

## Skills, compatibility, and discovery

Canonical skill descriptions optimize correct selection, not keyword coverage. The front door routes to one appropriate authoritative workflow. Orchestration workflows then choose substantial dependency-closed packages rather than the smallest possible slice.

Compatibility wrappers are non-authoritative routes. They are generated from one minimal template, preserve the original request, reject ambiguity, name one canonical target, perform no hidden effects, and fail closed when the target is unavailable. They are excluded from canonical workflow counts and retired only under the migration contract.

## CLI and validator obligations

Prompt validation must check behaviorally relevant structure rather than prose bulk:

- exact canonical role and route ownership;
- complete task-envelope fields and stable-ID resolution;
- no duplicate or contradictory law ownership;
- no model, mode, permission, install, discovery, or runtime inference from prompt text;
- least-privilege candidate and scratch effects;
- no private prompt, transcript, secret, or sensitive-path leakage;
- deterministic prompt/template generation where generated;
- representative route-selection, comprehension, false-pass, and mutation evaluations across supported task-shape configurations;
- explicit unsupported-capability and supplied-only ceilings.

`AgentTaskEnvelope-v1` uses one closed decision for each effect class rather than independent allowed/forbidden lists, so overlap is structurally impossible. Candidate paths are repository-relative; scratch paths are logical `scratch://` URIs resolved only by the scratch executor. Every verification category contains at least one named control or a typed, authority-referenced inapplicability reason. These structural rules still require the same-session semantic resolver above.

A valid manifest, schema, prompt digest, or green evaluator supports source quality only. It does not prove selection, comprehension, runtime behavior, product quality, or completion.

## Automation prompts

Automations use the same six-part prompt contract and a typed state cursor. They must be idempotent, bounded, zero-write while inspecting, honest when no legal action exists, and unable to raise claims. A generic reminder or unresolved template marker is not an orchestration automation.

## Adoption and invalidation

Adopting this contract changes prompt/context authority. The root must:

1. reconcile every affected normative file and registry row;
2. update the contract manifest and bundle digests;
3. regenerate affected package, route, agent, review, and template projections;
4. reopen evidence invalidated by changed prompt or routing bytes;
5. run representative evaluations and independent review;
6. preserve lower claim ceilings until source, package, install, discovery, runtime, journey, release, and completion surfaces independently pass.

No statement in this contract claims a particular model, mode, installation, runtime behavior, readiness, release, or completion.
