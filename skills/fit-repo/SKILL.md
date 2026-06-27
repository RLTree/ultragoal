---
name: fit-repo
description: Mandatory Harness Ultragoal entry contract for fitting a fresh or existing repo before any harnessed, initialized, retrofitted, ultragoal-ready, or material-review-ready claim.
---

# Fit Repo

`fit-repo` is the mandatory product entrypoint for Harness Ultragoal repo setup.
Do not claim a repo is harnessed, initialized, retrofitted, agent-first,
coverage-enforced, ultragoal-ready, or ready for material review without a
schema-valid fit-repo receipt.

## Required Sequence

1. Discover repo root, plugin source root, installed plugin root, cache root,
   and app registry exposure when available.
2. Classify the target as `fresh_repo`, `retrofit_repo`, or
   `blocked_unclassified_repo`.
3. Classify runtime and product surfaces.
4. Install or verify required templates, scripts, schemas, standards rows,
   coverage authority, coverage scope authority, worktree environment setup,
   Product Fitness authority, ultragoal bundle surfaces, and orchestrator
   automation surfaces.
5. Run the fast setup gate and target-repo setup validator.
6. Emit the schema-defined fit-repo receipt in the repo validation artifacts surface.
7. Withhold unsupported claims in the receipt claim ceiling.

## Hard Rules

- Missing setup surfaces become blockers with owner, reason, affected claim ids,
  required repair, and claim ceiling.
- Source, installed plugin, cache package, app registry, and marketplace
  surfaces are distinct proof surfaces.
- Reading docs is not fit-repo proof.
- Reviewer approval is not fit-repo proof.
- Package/static proof is not live app, registry, launcher, or marketplace
  proof.
- Product Cohesion is not Product Fitness. Product-impacting claims require
  Product Fitness receipt proof or explicit claim withholding.
