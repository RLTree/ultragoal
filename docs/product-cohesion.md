# Product Cohesion

This source-local product cohesion record covers the Harness Ultragoal plugin
as an agent-facing CLI and plugin product. The primary user is an agentic
engineering operator using the CLI to repair proof-bound repositories without
manual receipt spelunking.

The current product promise is narrow: command output, receipts, query paths,
and claim ceilings must point to the same repair loop so an agent can find the
next root cause and withhold readiness when proof is missing. This record does
not claim live application quality, install/cache visibility, marketplace
exposure, reviewer approval, release readiness, completion, or update_goal
eligibility.

The required source-local journey receipt lives at
`validation_artifacts/product-cohesion/journey-receipt.json`. Its referenced
artifacts are local proof anchors for the `product prove-cohesion` command
pass path and remain mutable validation evidence, not package resources.
