# ExecPlan: Mandatory Product Fitness And Quality-In-Use Enforcement

Contract status: REQUIRED by `harness-ultragoal-plans-and-orchestrator-automation-hardening.md`. This file is incorporated into every initialized or retrofitted Harness Ultragoal repo that inherits the plugin.

## Purpose

Product Cohesion proves that product surfaces connect into one coherent journey.
It does not prove that the journey is right for the target audience, job,
context, outcome, repeated use, accessibility burden, cognitive burden, or trust
burden.

This ExecPlan adds the missing authority layer: Product Fitness and
Quality-In-Use. Agents do not decide strictness. The repo contract dictates
strictness. Validators enforce strictness. Unsupported product claims fail.

## Evidence Basis

This plan consumes package-contained, redacted Deep Research V2 evidence from:

```text
docs/product-fitness-research-evidence.json
```

The clean run produced 25 claims, 75 verifier proof rows, and 137 source rows.
The final report-shape step failed from input-size pressure. The typed claims,
proofs, and sources are summarized in the package-contained evidence artifact.
Raw temporary export paths are not shipped proof authority. Narrative
projections are context only.

## Non-Negotiable Result

Every repo initialized or retrofitted by Harness Ultragoal MUST enforce Product
Fitness for every user-facing, operator-facing, developer-facing,
control-surface, workflow-launcher, dashboard, run-console, plugin, install,
marketplace, or daily-driver product claim.

A repo CANNOT claim product readiness, customer readiness, operator readiness,
daily-driver readiness, install success as product success, onboarding success
as product success, phase advancement for a product surface, material product
sign-off, production readiness, or release readiness unless one hard state is
true:

1. A schema-valid Product Fitness receipt supports the exact claim id, target
   audience, job, context, desired outcome, product journey, evidence surface,
   quality-in-use measures, accessibility gate, cognitive-load gate, repeated-use
   signal, and claim ceiling.
2. The repo withholds the claim with a blocker, owner, reason, required
   follow-up, affected claim ids, and claim ceiling.

There is no opinion-only path. There is no reviewer override. There is no test
pass substitution. There is no install substitution. There is no first-use
substitution for repeated-use proof.

## Mandatory Receipt

Every product-impacting claim MUST attach:

```text
validation_artifacts/harness/product-fitness-receipt.json
```

The receipt schema MUST require: schema id, receipt version, claim id, target
revision or source tree digest, target audience, job to be done, context of use,
desired user outcome, business or mission outcome, critical journey, first value
event, continuance or repeat-use signal, assumption tests, user evidence,
quality-in-use metrics, accessibility gate, cognitive-load and recovery
evidence, proof surface, claim ceiling, producer actor id, reviewer actor id,
actor-disjoint verdict, generated_at timestamp, and receipt digest.

The validator MUST reject missing, stale, mismatched, hand-written,
actor-nondisjoint, low-evidence, unsupported, or claim-detached receipts.

## Mandatory Laws And Error Codes

| Law | Enforcement objective | Required error codes |
| --- | --- | --- |
| Audience-Job-Context-Outcome Binding | Every product claim names exact audience, job, context, and desired outcome. Generic users, vague operators, broad teams, and unnamed workflows fail. | `product_fitness_audience_missing`, `product_fitness_job_missing`, `product_fitness_context_missing`, `product_fitness_outcome_missing`, `product_fitness_generic_user_claim` |
| Discovery Before Delivery | Material product claims name assumption tests before delivery proof is accepted. Building the feature is not evidence that the product is right. | `product_fitness_discovery_missing`, `product_fitness_assumption_test_missing`, `product_fitness_delivery_substituted_for_discovery` |
| Opinion Is Not Validation | Praise, reviewer confidence, stakeholder opinion, design taste, prompt agreement, and internal belief fail as Product Fitness proof. User evidence binds to audience, job, context, and outcome. | `product_fitness_opinion_only`, `product_fitness_reviewer_substituted_for_user_evidence`, `product_fitness_unbound_user_evidence` |
| Install And First Use Are Not Success | Install visibility, launch success, smoke tests, onboarding completion, and first run prove reachability only. They fail as product success, repeated-use proof, or daily-driver proof. | `product_fitness_install_substituted_for_success`, `product_fitness_first_use_substituted_for_success`, `product_fitness_smoke_test_substituted_for_success` |
| Multidimensional Success | Product success measures user outcome, quality-in-use, trust burden, accessibility burden, cognitive burden, recovery burden, and business or mission outcome. Single-metric success claims fail. | `product_fitness_single_metric_theater`, `product_fitness_quality_in_use_missing`, `product_fitness_business_or_mission_outcome_missing`, `product_fitness_recovery_burden_missing` |
| Quality-In-Use Gate | Product-impacting claims prove quality in the user's actual context of use: effectiveness, efficiency, satisfaction, freedom from risk, and context coverage. Lab-only, fixture-only, and happy-path-only claims fail unless the claim ceiling withholds real use. | `product_fitness_quality_in_use_receipt_missing`, `product_fitness_context_of_use_unproven`, `product_fitness_happy_path_only`, `product_fitness_fixture_substituted_for_real_use` |
| Accessibility As Product Fitness | Accessibility evidence binds to the critical journey. Missing accessibility evidence fails. | `product_fitness_accessibility_missing`, `product_fitness_accessibility_unbound_to_journey` |
| Continuance And Habit Evidence | Daily-driver, repeated-use, workflow, retained-use, and habitual-use claims attach continuance evidence. A single successful run fails as repeated-use proof. | `product_fitness_continuance_missing`, `product_fitness_daily_driver_overclaim`, `product_fitness_single_run_substituted_for_retention` |
| Outcome Over Output | Feature completion, UI delivery, command availability, test pass counts, fixture pass counts, and package publication fail as proof that users achieved the desired outcome. | `product_fitness_output_substituted_for_outcome`, `product_fitness_test_pass_substituted_for_user_value`, `product_fitness_package_publication_substituted_for_product_success` |
| Developer Experience Burden Gate | Developer-tool product claims prove the target developer completes the intended job with lower setup, cognitive, recovery, and trust burden. Powerful engines behind confusing entrypoints fail. | `product_fitness_developer_burden_unmeasured`, `product_fitness_entrypoint_confusion_unresolved`, `product_fitness_power_hidden_by_interface` |

## Mandatory Setup Surfaces

Every initialized or retrofitted repo with product-impacting surfaces MUST
contain or explicitly block `PRODUCT_FITNESS.md`,
`validation_artifacts/harness/product-fitness-receipt.json`,
`schemas/product-fitness-receipt.schema.json` or installed schema reference,
standards row `STD-PRODUCT-FITNESS-001`, Product Fitness validator wiring, red
fixtures for bypasses, green fixtures for valid proof, and a claim-ceiling path
for withheld product claims.

Missing surfaces fail initialization, retrofit, material product review, and
phase advancement.

## Mandatory Validator Work

The plugin validator MUST enforce product-impacting claim classification,
receipt presence, schema validity, freshness, claim id binding, target revision
binding, audience/job/context/outcome binding, discovery proof, user evidence
binding, quality-in-use dimensions, accessibility proof, cognitive-load proof,
recovery proof, continuance proof for repeated-use claims, substitution
rejection, claim ceiling downgrades, and engine-only false-positive protection.

The validator MUST keep Product Cohesion and Product Fitness separate. Product
Cohesion proves journey coherence. Product Fitness proves audience, job,
context, outcome, and quality-in-use fit. Product claims that need both receipts
fail when either receipt is missing.

## Mandatory Fixtures

Red fixtures MUST prove failure for missing Product Fitness receipt; missing
audience, job, context, outcome, or discovery evidence; opinion-only validation;
reviewer sign-off used as user evidence; install, first run, smoke test, test
pass, package publication, feature output, or single metric used as product
success; missing accessibility, cognitive-load, recovery, or continuance
evidence; daily-driver claim without continuance evidence; developer-tool power
hidden behind confusing entrypoints; stale receipt; cross-claim receipt reuse;
engine-only false-positive; unsupported feature/date roadmap promise; and
mandatory-law section containing discretionary modal language.

Green fixtures MUST prove a valid product claim with Product Cohesion and
Product Fitness receipts, valid control-surface claim with audience/job/context
binding, valid developer-tool claim with burden-reduction evidence, valid
daily-driver claim with continuance evidence, valid engine-only runtime claim,
and valid withheld product claim with blocker, owner, reason, required follow-up,
affected claim ids, and claim ceiling.

Every red fixture MUST fail for the intended error code. Every green fixture MUST
pass. Any mismatch blocks release.

## Mandatory Standards Enforcement

Add standards row:

```text
STD-PRODUCT-FITNESS-001
```

The row MUST identify source law, required behavior, deterministic enforcement
gate, fixture group, owner, affected claim ids, current status, blocker, and
repair action.

`unknown`, `unclassified`, `informational`, `reviewer-memory-only`, and
`prompt-only` status fail for this row. Reviewer approval does not close this
standard. Deterministic enforcement closes it. An explicit blocker with withheld
claims narrows the claim ceiling.

## Mandatory Skill And Template Routing

The plugin MUST route agents through Product Fitness without user reminders:
`harness-ultragoal:fit-repo` classifies requirements;
`harness-ultragoal:agent-first-repo-init` installs surfaces when
product-impacting surfaces exist; `harness-ultragoal:agent-first-repo-retrofit`
audits product claims and installs or blocks surfaces;
`harness-ultragoal:product-cohesion-gate` routes product-impacting claims to
Product Fitness instead of treating cohesion as sufficient;
`harness-ultragoal:proof-gate` rejects product success substitutions;
`harness-ultragoal:standards-gardener` promotes repeated misses into
deterministic gates and fixtures; `harness-ultragoal:orchestrator-reconciler`
treats missing Product Fitness proof as a material product sign-off blocker.

## Mandatory Strict-Language Guard

Every template, standards row, validator message, and skill section that defines
Product Fitness law MUST use mandatory language. Discretionary modal language in
tagged law sections fails template integrity. The validator MUST load the banned
token list from configuration and scan tagged law sections only.

## Exact Commands

Implementation MUST run and report:

```bash
cargo fmt --check
cargo test
./scripts/check
./scripts/check-agent-standards
python3 -B tools/ultragoal_audit.py --root . --receipt validation_artifacts/ultragoal-audit/validator-receipt.json
```

If the repo-native Rust validator replaces the Python command, the final report
MUST name the Rust command, the compatibility command, and the reason Python is
not authoritative.

## Validation And Acceptance

This ExecPlan is satisfied only when Product Fitness schema, template, standards
row, validator gates, red fixtures, and green fixtures exist; red fixtures fail
for intended error codes; green fixtures pass; product-impacting claims fail
without Product Fitness proof; engine-only runtime claims do not falsely require
Product Fitness proof; Product Cohesion and Product Fitness remain separate;
setup and retrofit flows install or block Product Fitness surfaces;
`scripts/check` or the repo-native validator runs the gate; plugin
manifest/package/cache surfaces include every new shipped file; a fresh validator
receipt records the new check and fixture counts; the claim ceiling withholds
live product success without same-surface evidence; and material review receives
this ExecPlan as current scope.

## Live Beneficial End-To-End Proof

Fixture proof supports package/static/fixture claims only.

Live product success requires same-surface evidence from the target product:
target audience or representative operator, actual job and context of use,
critical journey execution, first value event, outcome evidence, accessibility
evidence, cognitive-load and recovery evidence, continuance evidence for
repeated-use claims, and typed Product Fitness receipt.

Without those artifacts, the claim ceiling remains package/static/fixture proof.

## Idempotence And Recovery

Setup or retrofit MUST create missing Product Fitness surfaces or write a
blocker. Re-running setup MUST NOT duplicate rows, receipts, fixtures, or claim
ids. Stale receipts MUST be regenerated or blocked. Hand-edited receipts MUST
fail digest validation.

## Artifact Paths

Planned authority artifacts:

```text
docs/product-fitness-and-quality-in-use.md; templates/PRODUCT_FITNESS.md; templates/PRODUCT_FITNESS_RECEIPT.json
schemas/product-fitness-receipt.schema.json; validator/src/claim_semantics/product/fitness.rs; validator/src/audit/product/fitness.rs
fixtures/red/product-fitness-*.json; fixtures/target-repo/product-fitness-*/
validation_artifacts/harness/product-fitness-receipt.json; validation_artifacts/harness/product-fitness-ready-receipt.json
```

## Claim Ceiling

This ExecPlan proves only that the Product Fitness enforcement contract exists.
It does not prove implementation, live product success, user adoption, market
fit, install visibility, marketplace publication, or daily-driver fitness. Those
claims require fresh same-surface receipts.

## Parent-Thread Completion Message Contract

When the lane owner claims this ExecPlan is ready for review, the message to the
parent MUST include changed files, implemented laws, Product Fitness check id,
standards row status, red fixtures, green fixtures, exact commands and exit
codes, validator receipt path, package digest, unsupported claims, blockers, and
claim ceiling.
