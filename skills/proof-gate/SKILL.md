---
name: proof-gate
description: Validate completion claims, receipts, evidence surfaces, verification backlog, and false-completion red fixtures.
---

# Proof Gate

Use this skill before accepting lane completion, final completion, or any production/ready claim.

## Proof Surfaces Are Not Interchangeable

Runtime CLI proof does not prove UI creator proof. Fixture proof does not prove live use. Static tests do not prove installed app behavior. Browser screenshots do not prove API persistence. Every claim names its proof surface.
Engine/runtime proof does not prove product cohesion or Product Fitness.
Consumer-facing product claims need a product-cohesion receipt plus UI journey
evidence, and product-impacting success/readiness claims need a Product Fitness
receipt bound to audience, job, context, outcome, accessibility, cognitive load,
recovery, and continuance when repeated use is claimed.
Source-tree proof does not prove installed plugin, cache package, app registry,
marketplace, multi-agent launcher, toolbar, or runtime visibility. Browser,
UI, recording, replay, transcript, and video-alignment claims need same-surface
runtime identity and coordination receipts, not just CLI or fixture evidence.

## Claim Classification

Classify each claim using the canonical `ClaimStatus` enum in `../../schemas/common-defs.schema.json`. Positive claim ceilings may include only `proven_live` and narrowly scoped `proven_static`.

## Required Checks

Run `ultragoal-audit`; every required check id owned by
`../../schemas/common-defs.schema.json#/$defs/requiredValidatorCheckId` must pass.
Summaries may group the results by claim closure, evidence coupling, backlog
coverage, lane hygiene, validator provenance, red fixtures, and target-repo
audit capability, but prose is not a second checklist authority.
The `agent-standards-enforcement` check is part of the proof gate. Standards
rows are authority only when typed as mechanized, backlogged, blocked, or
informational; unclassified rows block completion. Reviewer approval cannot
close a standards obligation by itself.

Before launching reviewers, run the material review scope gate. Material
sign-off, release, promotion, readiness, production-use, phase advancement,
material code/runtime change, package/cache/app/marketplace/launcher/UI/runtime
visibility change, security/privacy/trust-boundary change, reviewer
model/persona/registry change, proof-anchor change, or repaired
`REVISE_BEFORE_NEXT_PHASE`/`BLOCKED` round requires full-scope material review.
Delta and advisory review outputs cannot be used as material `SIGN_OFF`.

When an Ultragoal contract delegates idle orchestration to automation, missing
automation binding, stale automation ticks, generic automation prompts,
unresolved activation placeholders, or thread/workspace mismatch lower the
claim ceiling until repaired by the automation contract and tick receipt.

## Refusal Rules

Reject completion when:

- evidence is stale and is the current claimed proof authority;
- proof surface is substituted;
- a named plugin, process, installed package, app registry, or runtime authority
  silently falls back to a source checkout, local clone, installed cache,
  reference package, or alternate tool without an authority-source receipt and
  explicit claim ceiling;
- a plugin availability claim relies on source package checks without separate
  source, installed-plugin, cache-package, package-sync, package-hygiene, and
  per-surface receipts;
- browser, UI, or runtime proof omits exact tool identity, version, binary path
  when relevant, artifact digests, workspace, or claim ceiling;
- CLI checks or fixtures are used as proof for live recording, live UI, replay,
  transcript alignment, video alignment, or parent-operation coordination;
- a proof-bearing generated artifact contains behavior-affecting derived
  authority values that were not recomputed from canonical current inputs or
  bound to a digest derived from those inputs;
- post-stop batch transcription, cleanup, or final alignment is missing or
  disabled while downstream claims trust final words;
- validators, review prompts, fixtures, or reports open, persist, or quote raw
  private transcript, audio, prompt, message, or event payloads instead of using
  digests, redacted snippets, category-only status, or leak summaries;
- a ready receipt is hand-written;
- a blocker lacks a real provisioning/escalation/install attempt when such an attempt was available;
- a product claim lacks product journey proof or overuses human handoff as a default path;
- a product-impacting claim uses reviewer agreement, install success, smoke
  tests, test pass counts, fixture pass counts, package publication, first use,
  feature delivery, or Product Cohesion alone as Product Fitness proof;
- a lane leaves a dirty worktree without explicit preserved paths;
- stale worktrees or sessions remain active;
- the final report claims more than the claim ceiling.

Do not turn detached or historical receipt drift into review theater. If a
receipt mismatch is outside the current claimed proof surface, identify the
current authority and either regenerate the stale artifact or record a cleanup
note. Fail only when the mismatch weakens the current claim, proof anchor, or
claim ceiling.
