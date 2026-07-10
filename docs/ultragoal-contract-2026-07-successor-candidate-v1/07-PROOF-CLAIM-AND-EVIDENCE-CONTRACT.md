# Proof, Claim, and Evidence Contract

## Separation model

Harness Ultragoal maintains six distinct concepts:

1. **Validation truth** — a typed rule ran against named inputs and produced a finding.
2. **Product truth** — the behavior occurred on the named product surface.
3. **Artifact truth** — an artifact is authentic, intact, schema-valid, dereferenceable, and bound to inputs.
4. **Observability truth** — semantic events for the operation can be queried on the named plane.
5. **Speed truth** — scoped work executed or verified reuse occurred under a named environment and measurement method.
6. **Completion authority** — all required product behaviors are independently reconciled at the minimum requested ceiling.

One truth may be evidence for another, but none are interchangeable. A valid receipt may describe a failed product. A passing unit test may coexist with a broken installed plugin. A dashboard may show an event from the wrong candidate. A fast cache lookup may prove no executed work.

## Evidence envelope

Every completion-bearing evidence item MUST contain or dereference:

- schema and producer version;
- `AuditContext` and candidate identity;
- product surface and behavior identifier;
- exact inputs and applicability decision;
- command effect and execution/capture identity;
- start/end monotonic timing and wall-clock observation;
- result plus diagnostic references;
- output artifact identities;
- independent reconciler identity and method, when reconciled;
- privacy classification and redaction state;
- expiry/invalidation rules;
- maximum claim ceiling.

Signing an envelope strengthens authenticity. It does not make its interpretation correct.

## Claim contracts

### CL-SOURCE — Source contract claim

- **Product behavior:** canonical tracked source contains the required law/component and passes static semantic validation.
- **Evidence surface:** current source snapshot, semantic inventory, schema/rule output.
- **Independent reconciliation:** independently enumerate tracked inputs and compare generated views to canonical source.
- **False pass:** a generated manifest lists a file that is absent or a law row exists without implementation.
- **Fail-closed signal:** `HUL-PROOF-001.SOURCE_ENUMERATION_MISMATCH`.
- **Stale/wrong-surface rejection:** any relevant source or generator digest change invalidates it; it cannot support package/install/app/runtime claims.
- **Smallest repair:** restore canonical source/generator agreement and implement missing behavior separately.
- **Exact rerun:** `ultragoal prove --surface source --json`.
- **Allowed ceiling:** `source_present`.

### CL-PACKAGE — Built package claim

- **Product behavior:** normalized package bytes contain exactly the intended source-derived resources with correct metadata.
- **Evidence surface:** canonical package snapshot, archive digest, generated provenance, build capture.
- **Independent reconciliation:** independently build or extract and compare bytes, paths, modes, links, omissions, extras, and version.
- **False pass:** source manifest is valid while the archive omits a skill or includes a private artifact.
- **Fail-closed signal:** `HUL-SUPPLY-001.PACKAGE_CONTENT_MISMATCH`.
- **Stale/wrong-surface rejection:** source, lockfile, build environment, generator, or archive change invalidates it; source digest alone is rejected.
- **Smallest repair:** regenerate the package from the frozen source context and reconcile the first differing entry.
- **Exact rerun:** `ultragoal package verify --snapshot <snapshot> --json`.
- **Allowed ceiling:** `surface_reconciled` for package only.

### CL-INSTALL — Installed plugin claim

- **Product behavior:** the selected plugin home contains the intended package bytes and no unexpected owned files.
- **Evidence surface:** install plan/capture and installed-tree snapshot.
- **Independent reconciliation:** compare installed tree to verified package snapshot using an independent reader after installation.
- **False pass:** installer returns zero but writes to the wrong home or leaves an older file.
- **Fail-closed signal:** `HUL-DISCOVERY-001.INSTALL_BYTES_MISMATCH`.
- **Stale/wrong-surface rejection:** package/install root/version change invalidates it; package proof cannot substitute.
- **Smallest repair:** explicitly reinstall only the named package into the selected home, preserving unrelated plugins.
- **Exact rerun:** `ultragoal prove --surface install --json`.
- **Allowed ceiling:** `surface_reconciled` for install only.

### CL-CACHE — Host cache claim

- **Product behavior:** the host cache references and serves the intended installed plugin identity without stale or mixed-version entries.
- **Evidence surface:** cache inventory and host-supported lookup observation.
- **Independent reconciliation:** resolve cache references to installed bytes and probe a cache-dependent discovery path.
- **False pass:** installed bytes are current while the application reads an older cached manifest.
- **Fail-closed signal:** `HUL-DISCOVERY-001.CACHE_IDENTITY_STALE`.
- **Stale/wrong-surface rejection:** install/cache/app-version change invalidates it; install proof cannot substitute.
- **Smallest repair:** invalidate only the identified plugin cache entry through an explicit supported action.
- **Exact rerun:** `ultragoal prove --surface cache --json`.
- **Allowed ceiling:** `surface_reconciled` for cache only.

### CL-APP-DISCOVERY — Application discovery claim

- **Product behavior:** the running host registers the intended plugin and exposes the correct front-door capability.
- **Evidence surface:** host registry/API observation plus user-visible discovery result.
- **Independent reconciliation:** start a new task/session and invoke a neutral discovery probe that cannot read source directly.
- **False pass:** plugin appears in a source or install list but is absent from the application or exposes the wrong version.
- **Fail-closed signal:** `HUL-DISCOVERY-001.APP_DISCOVERY_MISMATCH`.
- **Stale/wrong-surface rejection:** app process/version/registry/cache/install change invalidates it; screenshots without candidate identity are observations only.
- **Smallest repair:** refresh or restart the smallest documented host surface, then repeat the new-task probe.
- **Exact rerun:** `ultragoal prove --surface app --json` plus the recorded host probe.
- **Allowed ceiling:** `surface_reconciled` for application discovery.

### CL-RUNTIME — Representative runtime claim

- **Product behavior:** the discovered plugin loads the intended skill/agent guidance and invokes the current authority kernel in a representative fitted repository task.
- **Evidence surface:** task capture, loaded component identity, command capture, local semantic events.
- **Independent reconciliation:** replay the journey from a new task and verify repository behavior and candidate identity outside the plugin’s own assertion.
- **False pass:** application lists the plugin, but the skill is stale, routes to a removed command, or never loads.
- **Fail-closed signal:** `HUL-PLUGIN-001.RUNTIME_JOURNEY_MISMATCH`.
- **Stale/wrong-surface rejection:** plugin/app/repository/context change invalidates it; app registration alone is rejected.
- **Smallest repair:** correct the first broken route or identity transition and rerun only that journey.
- **Exact rerun:** `ultragoal prove --surface runtime --json` plus the named journey replay.
- **Allowed ceiling:** `journey_reconciled` for the exercised journey.

### CL-FIT — Repository fitting claim

- **Product behavior:** inspect/plan/apply/verify fitted the intended repository while preserving user-owned policy and existing workflows.
- **Evidence surface:** before/after semantic snapshots, authorized plan, write capture, host load, representative routine check.
- **Independent reconciliation:** re-read from disk, compare user-owned regions, and execute pre-existing plus new representative workflows.
- **False pass:** expected files exist but existing AGENTS instructions were overwritten or ignored.
- **Fail-closed signal:** `HUL-FIT-001.USER_CHANGE_OR_LOAD_MISMATCH`.
- **Stale/wrong-surface rejection:** repository/instruction/template/host change invalidates it; generated receipt alone is rejected.
- **Smallest repair:** restore the first altered user-owned decision or resolve the first authority conflict.
- **Exact rerun:** `ultragoal fit verify --json`.
- **Allowed ceiling:** `journey_reconciled` for fitting.

### CL-ROUTINE — Routine development claim

- **Product behavior:** changed-surface checks select a legal affected closure, operate on a dirty tree, and return accurate repair without hidden strict/global work.
- **Evidence surface:** query-graph trace, selected tools/tests, command captures, cache decisions, findings, phase timing.
- **Independent reconciliation:** mutation corpus compares routine selection/results to strict no-cache behavior for affected laws.
- **False pass:** a changed dependency is omitted and a stale cached test result returns green.
- **Fail-closed signal:** `HUL-INCREMENTAL-001.AFFECTED_SET_UNPROVEN`.
- **Stale/wrong-surface rejection:** semantic input/tool/cache-environment change invalidates reuse; bookkeeping timing is rejected as speed proof.
- **Smallest repair:** expand the affected closure or recompute the first unverifiable cache node.
- **Exact rerun:** `ultragoal check --changed --json`.
- **Allowed ceiling:** `routine_repair`.

### CL-STRICT-COVERAGE — Strict test/coverage claim

- **Product behavior:** the complete required candidate surface executed under strict cache rules and coverage data maps correctly to semantic inventory.
- **Evidence surface:** compiler/test captures, raw profile data, coverage reports, test list, exclusions, candidate identity.
- **Independent reconciliation:** no-cache rerun or verified same-candidate replay; inject known covered/uncovered/mutated cases.
- **False pass:** stale profdata or a narrow test subset produces an apparently high percentage.
- **Fail-closed signal:** `HUL-COVERAGE-001.STRICT_SCOPE_OR_PROFILE_INVALID`.
- **Stale/wrong-surface rejection:** source/binary/test list/toolchain/profile/exclusion change invalidates it; routine coverage cannot substitute.
- **Smallest repair:** delete only invalid profile nodes and rerun the first missing strict partition.
- **Exact rerun:** `ultragoal prove --surface strict-coverage --json`.
- **Allowed ceiling:** `strict_candidate` for test/coverage only.

### CL-OBSERVABILITY — Observability claim

- **Product behavior:** required semantic events for the current operation are locally queryable, privacy-safe, causally linked, and optionally exported when the requested claim requires export.
- **Evidence surface:** local spool, trace/metric/log records, exporter capture, backend query results.
- **Independent reconciliation:** query by current operation/candidate identity and compare expected emission, local persistence, export, and returned attributes.
- **False pass:** a JSON row exists locally while export is broken, or a dashboard displays an older candidate.
- **Fail-closed signal:** `HUL-OBSERVE-001.ROUNDTRIP_OR_IDENTITY_MISMATCH`.
- **Stale/wrong-surface rejection:** operation/schema/exporter/backend retention change invalidates relevant rungs; local and external planes never substitute for one another.
- **Smallest repair:** repair the first broken emission/persistence/export/query transition and rerun its bounded probe.
- **Exact rerun:** `ultragoal observe query <operation-id> --json` and named roundtrip probe.
- **Allowed ceiling:** `surface_reconciled` for the named observability plane.

### CL-ORCHESTRATION — Adaptive orchestration claim

- **Product behavior:** the root derives a current dependency graph, delegates legal packages, prevents conflicting writes, accepts independently, and recovers stale/blocked/incomplete work.
- **Evidence surface:** work graph versions, ownership decisions, worker reports, root acceptance, conflict/recovery traces.
- **Independent reconciliation:** adversarial scenario suite and read-only reviewer reconstruct the decision from source state.
- **False pass:** workers use separate paths but mutate the same generated authority, or worker self-report is treated as acceptance.
- **Fail-closed signal:** `HUL-WORKER-001.OWNERSHIP_OR_ACCEPTANCE_INVALID`.
- **Stale/wrong-surface rejection:** graph/context/ownership change invalidates unaccepted work; historical lane status is rejected.
- **Smallest repair:** quarantine the conflicting result and serialize the first shared-authority change.
- **Exact rerun:** the orchestration scenario named by the diagnostic; there is no generic receipt-only pass.
- **Allowed ceiling:** `journey_reconciled` for orchestration.

### CL-EVAL-IMPROVEMENT — Evaluation and improvement claim

- **Product behavior:** a reconciled failure becomes a valid evaluation, a scoped candidate improves it without regressions, and independent review approves promotion.
- **Evidence surface:** source trace, failure taxonomy, eval task/test audit, baseline/candidate runs, negative controls, review and rollback.
- **Independent reconciliation:** a non-authoring reviewer reproduces baseline and candidate against validated data and adjacent regressions.
- **False pass:** broken or low-coverage tests make a candidate score higher, or a ranking adapter recommends an unimplemented change.
- **Fail-closed signal:** `HUL-EVAL-001.DATA_OR_PROMOTION_INVALID`.
- **Stale/wrong-surface rejection:** task/test/model/tool/candidate or contamination status change invalidates comparison; source improvement cannot prove installed/runtime improvement.
- **Smallest repair:** correct or quarantine the first invalid eval case, then rerun baseline and candidate.
- **Exact rerun:** `ultragoal prove --surface improvement --json` plus the named eval suite.
- **Allowed ceiling:** `strict_candidate` for the measured surface, never integrated product by itself.

### CL-REAL-JOURNEY — Product-journey claim

- **Product behavior:** an intended operator/agent completes a named plugin lifecycle journey with correct effects, useful repair, and no hidden internal knowledge.
- **Evidence surface:** journey definition, environment/candidate identity, interaction capture, product outputs, outcome and comprehension score.
- **Independent reconciliation:** separate observer or deterministic exerciser checks start condition, transitions, effects, outcome, and recovery.
- **False pass:** a script directly invokes internal commands and bypasses marketplace discovery, plugin guidance, or application behavior.
- **Fail-closed signal:** `HUL-PRODUCT-001.JOURNEY_BYPASS_OR_OUTCOME_MISMATCH`.
- **Stale/wrong-surface rejection:** any required source/package/install/app/runtime/repository transition change invalidates it; unit tests cannot substitute.
- **Smallest repair:** repair the first broken journey transition and replay from its nearest independently verified prerequisite.
- **Exact rerun:** `ultragoal prove --surface journey --json` with the named `PJ-*` identifier.
- **Allowed ceiling:** `journey_reconciled`.

### CL-COMPLETION — Integrated completion candidate

- **Product behavior:** all applicable required laws and lifecycle journeys are current, independently reconciled, secure, maintainable, and at or above the requested ceiling; obsolete authority is retired or explicitly blocked.
- **Evidence surface:** canonical claim graph over all named surface claims, independent reviews, cleanup/retirement state, residual risks, exact candidate identity.
- **Independent reconciliation:** root reproduces required strict evidence and a non-authoring reviewer attempts falsification of the integrated candidate.
- **False pass:** green checklist, source audit, final packet, majority of gates, or worker consensus masks one unproven install/runtime journey.
- **Fail-closed signal:** `HUL-COMPLETION-001.MINIMUM_CEILING_UNSATISFIED`.
- **Stale/wrong-surface rejection:** any relevant candidate, product, environment, review, external surface, or law change invalidates affected claims; the minimum ceiling is recomputed.
- **Smallest repair:** execute the exact next action for the first dependency-ordered blocker.
- **Exact rerun:** `ultragoal prove --surface completion --json` followed by independent review; command availability itself is not proof.
- **Allowed ceiling:** `integrated_completion_candidate`; `released_product` additionally requires release/install/real-use authority.

## Freshness and congruence

Freshness is dependency-based, not a universal time-to-live. A result becomes stale when any declared semantic input, transitive dependency, tool identity, environment predicate, product surface identity, or claim rule changes. Time may be an input for drift-prone external surfaces or retention, but a recent timestamp cannot rescue a wrong candidate.

## Independent reconciliation

Independent means the reconciler does not trust the producer’s interpreted success field and observes the claimed behavior through a separate code path or product boundary. Independence may be provided by:

- a different parser/enumerator over the same immutable bytes;
- execution against a deliberately isolated behavioral fixture;
- host/application observation outside package source;
- a no-cache or verified-replay comparison;
- a non-authoring reviewer reproducing the outcome;
- an end-to-end journey exerciser that cannot shortcut the claimed transition.

Using the same helper, receipt, declaration, or generated row twice is not independent.

## Tamper and false-pass corpus

The strict suite MUST include at least:

- valid shape with false behavior;
- true behavior with tampered receipt;
- stale but recent-looking timestamp;
- current source with wrong package/install/app identity;
- omitted required input from generated inventory;
- cache key missing a transitive dependency;
- local telemetry without export and export from wrong candidate;
- routine subset mislabeled strict;
- worker self-acceptance and semantic write conflict;
- signed artifact with wrong product behavior;
- broken evaluation test producing an apparent improvement;
- completion packet with one lower-ceiling required surface.

## Completion authority rule

The integrated ceiling is the minimum ceiling of every applicable required claim after dependency and surface reconciliation. Optional claims are excluded only through a typed applicability decision. Unknown, stale, invalid, or blocked required claims cannot be averaged away.
