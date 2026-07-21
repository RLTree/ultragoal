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

Reviewer agreement, install success, smoke tests, test pass counts, fixture pass
counts, package publication, first use, feature delivery, and Product Cohesion
receipts alone MUST NOT prove product success.
