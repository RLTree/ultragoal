# Proof, Claims, Release, and Completion

## Claim registry

| Claim | Name | Prerequisite claims | Exact truth surface | Independent reconciler | Allowed ceiling | False-pass guard |
| --- | --- | --- | --- | --- | --- | --- |
| CL-SOURCE | Canonical source contract | none | live canonical source tree and adopted contract | independent source/manifest reviewer | source_only | Source coherence proves no package or runtime behavior. |
| CL-PACKAGE | Built package identity | CL-SOURCE | built package bytes and generated package inventory | independent unpack/reinventory verifier | package_only | Package identity proves no install or discovery. |
| CL-INSTALL | Installed bytes | CL-PACKAGE | measured installed bytes in authorized target | independent installer/byte reconciler | installed_bytes_only | Installed bytes prove no host discovery. |
| CL-DISCOVERY | Host discovery | CL-INSTALL | supported host's exposed discovery observation in a new session | independent host probe | discovered_only | Discovery proves no representative route behavior. |
| CL-RUNTIME | Representative runtime behavior | CL-DISCOVERY | representative live plugin and CLI invocations | independent runtime journey reviewer | runtime_observed | One invocation proves only the exercised behavior and environment. |
| CL-FIT | Repository fitting behavior | CL-RUNTIME | fresh and retrofit inspect/plan/apply/verify journeys | independent repository-fit reviewer | repository_fit | Generated files or receipts do not prove loaded behavior or preservation. |
| CL-ROUTINE | Routine development behavior | CL-FIT | dirty-tree affected-set execution and preserved workspace | independent routine-work reviewer | routine_verified | A green subset is valid only if impact closure and reuse are verified. |
| CL-OBSERVABILITY | Observability behavior | CL-RUNTIME | local query behavior and optional export roundtrip | independent observability/privacy reviewer | observability_verified | Event presence cannot prove product behavior or claims. |
| CL-STRICT | Strict validation profile | CL-SOURCE | claim-specific dependency-closed proof with negative controls | independent claim falsifier | strict_profile_verified | Receipts/tests/rows cannot substitute for named behavior. |
| CL-ORCHESTRATION | Adaptive orchestration behavior | CL-SOURCE | live work graph, leases, worker outputs, reviews, and root reconciliation | orchestration recovery reviewer | orchestration_verified | Parallel activity or merged code alone does not prove safe orchestration. |
| CL-EVAL-IMPROVEMENT | Evaluation and improvement behavior | CL-RUNTIME | audited tasks/scorers, paired runs, guarded non-regression, and review | independent evaluation reviewer | improvement_candidate | Metric gain alone does not prove product improvement. |
| CL-REAL-JOURNEY | Representative product journeys | CL-FIT, CL-ROUTINE, CL-OBSERVABILITY | fresh user/agent and maintainer journeys in supported environments | product journey reviewer | journey_verified | Artifact presence or internal tests do not prove quality in use. |
| CL-RELEASE | Release readiness | CL-PACKAGE, CL-INSTALL, CL-DISCOVERY, CL-RUNTIME, CL-FIT, CL-ROUTINE, CL-OBSERVABILITY, CL-STRICT, CL-ORCHESTRATION, CL-EVAL-IMPROVEMENT, CL-REAL-JOURNEY | exact live release proof set, verified package, supported install/discovery, and resolved release decisions | independent release reviewer and human release authority | release_candidate | Release candidate is not integrated product completion. |
| CL-COMPLETION | Integrated goal completion | CL-RELEASE | root-reconciled closure of all required live claims, surfaces, tools, journeys, findings, and decisions | Ultra root plus independent final reviewers | complete | Goal state, green finalizer, telemetry, package, or report cannot substitute for integrated completion. |

`CLAIM_REGISTRY.json` is the machine-readable claim authority. For every claim it names expected behavior, prerequisites, required requirements/surfaces/tools/decisions, required and current evidence, independent reconciler, false-pass controls, guard, repair, rerun, and allowed ceiling. Every current live evidence set is intentionally empty and `not_verified` in this snapshot handoff.

## Evidence envelope

Claim-bearing evidence must bind producer, method, live context ID, candidate identity, time/freshness rule, inputs, environment/tool identity, effects, outputs, artifact digests, and limitations. Producers do not reconcile completion-bearing claims they materially implement or score. Rejected evidence remains visible with reason.

## Prohibited substitutions

The following may support diagnosis or a narrower claim, but never substitute for the named product behavior: test existence, green test output without applicability proof, receipts, generated rows, mutable status files, telemetry/event presence, documentation, signatures, provenance, package bytes, host goal state, or a worker's success summary.

## Strict proof

`prove` selects one claim, computes its dependency-closed proof plan, runs positive and false-pass controls, captures candidate-bound evidence, and sends it to the designated independent reviewer. Unavailable surfaces lower only dependent claims. A strict proof pass cannot cross truth layers.

## Release and completion

`CL-RELEASE` requires the exact live release proof set, verified package/provenance, supported installation/discovery/runtime journeys, retirement state, and resolved release decisions. `CL-COMPLETION` additionally requires every mandatory product surface, Harness-owned custom tool, quality-in-use journey, orchestration behavior, open finding, and required decision to be root-reconciled at its exact ceiling. Release is not completion.

A blocked run reports the highest honest ceiling and does not mark the host goal or product complete.
