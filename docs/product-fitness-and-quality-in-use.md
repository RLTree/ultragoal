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

## Claim Ceiling

Package/static/fixture Product Fitness proof supports only the plugin
enforcement contract. Live product success, daily-driver fitness, install
visibility, marketplace publication, app registry visibility, and runtime
product proof require fresh same-surface receipts.
