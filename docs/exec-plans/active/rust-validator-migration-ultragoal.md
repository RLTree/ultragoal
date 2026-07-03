# Rust-First Validator Migration Contract

This is the active contract for the Rust-canonical Harness Ultragoal validator.
It must stay under 250 lines. Preserve the detailed history in receipts and
review records, not by growing this file into a mega-document.

## Truth Standard

Rust is the deterministic contract authority. Model-assisted semantic
classification produces typed receipts for non-deterministic language
interpretation. Rust validates those receipts, blocks stale or contradictory
claims, and preserves package/static/fixture proof boundaries without calling a
model during full audit.

Do not claim that Rust solves semantics. Do not claim live install, app
visibility, marketplace publication, real dogfood, or external product UX proof
until fresh same-surface receipts exist.

## Current Authority

- Canonical validator: `validator/`
- Canonical command:
  `cargo run --offline -- --root . audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json`
- Canonical receipt:
  `validation_artifacts/ultragoal-audit/validator-receipt.json`
- Python validator/reference/wrapper code is retired and must not return as
  authority, parity oracle, wrapper, or historical fallback for the supported
  audit surface.
- Source edits stale prior package digests, review-target receipts, archive
  receipts, and reviewer signoff.

## Required Reading

- `templates/AGENT_STANDARDS.md`
- `templates/agent-standards/`
- `docs/hypercritical-review-law.md`
- `docs/implementation-roadmap.md`
- `docs/product-cohesion-gate.md`
- `docs/source-article-synthesis.md`
- `docs/source-cards.json`
- `docs/schema-resolver.md`
- `docs/review-target-and-archive.md`
- `docs/review-loop-record.md`
- `schemas/completion-manifest.schema.json`
- `schemas/schema-authority-primitives.schema.json`

## Article-Derived Constraints

- Harness Engineering: repository knowledge is the system of record; use routed
  law files, not one giant prompt.
- Symphony: orchestration truth is durable task/workspace state; summaries are
  observability, not authority.
- Exec Plans: long-running work needs restartable progress, decisions,
  validation, and recovery.
- Parse, Don't Validate: typed boundary parsing should make illegal states hard
  to construct.
- AI Is Forcing Us To Write Good Code: small files, clear namespaces, isolated
  environments, and discoverable commands are throughput requirements.

## Hard Requirements

- Rust enforces typed schemas, path containment, digest integrity,
  catalog-only schema resolution, structured errors, package hygiene, fixture
  execution, target-repo checks, deterministic review-target receipts, and
  deterministic archive receipts.
- Full audit must not call a model or the network.
- Semantic-classification receipts bind claim id, text digest, contract id and
  version, classifier kind, model metadata when used, prompt digest when used,
  generated timestamp, producer actor, classifier actor, actor-disjoint verdict,
  semantic classes, rationale, confidence, ambiguity, proof gates, claim-ceiling
  recommendation, and receipt digest.
- Missing, stale, digest-mismatched, contradictory, low-confidence,
  actor-nondisjoint, malformed, or ambiguous semantic receipts fail cleanly or
  require reviewer classification.
- Deterministic lexical tripwires remain cheap backstops and are documented as
  enumerated lexical families only.
- Review authority lives in typed review receipts. Markdown reviewer reports
  are bound narrative artifacts for observability and must not decide claim
  ceilings, blocker state, counterexample coverage, or proof authority.
- Free-text interpretation never becomes proof authority directly. Known
  deterministic tripwires catch enumerated bypass families; open-ended language
  interpretation requires detached model or human semantic-classification
  receipts that Rust validates as untrusted input.
- Expected failures produce structured errors and receipts where applicable.
- Proof artifacts must be repo-contained, byte-digest matched, non-placeholder,
  non-mock, non-fixture proof unless explicitly target-fixture scoped.
- Source files target 100-200 lines with a hard 250-line cap unless generated
  or explicitly justified. Functions target under 60 lines.
- New dependencies require a purpose, alternatives, license, attack-surface,
  maintenance, and runtime-cost receipt.

## Required Fixture Classes

Preserve red fixtures for:

- product/UI paraphrase smuggling: native client, desktop client, graphical
  shell, GUI inspect/control, dashboard, run console, workflow launcher;
- install visibility smuggling: extension/add-on/plugin present, active,
  selectable, registered, chosen, loaded, enabled, running;
- publication smuggling: catalog, directory, gallery, marketplace, workspace
  registry;
- semantic receipt missing, stale, malformed, contradictory, ambiguous,
  low-confidence, actor-self-classified, wrong claim id, unknown semantic
  class, or unknown proof gate;
- deterministic audit attempting model/network access;
- schema resolver attempts for `http`, `https`, `ftp`, `file`, `data`,
  uncatalogued absolute ids, and absent relative refs;
- malformed manifest, valid fixture, red catalog, missing JSON patch, malformed
  JSON pointer, and missing base fixture;
- target-repo symlink/path-escape/digest/placeholder attacks;
- review-round persona/model/fresh-context/digest/claim-ceiling violations.

Preserve green fixtures for:

- app server running;
- CLI command available;
- backend worker active;
- schema/static validation complete;
- engine-only runtime receipt with no product claim;
- non-product proof-surface negative probe;
- valid semantic receipt for product work requiring Product Cohesion gates;
- valid semantic receipt for true runtime-only work.

## Review Cadence

Material sign-off rounds:

- all four merged personas;
- `gpt-5.5`, `high`;
- full scope every round;
- fresh reviewers every round;
- current validator, review-target, archive, registry, and claim-ceiling
  anchors when relevant;
- all four must return `SIGN_OFF` in the same round.

Required current personas:

1. Contract & Claim Falsifier.
2. Orchestration & Recovery Falsifier.
3. Security Trust-Boundary Falsifier.
4. Product & Simplicity Falsifier.

The retired legacy review team is historical context only. Do not use retired
prompt packets, retired custom-agent TOMLs, or generic substitute reviewers for
material sign-off rounds.

## Validation Before Any Completion Claim

Report exact commands, exit codes, counts, artifacts, and residual gaps for:

- Rust formatting;
- Rust unit tests;
- Rust full audit;
- red fixture count;
- generated artifact count;
- manifest inventory closure;
- target-repo fixture checks;
- no bytecode/cache/local-state hygiene;
- review-target receipt generation;
- deterministic archive receipt generation;
- dependency/security review;
- performance review;
- LOC exceptions.

## Current Phase

The Rust migration requires a fresh four-persona material sign-off review
against exact audit, review-target, archive, active-registry, and claim-ceiling
anchors. Do not treat inline Markdown anchor text as authority. If any reviewer
returns `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED`, close the round, repair the
blockers, regenerate validator/review-target/archive/registry anchors, and
start a fresh full-scope round with new reviewers.

## Completion Gate

Do not claim completion until the canonical Rust validator receipt is fresh,
red fixture count is equal or higher than the prework count unless justified,
generated artifacts and package digest are refreshed, review-target and archive
receipts are regenerated where relevant, and four fresh `gpt-5.5 high`
reviewers sign off in the same material sign-off round against the same
anchors.
