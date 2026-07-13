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
- resolve the bound role and every stable node, requirement, surface, tool, claim, migration, and decision ID exactly once against immutable source digests; a runtime role must resolve to a typed receipt in the root-owned runtime-capability namespace whose role, agent type, sandbox, and no-root-authority fields match the envelope;
- resolve one canonical write-scope row and the graph's root-only authority row exactly once from the same immutable authority source, bind both row digests, and reject unknown, duplicate, substituted, owner-mismatched, or out-of-scope candidate paths;
- reject unknown, duplicate, ambiguous, conflicting, stale, or root-spoofed authority;
- walk every candidate path without following symlinks, reject special files, hardlinks, filesystem aliases, traversal, and exact, ancestor, descendant, or case-folded overlap with `forbidden_paths`, keep candidate-source and logical `scratch://` authority separate, and bind the bounded existing-object identity set for final revalidation;
- reject proposed writes to the canonical host-protected roots `.git`, `.codex`, `.agents`, and `.codex-worktree`, plus plugin projection, adopted contract, root-decision, generated-authority, runtime-capability, and other root-only paths;
- treat every `propose_write` grant as instruction-only scope: the envelope cannot execute a workspace effect, and a separately adopted root-owned effect mediator must revalidate and enforce any later write;
- before any external-effect or session validation, capability mint, replay lookup, or consumption, descriptor-open the supplied root as one existing canonical directory without following its final name, reject missing, non-directory, special, symlink, lexical, and physical aliases, bind its device/inode/type and internally derived URI/digest, require the candidate repository fields to match exactly, retain the initial descriptor through the capability lifetime, and at final mediation revalidate it plus an independent no-follow reopen of the supplied path as the exact same root object; reject same-path rename-and-replacement even when original authority leaves are preserved, while consuming nothing on mismatch; resolve every scoped external effect and approval exactly once from immutable current rows under the protected root-owned `docs/ultragoal-successor-live/root-decisions/effect-authority` namespace; traverse and open the authority path descriptor-first without following links; require repository-device, single-link, bounded regular sources; bind source device, inode, mode, link count, size, timestamps, content digest, effect ID and identity, provider, action, kind, target, approval ID/link/status/validity/single-use state, and row digest; resolve time through one protected root-issued single-use session receipt bound to repository, pre-context candidate ID, authority session, envelope capture time, exact effect-source set, nonce/session ID, bounded skew, source identity, and row digest; require one mediator instance to execute the complete successful initial validation and only then internally mint an opaque object-identity capability bound in mediator-private state to the exact validation result, repository root object, candidate, main and effect authority sessions, effect-source set, receipt, and trusted time; provide no separate capability builder, registration method, injectable state, or caller-supplied ledger; permit only that same mediator to accept the same non-constructible, non-cloneable, non-serializable object at final revalidation and enforce a five-minute maximum age and monotonic trusted time; after every final check passes, atomically reserve the exact protected approval identities and session-receipt identity in one factory-private registry shared by every mediator instance, then consume the token, while consuming nothing on failure; use the internally derived repository root plus exact protected namespace plus stable `approval_id` or `receipt_id` tombstones as the replay decision, never caller repository fields, retain root-object/source/row/session/result bindings only as supplemental exact-set keys, require a new stable ID for legitimate reissue, and scope tombstones only across independently canonicalized repositories; reject missing, unknown, duplicate, ambiguous, stale, backdated, future, expired, consumed, substituted, reused, aliased, replaced, mismatched, rolled-back, fabricated, cross-mediator, cloned, serialized, omitted, or unreferenced rows, capabilities, and authorization identities; reopen the same paths and require the same objects at final-session revalidation; keep the fixed effect matrix consistent with the resolved scope; and withhold cross-process or restart single-use claims until a durable canonical root mediator proves them;
- resolve the typed output schema by exact path and digest, constrain repository result targets to exact delegated generated outputs and scratch targets to exact bounded scratch grants, and reject protected or undelegated output targets;
- derive `context_id` from the complete closed envelope with only the derived `candidate_identity.context_id` field omitted, thereby binding goal, success, role, candidate, scope, authority, evidence, unsupported-surface ceilings, effects, full output contract, verification, stop rule, and no-claim statement; and
- revalidate the candidate, role, authority sources, output schema, and selected set before accepting a result.

Schema acceptance, self-reported zero conflict counts, registry prose, or a supplied digest never substitutes for that resolver. The deterministic `AGENT-TASK-ENVELOPE-RED-FIXTURES.json` corpus and `verify-agent-task-envelope.py` harness must reject all named adversarial classes before any envelope is used. Production adoption requires equivalent behavior in the canonical Rust boundary; the proposal harness is review evidence, not live product authority.

External-effect authorization is explicitly two-stage. Envelope validation only proves that a candidate-bound effect, approval, and root-issued time/session receipt are internally current at one trusted check; it never grants or executes the effect. The same separately adopted root-owned mediator instance must perform the complete initial check and final check. Initial success is the only branch that can mint the opaque capability and register its object identity plus exact result binding in private mediator state; callers cannot build, register, clone, serialize, or inject it. Final mediation rejects any other object or mediator, requires identical repository, candidate, main and effect authority sessions, effect-source set, and receipt, rejects capabilities older than five minutes or clocks earlier than initial trusted time, reopens and revalidates every bound authority object, and rechecks receipt and approval expiry. A lock-protected factory-private registry shared by all mediator instances then atomically checks and consumes every approval member and the receipt identity together with the token; any prior validation failure leaves all identities unconsumed. Immutable per-repository namespace-and-ID tombstones prevent revival through any mutable receipt or approval fields, while new stable IDs and the same IDs in an unrelated repository remain available. The Python harness proves this behavior only within one process. It provides no cross-process, restart, crash-recovery, or durable single-use proof, so adoption remains deferred pending the canonical durable root mediator. The proposal validator has no trusted live clock or receipt, so every live nonempty external-effect envelope fails closed.

The current dependency graph's `exclusive_semantics` and `forbidden` values are explanatory prose, not machine-enforceable semantic prefixes. This proposal therefore treats `semantic_symbols` as non-authoritative instruction labels only. No semantic effect may be mediated until an adopted root-owned catalog supplies structured allowed and root-only semantic prefixes and the canonical resolver binds them to the same write-scope session. Prose normalization or prompt inference cannot supply that missing authority.

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

`AgentTaskEnvelope-v1` uses one closed decision for each effect class rather than independent allowed/forbidden lists, so overlap is structurally impossible. Candidate paths are repository-relative; scratch paths are logical `scratch://` URIs resolved only by the scratch executor. Every proposed path must fall inside the exact bound canonical write scope, and generated outputs must fall inside its exact generated-output set. A repository-relative `result_target` must exactly equal one delegated generated output; a scratch result target must exactly equal one bounded scratch grant; `message://final` remains non-persistent. `propose_write` is an instruction-only scope marker, never executable authority. Candidate scope never implies Git, host-configuration, shared-root authority, or a public-command adoption; any later workspace effect requires a separately adopted root-owned mediator. External-effect names also carry no authority: a non-empty external-effect scope requires an exact same-session effect and unconsumed approval resolution from the protected root authority namespace, and an empty scope requires an empty effect-authority binding. The proposal resolver enforces full-envelope context commitment, same-session write-scope and root-authority binding, protected-path classes, allowed/forbidden prefix disjointness, and a bounded no-follow identity preflight; that preflight does not substitute for descriptor-safe effect enforcement. Every verification category contains at least one named control or a typed, authority-referenced inapplicability reason. These structural rules still require the same-session semantic resolver above.

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
