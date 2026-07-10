# CLI Kernel and Harness-Owned Custom Tools

## Public command groups

| Command | Default effect | Product purpose |
| --- | --- | --- |
| inspect | read | Inspect live context, inventory, capabilities, findings, and claim ceilings. |
| next | read | Select one deterministic legal next action or authority request. |
| fit | read_for_inspect_and_plan; workspace-write_for_explicit_apply | Inspect, plan, apply, and verify fresh setup or retrofit. |
| check | read_plus_declared_local_build_artifacts | Run routine or strict affected validation with verified reuse. |
| diagnose | read | Explain causal failures, repairs, reruns, and ceilings. |
| prove | declared_local_write | Execute claim-specific proof and independent reconciliation. |
| observe | read_for_query; external-write_for_explicit_export | Query local semantic events or explicitly export approved data. |
| package | declared_local_write; external-write_only_for_explicit_publish | Build, inventory, verify, install-test, and prepare distribution. |
| eval | declared_local_write; external-write_only_for_explicit_adapter | Audit and run evaluations, harvest failures, and produce promotion candidates. |
| migrate | read_for_plan; workspace-write_for_explicit_apply; destructive_requires_decision | Route legacy surfaces and verify retirement. |

The public parser must be typed and conventional. Effect class is structural, not inferred from prose. Parse errors, help, version, inspect, next, and query paths must write nothing—including telemetry or caches—unless the user explicitly selects a documented output effect. Machine output is versioned and isolated on stdout; diagnostics use stderr; exit codes distinguish success, actionable finding, invalid invocation, blocked authority, unsupported capability, and internal failure.

## Harness-owned custom tools

| Tool | Name | Owner | Snapshot status | Dependencies | Claims blocked until proof |
| --- | --- | --- | --- | --- | --- |
| HCT-CONTEXT | LiveContext and effect boundary | Authority Kernel | fragmented_partial | none | CL-FIT, CL-ROUTINE, CL-STRICT, CL-ORCHESTRATION, CL-COMPLETION |
| HCT-INVENTORY | Semantic inventory and authority catalog | Product Architecture | fragmented_large_hand_maintained_catalogs | HCT-CONTEXT | CL-SOURCE, CL-PACKAGE, CL-RELEASE |
| HCT-IMPACT | Affected-set graph and verified reuse | Routine Development | partial_heuristic_and_distributed | HCT-CONTEXT, HCT-INVENTORY, HCT-CAPTURE | CL-ROUTINE, CL-STRICT |
| HCT-CAPTURE | Command execution and artifact capture | Authority Kernel | fragmented_receipt_writers | HCT-CONTEXT | CL-PACKAGE, CL-INSTALL, CL-FIT, CL-ROUTINE, CL-STRICT, CL-EVAL-IMPROVEMENT |
| HCT-FIXTURES | Isolated behavioral fixture scheduler | Independent Review | partial_large_fixture_corpus | HCT-CONTEXT, HCT-CAPTURE | CL-ROUTINE, CL-STRICT, CL-EVAL-IMPROVEMENT, CL-RELEASE |
| HCT-STATE | Findings, repair, current state, and next | Operator Experience | partial_duplicate_views | HCT-CONTEXT, HCT-INVENTORY | CL-ROUTINE, CL-ORCHESTRATION, CL-COMPLETION |
| HCT-DISTRIBUTION | Package, install, and discovery reconciler | Distribution and Release Engineering | fragmented_incomplete_host_proof | HCT-CONTEXT, HCT-INVENTORY, HCT-CAPTURE | CL-PACKAGE, CL-INSTALL, CL-DISCOVERY, CL-RUNTIME, CL-RELEASE |
| HCT-FIT | Repository fit reconciler | Repository Fitting | partial_split_init_retrofit | HCT-CONTEXT, HCT-INVENTORY, HCT-CAPTURE, HCT-STATE | CL-FIT, CL-REAL-JOURNEY |
| HCT-OBSERVE | Semantic event store, query, and export | Observability | partial_split_local_external | HCT-CONTEXT, HCT-STATE, HCT-CAPTURE | CL-OBSERVABILITY |
| HCT-EVAL | Evaluation and improvement engine | Agent Quality | partial_vendor_adapter_heavy | HCT-CONTEXT, HCT-CAPTURE, HCT-FIXTURES, HCT-OBSERVE | CL-EVAL-IMPROVEMENT, CL-REAL-JOURNEY |
| HCT-CLAIMS | Claim graph and independent reconciliation | Proof Authority | fragmented_duplicate_finalizers | HCT-CONTEXT, HCT-INVENTORY, HCT-CAPTURE, HCT-STATE | CL-COMPLETION, CL-DISCOVERY, CL-EVAL-IMPROVEMENT, CL-FIT, CL-INSTALL, CL-OBSERVABILITY, CL-ORCHESTRATION, CL-PACKAGE, CL-REAL-JOURNEY, CL-RELEASE, CL-ROUTINE, CL-RUNTIME, CL-SOURCE, CL-STRICT |
| HCT-MIGRATE | Migration router and retirement verifier | Maintenance and Migration | absent_as_single_authority | HCT-CONTEXT, HCT-INVENTORY, HCT-CAPTURE, HCT-STATE, HCT-CLAIMS | CL-REAL-JOURNEY, CL-RELEASE, CL-COMPLETION |

Detailed APIs, external substrates, replacement mappings, implementation contracts, and proof requirements are authoritative in `CUSTOM_TOOL_INVENTORY.json`.

## Tool implementation law

A tool may use Cargo, clap, nextest, tracing/OpenTelemetry, Sigstore/SLSA, Git, or vendor evaluation APIs as substrates, but the Harness semantic contract remains owned locally. Missing optional substrates produce capability findings and bounded fallbacks. Missing required Harness-owned semantics block every downstream claim listed in the tool inventory.

## Performance and dirty-tree policy

Routine work computes a conservative semantic affected set, expands unknown edges, verifies every reuse decision, reports selected/executed/reused/skipped work, and preserves unrelated modified, staged, untracked, and worktree state. Strict proof closes only the dependencies of the named claim; release ceremony is not a routine default.
