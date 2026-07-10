# Plugin Product and Journeys

## Product journey registry

| Journey ID | User outcome | Owned surfaces |
| --- | --- | --- |
| J-ACQUIRE | Acquire the intended plugin | PS-SOURCE, PS-PLUGIN-MANIFEST, PS-MARKETPLACE |
| J-INSTALL | Install and verify package bytes | PS-PLUGIN-MANIFEST, PS-MARKETPLACE, PS-PACKAGE, PS-INSTALL |
| J-DISCOVER | Discover the plugin in the selected host | PS-PLUGIN-MANIFEST, PS-MARKETPLACE, PS-DISCOVERY |
| J-ENTER | Understand and invoke the front door | PS-SKILLS, PS-DISCOVERY, PS-ENTRY |
| J-FIT-FRESH | Fit a fresh repository | PS-SKILLS, PS-FIT-FRESH |
| J-FIT-RETROFIT | Retrofit an existing repository | PS-SKILLS, PS-FIT-RETROFIT |
| J-ROUTINE | Perform routine dirty-tree work | PS-SKILLS, PS-CLI, PS-ROUTINE |
| J-DIAGNOSE | Diagnose and repair a failure | PS-SKILLS, PS-CLI, PS-DIAGNOSE, PS-OBSERVE-LOCAL |
| J-PROVE | Run claim-specific strict proof | PS-SKILLS, PS-CLI, PS-PROOF |
| J-OBSERVE | Query and explain product behavior | PS-OBSERVE-LOCAL, PS-OBSERVE-EXPORT |
| J-ORCHESTRATE | Complete dependency-closed multi-scope work | PS-AGENTS, PS-ORCHESTRATION, PS-GOAL |
| J-EVALUATE | Evaluate behavior and harvest failures | PS-SKILLS, PS-AGENTS, PS-EVAL |
| J-RESEARCH | Refresh research and propose law changes | PS-SKILLS, PS-RESEARCH |
| J-UPGRADE | Upgrade, route, and retire old surfaces | PS-SOURCE, PS-SKILLS, PS-CLI, PS-RESEARCH, PS-UPGRADE |
| J-RELEASE | Build and verify a release candidate | PS-CLI, PS-PACKAGE, PS-PROOF, PS-RELEASE |
| J-COMPLETE | Reconcile integrated completion honestly | PS-PROOF, PS-ORCHESTRATION, PS-GOAL, PS-COMPLETION |

## Canonical skill topology

| Canonical skill | Purpose | Legacy routes |
| --- | --- | --- |
| harness-ultragoal | Concise front door, intent classification, authority disclosure, and route selection. | ultragoal, harness-engineering |
| repository-fit | Fresh setup and existing-repository retrofit through inspect/plan/apply/verify. | agent-first-repo-init, agent-first-repo-retrofit, fit-repo |
| routine-work | Dirty-tree-safe affected checks, verified reuse, and exact execution reporting. | execplan-lane, agent-runtime-legibility |
| diagnose-and-observe | Causal diagnosis, local semantic observability, repair, and deterministic next action. | agent-observability-stack |
| goal-run | Durable Goal-mode execution, root orchestration, checkpoints, and honest stops. | orchestrator-reconciler |
| prove | Claim-specific strict proof and independent reconciliation. | proof-gate, product-cohesion-gate, product-fitness-gate |
| improve-and-maintain | Evaluation, failure harvesting, research refresh, migration, compatibility, and retirement. | agent-improvement-loop, standards-gardener |
| product-journey-review | Fresh-user, fresh-agent, maintainer, security, recovery, and proof-falsification journeys. | none |

The `harness-ultragoal` skill is the concise front door. It must reveal only the minimum routing context, state authority/effect boundaries, and load the specialized skill needed for the current intent. Every active skill must contain valid `name` and `description` frontmatter, refer only to shipped paths/commands, and have one semantic owner.

## Read-only custom agents

| Agent | Purpose | Required sandbox |
| --- | --- | --- |
| repo-recon | Recompute live repository, worktree, command, component, and authority state. | read-only |
| research-verifier | Revalidate mutable primary-source facts and classification without changing product law. | read-only |
| product-journey-reviewer | Attack acquisition-through-completion journeys and quality in use. | read-only |
| claim-falsifier | Attack wrong-surface, stale, surrogate, bypass, and reward-hacking proof. | read-only |
| security-reviewer | Attack confinement, permissions, secrets, supply-chain, and trust boundaries. | read-only |
| orchestration-recovery-reviewer | Attack work decomposition, write leases, shared authority, recovery, and stop conditions. | read-only |

Project-scoped files belong under `.codex/agents/`. These roles do not receive durable write ownership. Implementation workers are run-scoped leases described in `04-ORCHESTRATION-AND-WRITE-SCOPES.md`.

## Product surfaces

| Surface | Name | Owner | Snapshot assessment | Implementation disposition | Independent proof |
| --- | --- | --- | --- | --- | --- |
| PS-SOURCE | Canonical source and contract | Product Architecture | fragmented | Consolidate into this contract index, live canonical inventories, and generated projections; archive candidate/history outside active authority. | Independent manifest/reference validation against the live tree and adopted contract digest. |
| PS-PLUGIN-MANIFEST | Standards-compliant plugin manifest | Plugin Product | partial_lane_era | Rewrite descriptions, remove lane/gate UX, include only supported component fields, and bind version to package inventory. | Schema/path validation plus package inspection; host discovery remains a separate claim. |
| PS-MARKETPLACE | Repository and personal marketplace wiring | Distribution and Release Engineering | incomplete | Add repo-scoped marketplace support and tested personal installation instructions; do not infer host registration from catalog bytes. | Catalog schema/path validation, install-byte reconciliation, and independent new-session host probe. |
| PS-SKILLS | Canonical skill topology | Plugin Product | duplicated_and_three_invalid_frontmatters | Consolidate 14 skills into the final semantic topology and route or retire legacy names. | Metadata/reachability validation plus new-task representative invocations in supported hosts. |
| PS-AGENTS | Project-scoped custom agents and role contracts | Ultra Root | wrong_discovery_path_and_static_roles | Move canonical read-only roles to .codex/agents and generate write-worker instructions per work package; retire static lane agents. | Host agent discovery probe, sandbox/effect verification, collision tests, and root acceptance replay. |
| PS-CLI | Rust authority and acceleration CLI | CLI Product | large_fragmented_37_variant_surface | Replace the gate/receipt-heavy public grammar with ten product-semantic groups and explicit effect types; provide temporary routers only where justified. | Parser/property tests, no-write probes, behavior tests, help snapshots, exit-code tests, and representative operator journeys. |
| PS-PACKAGE | Deterministic plugin package | Distribution and Release Engineering | partial | Build from canonical inventory rather than the internal draft; emit deterministic archive, digest, and provenance envelope. | Independent clean-environment rebuild/compare or an explicitly lower reproducibility ceiling, plus expectation-based provenance verification. |
| PS-INSTALL | Installation and installed-byte identity | Distribution and Release Engineering | instructions_only | Implement capability-probed install verification and clear repo/personal scope; require authorization for writes outside the repository. | Post-install byte inventory and digest reconciliation in the selected live location. |
| PS-DISCOVERY | Host discovery and new-session visibility | Distribution and Release Engineering | unproven_by_snapshot | Add host capability adapters and independent new-session probes; report unsupported surfaces without fabricating state. | Same-version package/install reconciliation plus a host-observed listing and representative task invocation. |
| PS-ENTRY | First-run front door | Plugin Product | multiple_competing_entries | Create one harness-ultragoal skill and route specialized skills by state and intent. | Fresh-agent comprehension evaluation and real task completion with bounded context. |
| PS-FIT-FRESH | Fresh repository fitting | Repository Fitting | partial_receipt_heavy | Unify under HCT-FIT and repository-fit skill; generated files derive from typed desired state. | Behavioral fixture and live temporary-repository journey, including idempotency and rollback. |
| PS-FIT-RETROFIT | Existing repository retrofit | Repository Fitting | partial_receipt_heavy | Use semantic ownership and three-way reconciliation; conflicts become findings, never silent overwrites. | Adversarial repositories with pre-existing instructions, partial setup, local edits, conflicts, symlinks, and rollback. |
| PS-ROUTINE | Routine development loop | Routine Development | fragmented_with_global_gate_pressure | Consolidate impact, cache, test, fixture, and result semantics under routine profile; no artifacts by default. | Dirty/clean trees, uncertainty expansion, cache poisoning, timing, interruption, and strict-comparison tests on representative repository tiers. |
| PS-DIAGNOSE | Diagnosis, repair, and next action | Operator Experience | partial_multiple_status_surfaces | Unify current state, findings, repair, diagnose, and next; reject prose-only or duplicate authority. | Injected causal failures, ambiguity, unsupported capability, stale context, and operator comprehension tests. |
| PS-PROOF | Claim-specific strict proof | Proof Authority | fragmented_receipt_and_final_packet_authorities | Replace universal receipt ceremony and duplicate finalizers with HCT-CLAIMS and claim-specific validators. | Stale, wrong-surface, omitted, contradictory, self-signed, tampered, and reward-hacked evidence corpus. |
| PS-OBSERVE-LOCAL | Zero-dependency local observability | Observability | partial_receipt_and_stack_split | Make a bounded append/query store and semantic catalog authoritative for diagnosis, not for claims. | Roundtrip, corruption, truncation, ordering, duplicate, cardinality, redaction, and causal-query tests. |
| PS-OBSERVE-EXPORT | Optional observability export | Observability | optional_partial | Keep export optional; absence lowers only export claims, never routine product behavior. | Consent, redaction, outage, delay, duplicate, wrong-candidate, and backend-query reconciliation. |
| PS-ORCHESTRATION | Adaptive Ultra orchestration | Ultra Root | historical_static_lane_model | Replace lanes with run-scoped work packages and a root-owned scope/claim graph; move historical registries out of active authority. | Conflict, stale worker, blocked worker, drift, interruption, recovery, root acceptance, and no-overlap trials. |
| PS-EVAL | Behavior evaluation and improvement | Agent Quality | partial_adapter_and_receipt_heavy | Create vendor-neutral HCT-EVAL; keep Promptfoo/OpenAI adapters optional and current; retire score-only promotion. | Broken-task, scoring perturbation, contamination, negative-control, replay, and behavior-inspection tests. |
| PS-RESEARCH | Research and law authority | Research Authority | broad_but_stale_and_fragmented | Use the final source registry and promotion workflow; archive duplicate cards and generated mappings. | Source availability/currentness checks and trace validation that each adopted external requirement has the required classifications and mappings. |
| PS-UPGRADE | Migration, compatibility, and retirement | Maintenance and Migration | large_unfinished_lane_gate_migration | Apply the explicit migration map, generated deprecation inventory, compatibility telemetry, and removal gates. | Compatibility route tests, zero-active-reference inventory, generated-surface cleanup, rollback, and representative upgrade journey. |
| PS-GOAL | Host durable goal integration | Goal Integration | not_live_verifiable | Use CODEX-ULTRA-GOAL.md; capability-probe Goal mode and lower only the host-goal claim if unavailable. | Host-exposed goal state and runtime metadata captured without prompt inference; product progress remains separately computed. |
| PS-RELEASE | Release candidate and publication boundary | Distribution and Release Engineering | not_established | Implement release claim graph but stop for unresolved channel, license, signing identity, and publication authorization decisions. | Exact live release profile, independent claim reconciliation, and explicit human release authorization. |
| PS-COMPLETION | Integrated completion authority | Human Release Authority | not_established | Use HCT-CLAIMS only after all predecessors; no duplicate final packet or checklist authority. | Fresh live context, dependency-closed claim graph, independent final falsification, and no unresolved completion blocker. |

## Journey closure rule

A journey is closed only when the intended user can enter it from a supported discovery surface, complete the named behavior, diagnose failures, obtain an exact safe next action, and reach the claim ceiling supported by its live truth surface. Files, docs, schemas, test existence, or telemetry are insufficient.
