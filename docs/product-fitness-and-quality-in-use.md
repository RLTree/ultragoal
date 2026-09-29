# Product Fitness And Quality-In-Use

Product Fitness is a mandatory proof gate for product-impacting claims.

Product Cohesion proves journey coherence. Product Fitness proves that the
journey fits a named audience, job, context, desired user outcome, quality-in-use
bar, accessibility burden, cognitive burden, recovery burden, and continuance
claim.

## Law

Every product-impacting claim MUST attach a schema-valid Product Fitness receipt
or MUST withhold the claim with blocker, owner, reason, affected claims, required
follow-up, and claim ceiling.

Product-impacting claims include user-facing, operator-facing, developer-facing,
control-surface, workflow-launcher, dashboard, run-console, plugin, install,
marketplace, and daily-driver claims.

The following artifacts MUST NOT substitute for Product Fitness proof:

- reviewer agreement;
- install success;
- smoke tests;
- test pass counts;
- fixture pass counts;
- package publication;
- first use;
- feature delivery;
- Product Cohesion receipts alone.

## Required Receipt

The authoritative receipt path is:

```text
validation_artifacts/harness/product-fitness-receipt.json
```

The receipt MUST bind the exact claim id to target revision, target audience,
job to be done, context of use, desired user outcome, business or mission
outcome, critical journey, first value event, assumption tests, user evidence,
quality-in-use metrics, accessibility gate, cognitive-load gate, recovery gate,
continuance signal when repeated use is claimed, proof surface, actor-disjoint
review, and claim ceiling.

The v2 receipt keeps this one canonical receipt family and adds typed operator,
evidence-class, independent source-to-journey identities, public-entry/bypass,
real-work, and minimal manual journey observations. Every identity is observed
or explicitly withheld. Legacy dogfood receipts remain non-product
orchestration records and are forbidden substitutions for this receipt.

The v2 receipt declares one `claimed_surface` and a ceiling for every truth
surface. The claimed ceiling must equal that surface's ceiling. A live claim
requires current same-surface evidence for the claimed surface and the
dependency predecessors needed to reach it. Later or unrelated surfaces remain
withheld without lowering an earlier valid claim: installed evidence can prove
an observed marketplace surface, while journey and human-use claims still
require a current journey observation.

The typed disposition keeps v1 compatibility and validates v2 operator kind,
evidence class, exact observed-or-withheld surface identities, public entry and
bypass rejection, real repository/task outcome, manual journey observations,
candidate freshness, per-surface claim-ceiling support, and an explicit legacy
dogfood substitution rejection. `dogfood-receipt.v1` is rejected by both typed
and audit Product Fitness paths.

## Claim Ceiling

Package/static/fixture Product Fitness proof supports only the plugin
enforcement contract. Live product success, daily-driver fitness, install
visibility, marketplace publication, app registry visibility, and runtime
product proof require fresh same-surface receipts.
