---
name: product-cohesion-gate
description: Use when a goal changes a user-facing product, UI, dashboard, control surface, workflow launcher, or consumer-visible feature.
---

# Product Cohesion Gate

Use this skill when a goal can change how a user understands, starts,
monitors, trusts, or completes work through a product surface.

This gate is conditional. Do not invoke it for isolated backend, schema, or
infrastructure work unless the goal, claim, or user explicitly depends on a
consumer-facing product experience.

For plugins and agent-first harnesses, the agent workflow itself is a product
surface. If users must manually remember which plugin skill, agent, script, or
receipt comes next, treat that as a Product Cohesion smell and update the
plugin resource map, skill routing, or standards module with the smallest
durable fix.

## Purpose

Engine correctness is not product success. A product-surface claim needs proof
that the power is presented as a coherent user journey:

- who the surface is for;
- what job the user is trying to complete;
- what product promise is being made;
- which surfaces the user travels through;
- how the engine state, proof owed, receipts, and claim ceiling are visible;
- when human attention is truly required and why the agent could not continue;
- what is deliberately withheld when only mechanics are proven.

Product Cohesion is not Product Fitness. Product success, readiness,
daily-driver, release, material product sign-off, and quality-in-use claims MUST
also route through `harness-ultragoal:product-fitness-gate`.

## Required Artifacts

When product cohesion is required, the target repo should expose:

```text
docs/product-cohesion.md
validation_artifacts/product-cohesion/journey-receipt.json
```

The repo gate should emit:

```text
harness-check:product-cohesion pass
```

If the product surface is runnable, the receipt must cite screenshot,
accessibility, browser, or computer-use artifacts for the named journey. A
static product brief can guide implementation, but it cannot support a
`proven_live` product claim by itself.

## Procedure

1. Name the product surface and primary user.
2. Write the product promise in concrete user language.
3. Map the critical journey from intent to outcome across screens, commands, or
   control surfaces.
4. Name the engine/runtime state exposed at each step.
5. Define the human-attention policy: what the agent handles automatically,
   what is allowed to interrupt the user, and what evidence proves the
   interruption was necessary after harness-owned paths were exhausted.
6. Capture UI proof for the journey, including empty/loading/error/blocked
   states when those states affect trust.
7. Record what proof is missing and lower the claim ceiling instead of filling
   the gap with engine-only evidence.

## Refusal Rules

Reject or withhold the product claim when:

- runtime, CLI, API, or unit-test proof is used as a substitute for UI journey
  proof;
- the product promise is not visible in the flow being claimed;
- screens are individually functional but the user cannot tell where to start,
  what happened, what needs attention, or what proof backs the result;
- blocked or unsupported product states are hidden behind optimistic copy;
- "Needs you" or equivalent human-handoff states become a default routing path
  instead of an exceptional, evidenced escalation;
- human attention is requested without evidence that route selection, retries,
  receipt lookup, blocker classification, or other autonomous recovery paths
  were exhausted;
- the journey receipt lacks a non-zero digest or names only fixtures, mock
  screenshots, dummy data, or placeholder artifacts.
- product success is claimed from Product Cohesion without Product Fitness
  proof.
