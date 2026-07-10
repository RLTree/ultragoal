# Integrated Product Contract

## Product definition

Harness Ultragoal is one product with four inseparable layers:

1. **Plugin product** — acquisition, installation, discovery, skills, agents, templates, repository fitting, guidance, maintenance, research, and real-use journeys.
2. **Authority kernel** — the Rust `ultragoal` CLI and library that compute deterministic, typed, fail-closed decisions.
3. **Harness-owned tooling** — product truth, affected-set legality, verified reuse, evidence reconciliation, current state, repair, evaluation, and claim authority.
4. **Lifecycle** — install, discover, fit, work, diagnose, prove, improve, maintain, and complete a durable goal.

No layer may claim product completion independently. A fast validator without a usable plugin is not the product. A discoverable plugin without enforced truth is not the product.

## Product outcomes

The product MUST let a repository operator or agent:

- acquire and verify the intended plugin;
- discover a small, accurate entry point;
- fit a new or existing repository without destroying local decisions;
- perform routine work quickly on a dirty tree;
- understand current product state without opening receipts manually;
- receive one exact next repair for each blocker;
- run strict, independently reconciled proof when a claim requires it;
- orchestrate concurrent work without overlapping ownership or split authority;
- improve the system from traces and failures without self-certifying changes;
- stop honestly when evidence is stale, missing, incongruent, or blocked;
- retire obsolete machinery after replacement proof exists.

## Binding laws

### HUL-PRODUCT-001 — Integrated lifecycle product

The plugin, CLI, Harness-owned tooling, and lifecycle MUST be planned, implemented, validated, and released as one dependency graph. The product owner is Product Architecture. The validator MUST reject a completion claim if any required lifecycle journey lacks current product proof. A source manifest or passing CLI test alone cannot satisfy this law.

### HUL-PLUGIN-001 — Progressive plugin closure

The plugin MUST expose a concise front door, progressively disclose specialized instructions, and close every routed journey with product-owned guidance, repair, and proof. The product owner is Plugin Product. The validator MUST check component reachability, description accuracy, context budget, duplicate routes, and journey closure; real application discovery and execution MUST be reconciled independently.

### HUL-DISCOVERY-001 — Surface-specific discovery truth

Source, built package, installed bytes, local cache, application registry, user-visible discovery, and task runtime are distinct truth surfaces. The product owner is Distribution and Discovery. A claim MUST name its surface and MUST NOT be promoted by evidence from another surface. Each transition MUST reconcile content identity, version, origin, and observed behavior.

### HUL-FIT-001 — Reconciled setup and retrofit

Repository fitting MUST inspect before writing, distinguish fresh setup from retrofit, preserve user-owned changes, produce a bounded plan, apply only authorized mutations, and verify the resulting behavior. The product owner is Repository Fitting. A generated setup receipt cannot prove that the active repository loads or follows the installed contract.

### HUL-STATE-001 — Immutable current-input snapshot

Every authority-bearing operation MUST consume one immutable `AuditContext` that identifies repository root, worktree state, relevant source digests, package/install/runtime identities, configuration, tool versions, optional-surface availability, and command effect. The product owner is Authority Kernel. Evidence without a congruent context MUST be stale or wrong-surface, never current.

### HUL-INCREMENTAL-001 — Verified incremental legality

Routine work MAY inspect an affected subset only when a versioned query graph proves that the subset is closed over product dependencies. Cache reuse MUST bind node semantics, direct and transitive inputs, tool identity, environment predicates, output integrity, and candidate identity. The product owner is Incremental Engine. Uncertain impact expands the set or lowers the claim ceiling.

### HUL-AUTHORITY-001 — Single typed authority graph

All findings, product states, repairs, next actions, and claims MUST derive from one typed authority graph. User-visible commands MAY select views but MUST NOT implement parallel definitions of readiness or completion. The product owner is Authority Kernel. Duplicate claim engines are blockers until consolidated or explicitly non-authoritative.

### HUL-PROOF-001 — Independent claim reconciliation

No completion-bearing artifact may validate itself. Every claim MUST name product behavior, evidence, an independent reconciler, false-pass controls, stale and wrong-surface rejection, a repair, a rerun, and an allowed ceiling. The product owner is Proof System. Missing reconciliation makes the item an observation only.

### HUL-COVERAGE-001 — Routine and strict coverage intelligence

Routine coverage MUST provide fast, changed-surface guidance and may use verified reuse. Strict coverage MUST execute or independently verify same-candidate results across the complete required surface. Line, region, branch, mutation, journey, and claim coverage MUST not be collapsed into one percentage. The product owner is Test Intelligence.

### HUL-OBSERVE-001 — Queryable semantic observability

Product events MUST use stable semantic names, bounded high-cardinality attributes, traceable operation and candidate identifiers, and explicit privacy classes. A zero-dependency local plane MUST support deterministic query and repair. Optional OpenTelemetry export MUST be independently roundtrip-tested. The product owner is Observability.

### HUL-REPAIR-001 — Exact repair and next action

Every blocking diagnostic MUST contain a stable code, concise cause, smallest safe repair, exact rerun, effect classification, and resulting claim ceiling. `ultragoal next` MUST compile one prioritized action from the same findings graph and MUST remain read-only. The product owner is Operator Experience.

### HUL-GOAL-001 — Goal lineage without simulation

Harness Ultragoal MUST integrate with the platform’s durable goal mechanism when available, record repository contract lineage, and distinguish platform goal state from product work state. It MUST NOT create a competing goal engine or infer platform state from repository receipts. The product owner is Goal Integration.

### HUL-ORCHESTRATION-001 — Adaptive Ultra work graph

The root Ultra agent MUST derive product-semantic work packages from current state and dependency edges, not a fixed historical lane list. Work packages MUST declare safety class, reads, writes, prerequisites, outputs, acceptance, and claim effect. The product owner is Orchestration.

### HUL-WORKER-001 — Typed worker ownership and acceptance

Reconnaissance and adversarial workers are read-only. Write-owning workers may begin only after their contract and ownership are stable, and their write sets MUST be disjoint unless root-serialized. Worker reports MUST use the defined output schema. Root acceptance MUST reproduce or independently verify completion-bearing results. The product owner is Orchestration.

### HUL-REVIEW-001 — Independent falsification

Material product claims MUST undergo review by an actor that did not author the implementation or its primary evidence. Review MUST try false-pass, stale-evidence, wrong-surface, bypass, usability, and recovery cases. The product owner is Review Authority. Reviewer approval without reproduced evidence is advisory.

### HUL-SECURITY-001 — Confinement, permission, and secret safety

All commands and workers MUST declare effects, constrain paths to authorized roots, handle symlinks and races, preserve user changes, minimize privileges, redact secrets and private paths before persistence or export, and fail closed on ambiguous authority. The product owner is Security.

### HUL-SUPPLY-001 — Reproducible package integrity

Package claims MUST connect canonical source, dependency resolution, build environment, generated inventory, archive bytes, digest, provenance, install bytes, and application-observed identity. The product owner is Distribution Security. Signatures or provenance MAY strengthen release integrity but cannot replace behavior verification.

### HUL-CLI-001 — Discoverable, typed, non-surprising CLI

The CLI MUST use typed domain values and a conventional parser, provide stable help and machine-readable output, distinguish read and write commands structurally, avoid hidden writes, use documented exit codes, and keep internal gate architecture out of the primary UX. The product owner is CLI Product.

### HUL-EVAL-001 — Valid evaluations and failure harvesting

Agent and product evaluations MUST validate task clarity, test correctness, coverage of intended behavior, scoring integrity, contamination risk, and reproducibility before their results can block or promote a claim. Harvested failures MUST be triaged before becoming law or fixtures. The product owner is Agent Quality.

### HUL-IMPROVEMENT-001 — Evidence-driven improvement loop

Improvement proposals MUST connect observed failure, trace or replay, classified finding, scoped hypothesis, candidate change, evaluation, adversarial review, promotion decision, and rollback. The system MUST NOT self-certify its own improvement. The product owner is Product Improvement.

### HUL-PERFORMANCE-001 — Fast routine work, honest strict proof

Routine commands MUST remain useful when the tree is dirty and optional surfaces are absent. Performance evidence MUST measure executed work or verified same-candidate reuse, report scope and cache state, and reject bookkeeping-only timing. Strict proof MAY be slower but MUST expose the critical path and avoid accidental global work in routine mode. The product owner is Performance.

### HUL-PRIVACY-001 — Telemetry minimization and retention

Events and artifacts MUST collect only product-required fields, classify data before persistence, exclude secrets, bound high-cardinality values, define local and exported retention, and permit deterministic deletion. The product owner is Privacy. Baggage and external telemetry are untrusted propagation surfaces.

### HUL-MAINTENANCE-001 — Measured entropy and retirement

The product MUST inventory obsolete files, generated drift, cache growth, deprecated commands, abandoned work state, and redundant authority. Cleanup MUST be planned, authorized, reversible where practical, and verified. The product owner is Maintenance. Deletion counts alone are not improvement proof.

### HUL-COMPLETION-001 — Honest stop and completion ceiling

Only the root claim authority may issue an integrated completion decision. It MUST use current inputs, required product-journey proof, independent reconciliation, resolved blockers, cleanup status, and the minimum ceiling across all required claims. Missing or uncertain evidence MUST lower the ceiling and produce a precise stop reason. The product owner is Root Claim Authority.

## Product state model

Each required product surface has one of these states:

- `unknown` — insufficient current information;
- `not_applicable` — excluded by a typed applicability rule;
- `blocked` — required prerequisite or authority missing;
- `observed` — evidence exists but is not independently reconciled;
- `reconciled` — behavior and evidence agree for the current context;
- `claimable` — all claim guard conditions are satisfied at a named ceiling;
- `stale` — identity or time/input congruence failed;
- `invalid` — malformed, tampered, wrong-surface, or internally contradictory.

States are monotonic only within one immutable `AuditContext`. A new relevant input creates a new context and may invalidate prior state.

## Claim ceilings

From lowest to highest:

1. `description_only`
2. `source_present`
3. `observation_only`
4. `routine_repair`
5. `surface_reconciled`
6. `journey_reconciled`
7. `strict_candidate`
8. `integrated_completion_candidate`
9. `released_product` — requires external release, installation, and real-use authority outside a source-only run

No average or majority vote may raise the minimum required claim ceiling.

## Diagnostic contract

Every diagnostic MUST include:

```json
{
  "diagnostic_id": "HUL-DISCOVERY-001.INSTALL_BYTES_MISMATCH",
  "severity": "blocker",
  "surface": "installed_plugin",
  "context_id": "sha256:...",
  "cause": "Installed bytes do not match the verified package snapshot.",
  "evidence_refs": ["artifact://..."],
  "smallest_repair": "Reinstall the verified package into the selected plugin home.",
  "rerun": "ultragoal prove --surface install --json",
  "effect": "writes_install_state",
  "claim_ceiling": "source_present"
}
```

Paths and commands in diagnostics MUST be portable or explicitly environment-bound. Private absolute paths MUST not escape into exported evidence.

## Bypass resistance

The product MUST reject:

- manually setting a completion field;
- editing a receipt after execution;
- substituting a source digest for installed bytes;
- using a passing fixture report as runtime proof;
- reusing cache output whose candidate or dependencies cannot be verified;
- hiding a required surface by omitting it from a generated manifest;
- reporting a narrow subset as strict/global proof;
- satisfying review with the authoring worker’s own assertion;
- converting an optional unavailable surface into a silent pass;
- treating a platform goal, worker task, contract work package, and historical lane as the same identifier.

## Compatibility and evolution

Laws are stable product semantics. A law may be changed only through:

1. source or product-failure motivation;
2. explicit compatibility analysis;
3. migration aliases for affected diagnostics, schemas, and fixtures;
4. updated validator and adversarial proof;
5. independent review;
6. a versioned contract release.

History belongs in release notes or migration artifacts, not in the concise active contract.
