# Review Loop Record

## Purpose

This file is the current review-history index. It is evidence routing, not
review law, proof authority, or standing approval.

Current review outcomes must be represented in typed review receipts bound to
the same validator, review-target, and archive anchors. Markdown review reports
are narrative attachments only.

## Current Authorities

- Review-round receipt schema: `schemas/review-round-receipt.schema.json`.
- Review-round fixture authority: `fixtures/review-round/valid/review-round-receipt.json`.
- Review-round anchor fixtures: `fixtures/review-round/anchors/`.
- Review persona prompts: `agents/contract-claim-falsifier.md`,
  `agents/orchestration-recovery-falsifier.md`,
  `agents/security-trust-boundary-falsifier.md`, and
  `agents/product-simplicity-falsifier.md`.
- Detached review-target and archive contract:
  `docs/review-target-and-archive.md`.

## Historical Context

Raw historical reviewer prose is intentionally not shipped as an always-loaded
package document. Current agents should use typed receipts, fixture reports, and
the compact index in this file instead of stale narrative logs.

## Review Cadence

- Every material review round is a sign-off attempt using all four canonical
  personas with fresh context, full scope, `gpt-5.5`, and `high`.
- Each reviewer checks the current validator receipt, review-target digest,
  archive receipt when relevant, active registry exposure, and claim ceiling.
- All four reviewers must return `SIGN_OFF` in the same round. Any
  `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED` invalidates the round; repair,
  regenerate anchors, close reviewers, and launch a fresh full-scope round.

Any missing persona, reused reviewer, stale anchor, or prose-only approval
invalidates the round.

The earlier separated six-role team remains historical context. It is not a
current approval team unless a future typed receipt/schema migration restores
it.
