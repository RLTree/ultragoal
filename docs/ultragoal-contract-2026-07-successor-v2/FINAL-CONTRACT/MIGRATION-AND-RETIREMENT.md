# Migration and Retirement

## Objective

Replace static lane/gate-era orchestration and duplicate authority without breaking supported users or preserving permanent shadow systems. Migration is semantic: renaming visible files is insufficient.

Snapshot scale establishes the need for generated migration tooling: 13,542 `lane` references across 872 tracked files, 10,808 `gate` references across 1,221 files, and 297 `gpt-5.5` references across 172 files. These are snapshot observations, not live counts; HCT-INVENTORY and HCT-MIGRATE must recompute live state.

## Canonical routes

| Class | Old surface | Canonical target | Disposition | Removal proof |
| --- | --- | --- | --- | --- |
| skills | ultragoal | harness-ultragoal | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | harness-engineering | harness-ultragoal | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | agent-first-repo-init | repository-fit | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | agent-first-repo-retrofit | repository-fit | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | fit-repo | repository-fit | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | execplan-lane | routine-work | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | agent-runtime-legibility | routine-work | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | agent-observability-stack | diagnose-and-observe | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | orchestrator-reconciler | goal-run | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | proof-gate | prove | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | product-cohesion-gate | prove | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | product-fitness-gate | prove | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | agent-improvement-loop | improve-and-maintain | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| skills | standards-gardener | improve-and-maintain | compatibility route then retire | No active references or observed invocations beyond adopted compatibility window; canonical route passes representative tasks. |
| agents | contract-claim-falsifier | claim-falsifier | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-contract-claim-falsifier | claim-falsifier | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-material-review-scope-gatekeeper | product-journey-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-orchestration-recovery-falsifier | orchestration-recovery-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-product-simplicity-falsifier | product-journey-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-repo-initializer | run-scoped WS-FIT worker, not a static custom agent | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-retrofit-planner | run-scoped WS-FIT worker or read-only repo-recon planning | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | harness-security-trust-boundary-falsifier | security-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | material-review-scope-gatekeeper | product-journey-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | orchestration-recovery-falsifier | orchestration-recovery-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | plugin-scout | repo-recon | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | product-simplicity-falsifier | product-journey-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | security-trust-boundary-falsifier | security-reviewer | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| agents | standards-extractor | research-verifier | move/route to current `.codex/agents/` read-only role or run-scoped worker | Current path discovery passes; old path has no active readers; broad write authority absent. |
| commands | Archive | package/migrate | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Audit | inspect/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Control | inspect/next | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Coverage | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | CurrentState | inspect | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | FinalPacket | prove/package (finalizer authority retired) | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | FixtureSchedule | check/eval/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | FoundationalTrace | inspect/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Garbage | migrate | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Halo | prove or retire after semantic inventory | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Help | typed parser help; zero-write | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | ImpactedRustTests | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | ImprovementLoop | eval | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | LineCaps | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | LiveLoop | goal-run route plus check/diagnose | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | MandatoryLawValidation | prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Namespace | inspect/migrate | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | NextAction | next | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Observe | observe/diagnose | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | OpenAi | eval/observe adapter | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | PackageDigest | package/inspect | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | PackageInventory | package/inspect | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Performance | check/eval | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Product | inspect/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Promptfoo | eval adapter | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | RedReport | diagnose/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | ReviewRound | prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | ReviewTarget | inspect | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Routine | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Rust | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | SchemaValidation | check | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | SemanticReceipts | inspect (projection only; receipt authority retired) | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Session | inspect/observe | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | SourceObligations | inspect/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | Standards | inspect/eval/migrate | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | TransactionalFinalization | prove (authority retired into HCT-CLAIMS) | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| commands | TypedBoundaries | check/prove | semantic alias with warning or remove at approved boundary | Alias equivalence, usage measurement, deadline, and active-reference scan pass. |
| candidate_custom_tools | CT-AGENT-EVAL | HCT-EVAL | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-ARTIFACT-DEREF | HCT-CAPTURE | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-AUDIT-CONTEXT | HCT-CONTEXT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-CLAIM-GRAPH | HCT-CLAIMS | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-COMMAND-CAPTURE | HCT-CAPTURE | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-COVERAGE | HCT-IMPACT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-CURRENT-STATE | HCT-STATE | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-FIT-RECONCILE | HCT-FIT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-FIXTURE-SCHEDULER | HCT-FIXTURES | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-IMPACTED-TESTS | HCT-IMPACT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-INCREMENTAL-GRAPH | HCT-IMPACT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-NEXT | HCT-STATE | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-NODE-CACHE | HCT-IMPACT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-OBSERVE-REPAIR | HCT-OBSERVE + HCT-STATE | split responsibilities and consolidate authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-PACKAGE-SNAPSHOT | HCT-DISTRIBUTION | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-PLUGIN-DISCOVERY | HCT-DISTRIBUTION | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-SEMANTIC-INVENTORY | HCT-INVENTORY | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-SURFACE-SPEC | HCT-CONTEXT | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| candidate_custom_tools | CT-TELEMETRY-RECONCILE | HCT-OBSERVE | consolidate implementation and authority | Old API has no active writer/reader; every canonical target's negative and behavior proof passes. |
| orchestration | static lane registry and lane ownership | dependency graph plus run-scoped semantic write leases | generate migration plan; route old descriptors during compatibility | No live scheduler/claim depends on lane IDs; write-scope collision and recovery proof passes. |
| proof authority | audit/product/standards/final-packet/transactional finalization writers | HCT-CLAIMS | convert to evidence producers or read-only projections | Only HCT-CLAIMS can emit claim decisions; tampered projections cannot change ceilings. |
| state authority | mutable receipts and current-state stores | LiveContext-bound findings/state graph | ingest as migration evidence then archive/non-authoritative | No active command reads retired state as authority; recomputation ignores tampering. |
| model assumptions | hard-coded GPT-5.5/model-mode strings | capability-probed runtime metadata and configurable evaluation matrices | parameterize active references; retain history only in non-authoritative archives | Current product runs without predecessor identity assumptions; prompt text cannot prove runtime. |

## Compatibility contract

Every compatibility route must have one owner, exact semantic target, bounded warning, usage measurement, adopted deadline or version boundary, removal condition, and independent equivalence proof. Compatibility is not a second authority. Missing human authorization defaults to non-destructive routing or archival, not deletion.

## Retirement sequence

1. Recompute live active readers, writers, routes, generated outputs, aliases, schemas, fixtures, packages, and documentation references.
2. Establish canonical replacements and root-owned registry entries.
3. Implement behaviorally equivalent compatibility routes where adopted.
4. Migrate state/evidence as non-authoritative inputs; invalidate stale dependent evidence.
5. Update all active references and generated projections.
6. Verify canonical journeys and false-pass controls.
7. Remove old readers/writers/public routes only at the authorized boundary.
8. Prove no duplicate authority, no active reference, no stale package surface, and no unowned generated output remains.

## Required destructive decision

Physical deletion of historical or legacy material is not authorized by this handoff. `OD-009` must be resolved before destructive cleanup. Until then, obsolete material may be archived or clearly marked non-authoritative while release/completion claims remain appropriately lowered.
