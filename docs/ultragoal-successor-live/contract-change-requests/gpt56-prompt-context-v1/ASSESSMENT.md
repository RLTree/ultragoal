# Prompt And Context Assessment

Status: integration-ready proposal; not adopted contract authority  
Target: Harness Ultragoal successor contract and its plugin, agent, review, and CLI prompt surfaces  
Assessment basis: current source bytes on `codex/successor-contract-v2-live-product` at HEAD `e28347f0a5c4e23b703c41c0b9266ebb972d022a`; the worktree was dirty from concurrent parent work  
Behavioral ceiling: source-level design assessment only; no Luna, Terra, Sol, or Sol Ultra A/B evaluation was run

## Final assessment

The current pasteable goal is already a strong prompt for a capable orchestration model. It is 417 words and correctly supplies the outcome, live-proof boundary, root authority, delegation contract, invariants, verification bar, and terminal condition. The six current `.codex/agents/*.toml` roles are also appropriately concise. The problem is not primarily prompt wording.

The material weakness is instruction topology:

1. The lean goal immediately expands into a 25,656-word prescribed startup set. The complete final-contract directory is 38,218 words. Durable authority has been moved out of the prompt, but not yet progressively disclosed to the model.
2. The same laws recur across the goal, bootstrap, readme, standards, skills, legacy wrappers, agent prompts, review packets, and validators. Repetition increases context cost and contradiction risk without increasing authority.
3. Every material review round is currently specified as a fresh four-person, full-scope, unanimous sign-off attempt. That is appropriate at a final integrated claim boundary, but wasteful for rolling artifact-set acceptance. A first decisive material defect already determines `REWORK`.
4. Current reviewers are correctly source-read-only, while the review contract also expects reproduction of build and test evidence. Commands such as Cargo tests require isolated scratch writes. The missing distinction is read-only candidate authority versus scratch-only verification authority.
5. Fourteen compatibility skill wrappers contain 2,996 words, slightly more than the 2,867 words in the eight canonical skills. The migration purpose is valid, but the wrappers remain a discovery and context tax until retirement.
6. The plugin default route asks for the "smallest authoritative workflow." This can reward micro-slicing when the appropriate unit is a substantial dependency-closed node.
7. Review prompt packets bind useful identities and digests but use the opaque scope label `full_current_scope`. Unless a separate task envelope supplies exact artifacts, semantics, claim, evidence, effects, output, and stop rules, the reviewer must reconstruct too much from context.
8. The CLI validates agent manifest shape and read-only status but not prompt focus, duplicate law ownership, task-envelope completeness, or model-independent behavioral quality.
9. The orchestrator automation prompt is referenced by template-integrity enforcement but is absent from the current source tree, so its quality and behavior cannot be evaluated.

## What must remain

Optimization must not remove the contract's load-bearing guarantees:

- live repository and runtime evidence outrank snapshots, chat, memory, and prompt assertions;
- source, package, install, discovery, runtime, journey, release, and completion remain separate truth surfaces;
- only the root owns adopted manifests, shared authority, integration, migration authority, claims, release, and final reporting;
- implementation workers use disjoint run-scoped leases and return `WorkerResult-v1`;
- independent review precedes root acceptance;
- unavailable capabilities and external authority lower only dependent ceilings;
- read, help, parse, inspect, next, and query paths have zero hidden writes;
- tests, receipts, generated rows, documentation, telemetry, signatures, and provenance support but never replace product behavior;
- completion requires current candidate-bound representative behavior and final independent falsification.

## Recommended operating design

Use one lean outcome contract across supported model configurations. Change the exposed model, reasoning, and orchestration configuration more often than the prompt. Do not infer a model or mode from a routing label or prompt.

The durable contract remains complete authority, but startup loads only:

1. the authority index and manifest;
2. stable laws;
3. the dependency graph row for active nodes and their dependency closure;
4. this model-adaptive prompt/context contract;
5. the current live board, accepted evidence, open findings, and active WorkerResults.

The context router then selects exact requirement, surface, tool, claim, migration, research, and decision rows by stable ID. Full registries are loaded only for global reconciliation, cross-registry repair, final release, or completion.

Root orchestration should allocate substantial dependency-closed node packages. One implementation lane owns one node at a time. Review is candidate-set-specific and rolling. A material defect stops that review round immediately. Comprehensive multi-role review remains mandatory at the exact integrated claim boundary named by the contract.

Current agent manifests remain role-only. Each invocation receives a typed `AgentTaskEnvelope-v1` that supplies the current objective, exact scope, candidate, proposed claim or decision, evidence, forbidden effects, expected output, and stop rule. This prevents static role prompts from becoming duplicate contract authorities.

Read-only reviewers inspect candidate source and evidence. When executable reproduction requires writes, an independent verification executor receives an ephemeral scratch-only lease with an isolated `CARGO_TARGET_DIR` and no candidate-source authority. If the host cannot provide that capability, the reviewer labels executable evidence supplied-only and the affected ceiling remains lower.

Compatibility wrappers should be generated from one minimal template, excluded from ordinary canonical routing, and retired only under the adopted migration proof. CLI validation should enforce route ownership, prompt/task-envelope structure, duplicate-law absence, forbidden prompt inference, and representative prompt evaluations rather than judging prose length alone.

## Root adoption boundary

The files in this change request are proposed replacements and additions. They intentionally do not edit the adopted digest-bound contract, contract manifest, critical-path board, package manifest, public command catalog, canonical skills, current agent manifests, migration registry, or claims. The parent Ultra root must independently review the proposal, choose the adopted requirement changes, apply root-owned edits, recompute all affected digests and projections, and lower or reopen every proof surface invalidated by adoption.

This assessment makes no readiness, release, node-closure, or goal-completion claim.
