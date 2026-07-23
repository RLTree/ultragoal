# Product Fitness And Quality-In-Use

Product Fitness is mandatory at a product-impacting claim boundary.

Every promoted product-impacting claim MUST attach
`validation_artifacts/harness/product-fitness-receipt.json` or MUST withhold the
claim with blocker, owner, reason, affected claim ids, required follow-up, and
claim ceiling.

During implementation, keep Product Fitness observations manual and ephemeral.
Persist the single canonical receipt only when a product, daily-driver,
broad-reuse, readiness, release, or completion claim consumes it. Do not emit a
receipt after every edit, lane freeze, or source-only review.

Product Fitness is separate from Product Cohesion. Product Cohesion proves
journey coherence. Product Fitness proves audience, job, context, outcome, and
quality-in-use fit.

Product Fitness proof MUST bind:

- target audience;
- job to be done;
- context of use;
- desired user outcome;
- business or mission outcome;
- critical journey;
- first value event;
- assumption tests;
- user evidence;
- effectiveness, efficiency, satisfaction, freedom from risk, and context
  coverage;
- accessibility evidence bound to the journey;
- cognitive-load and recovery burden;
- continuance evidence for repeated-use or daily-driver claims;
- claim ceiling.

The minimal manual-first journey row records time to verified value, human
interventions, review rounds, recovery outcome, retained artifact/cache cost,
and any observed false pass or false rejection. Do not create telemetry
infrastructure solely to collect this row.

Product Fitness v2 also records the operator kind and evidence class; exact
source, package, marketplace, install, cache, app-registry, discovery, runtime,
and journey identities as separately observed or withheld; the public entry and
any rejected bypass; the real repository, task, useful outcome, representative
failure, diagnosis, recovery, and repeat-use result. Agent-use evidence cannot
be relabeled as human use. A legacy `dogfood-receipt.v1` is orchestration and
cleanup context only and cannot satisfy Product Fitness, real-use, human-use,
daily-driver, readiness, release, or completion evidence.

Every v2 receipt names one claimed surface and one ceiling for every truth
surface. A live claim requires current same-surface observation for that
surface and its required predecessors; it does not require unrelated later
evidence. Installed evidence can support an observed marketplace surface
without pretending a human journey occurred. Journey and human-use ceilings
still require current journey observation, and repeated human use remains the
only evidence class that can support continuance.

The typed Product Fitness disposition accepts v1 records unchanged. A v2
disposition requires every v2 observation, candidate/current same-surface
evidence, exact surface identities, a rejected bypass when attempted, and an
honest per-surface ceiling. The audit path rejects `dogfood-receipt.v1`
explicitly and requires v2 receipts to name it as a rejected substitution.

Reviewer agreement, install success, smoke tests, test pass counts, fixture pass
counts, package publication, first use, feature delivery, and Product Cohesion
receipts alone MUST NOT prove product success.

Agentic Engineering adoption is evaluated only through the existing Product
Fitness authority after a reproducible installed journey exists. Compare the
same Harness route without the relevant advisory lens and with automatic
selection while holding repository state, candidate, model and reasoning
configuration, tools, permissions, acceptance criteria, reviewer, and claim
ceiling constant.

Record routing and authority correctness, accepted outcome, material defects,
false completion, missed and unnecessary activation, unsafe-effect attempts,
correction and review rounds, human interventions, context and token use,
elapsed time and cost, time to verified value, recovery, repeat use, and
unsupported claims. Repeat only when the arms disagree, nondeterminism blocks a
decision, or a protected failure needs confirmation. Adopt a mechanism only
when it preserves every protected boundary and improves a meaningful product
measure without materially worsening the others.

Structural skill coverage, generated reports, upstream confidence, agent use,
and one successful journey do not prove human usefulness or Product Fitness.
Routine advisory observations remain ephemeral until the single current
Product Fitness receipt consumes them at a claim boundary.
