# Authority and Product Laws

## Normative semantics

`MUST`, `MUST NOT`, `REQUIRED`, `SHALL`, and `SHALL NOT` are binding within the adopted contract. `SHOULD` records an advisory default that can be departed from only with rationale and measured consequences. `MAY` is optional. A handoff requirement is not automatically adopted into the live repository; live adoption must be recorded by the Ultra root or authorized human owner.

A law's binding status never depends on whether its validator exists. Missing implementation or proof creates a blocker and lowers claims; it cannot erase the requirement.

## Stable laws

| Law ID | Name | Owner | Binding statement |
| --- | --- | --- | --- |
| HUL-PRODUCT-001 | Integrated plugin-first lifecycle | Product Architecture | Harness Ultragoal is one plugin product whose Rust CLI and Harness-owned tools enable and enforce the lifecycle; no layer may independently claim product completion. |
| HUL-PLUGIN-001 | Standards-compliant discoverable plugin | Plugin Product | The plugin must use supported manifests and discovery paths, expose one concise front door, progressively disclose specialized workflows, and close every routed journey. |
| HUL-FIT-001 | Safe fresh setup and retrofit | Repository Fitting | Repository fitting must inspect before writing, distinguish fresh setup from retrofit, preserve user authority, plan bounded mutations, apply only authorized changes, and verify behavior. |
| HUL-ROUTINE-001 | Fast dirty-tree routine iteration | Routine Development | Routine work must remain useful on dirty trees, select only a legally closed affected set, use verified reuse, avoid hidden global work, and report what actually executed. |
| HUL-STATE-001 | Immutable context and typed current state | Authority Kernel | Authority-bearing operations must consume one immutable live context and derive findings, state, repairs, next action, and claim ceilings from one typed graph. |
| HUL-CLI-001 | Typed explicit-effect CLI kernel | CLI Product | The CLI must use a conventional typed parser, a small product-semantic command surface, structural effect declarations, stable machine output, documented exit codes, and no hidden writes. |
| HUL-REPAIR-001 | Causal diagnostics and exact next action | Operator Experience | Every blocker must explain cause, smallest safe repair, exact rerun, effect, and resulting ceiling; next must deterministically select one legal action without mutation. |
| HUL-OBSERVE-001 | Local-first semantic observability | Observability | Stable semantic events and a zero-dependency local query plane must support diagnosis; optional exports require privacy controls and roundtrip verification and never substitute for behavior proof. |
| HUL-PROOF-001 | Claim-specific independent proof | Proof Authority | Each claim must name its exact behavior, truth surface, current evidence, independent reconciler, false-pass controls, guard, repair, rerun, and allowed ceiling. |
| HUL-ORCHESTRATION-001 | Adaptive root authority and disjoint work | Ultra Root | The Ultra root must recompute live state, own shared authority and integration, delegate read-only reconnaissance, and grant write workers explicit disjoint semantic scopes with independent acceptance. |
| HUL-REVIEW-001 | Independent falsification | Independent Review | Material claims require an actor independent of implementation and primary evidence to attack stale, wrong-surface, bypass, reward-hacking, usability, security, and recovery failure modes. |
| HUL-SECURITY-001 | Confinement, permissions, secrets, and trust | Security | Commands and workers must constrain paths and effects, resist symlink and race attacks, preserve local work, minimize privileges, redact sensitive data, and stop for external or destructive authority. |
| HUL-SUPPLY-001 | Reproducible distribution and integrity | Distribution and Release Engineering | Source, dependency resolution, generated inventory, package bytes, provenance, installed bytes, and observed plugin identity must be linked by digest and verified against expectations. |
| HUL-EVAL-001 | Valid evaluation and behavior improvement | Agent Quality | Evaluations must audit task validity, scoring, contamination, representativeness, reproducibility, and negative controls; improvement must be measured against behavior and independently reviewed. |
| HUL-RESEARCH-001 | Source classification and law promotion | Research Authority | External facts, binding product requirements, advisory practices, hypotheses, and rejected recommendations must remain distinct and traceable to current primary sources. |
| HUL-MAINTENANCE-001 | Migration, compatibility, and retirement | Maintenance and Migration | Replaced lanes, gates, commands, tools, aliases, packages, generated surfaces, and authority stores must be routed or retired under measured compatibility and removal proof. |
| HUL-GOAL-001 | Durable goal integration without simulation | Goal Integration | Harness Ultragoal may integrate with the host durable goal mechanism when exposed but must not simulate host state, infer model or mode from prompts, or merge platform goal state with product state. |
| HUL-COMPLETION-001 | Claim ceiling and honest stop | Human Release Authority | Readiness, release, and completion remain blocked until their exact live proof surfaces pass; unresolved external authority and destructive decisions must stop only the dependent claim. |

## State separation

Every granular requirement carries four independent dimensions:

- **Normative status:** what this contract requires.
- **Live adoption status:** whether the live repository has explicitly adopted it.
- **Implementation status:** what live behavior exists.
- **Proof/claim status:** whether current candidate-bound evidence satisfies a named claim.

No generated report, validator, receipt, event, test, or finalizer may promote any other dimension by existence alone.

## Product architecture

Harness Ultragoal is one plugin-led lifecycle product. The plugin owns user intent, progressive disclosure, repository journeys, observability/repair UX, and quality in use. The Rust CLI owns deterministic context, effects, typed state, acceleration, enforcement, machine interfaces, and evidence capture. Harness-owned custom tools close semantic gaps that ordinary external tools do not cover. External substrates remain replaceable adapters and never own product law or claim authority.
