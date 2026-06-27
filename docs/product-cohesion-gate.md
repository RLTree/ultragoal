# Product Cohesion Gate

The harness must prevent a strong engine from being undersold by a weak product
surface. The failure mode is not "the UI is ugly." It is a product where
powerful tools, receipts, contracts, launchers, and proof surfaces exist but do
not compose into a journey a user can naturally understand and trust.

## Trigger

Use this gate when a goal affects any product surface listed below.

| Trigger text or structure | Required classification | Owed proof |
| --- | --- | --- |
| User-facing product, dashboard, local app, workflow launcher, issue board, run console, settings/control surface, or consumer-visible feature | Product-applicable | Product Cohesion receipt, UI journey evidence, runtime evidence, claim ceiling |
| `requires_product_cohesion: true`, `claim_kind: product_cohesion`, or `claim_surface: product_cohesion` | Product Cohesion | Product Cohesion receipt and attached proof artifacts |
| Runtime-only engine, CLI, worker, schema, static validation, or backend claim with no user journey text | Explicit non-product | Runtime/static proof plus non-product rationale |
| Ambiguous product language or mixed engine/product wording | Reviewer classification required | Product proof or withheld/backlog claim |

Every included `feature_completion` claim must declare
`product_applicability`. Runtime-only evidence may close an engine-only claim
only when the classification is explicitly non-product, such as
`engine_runtime_only`, `internal_non_product`, or
`proof_surface_negative_probe`, with a rationale. If any product-applicability
flag is true, or if the claim text asserts a user-facing, local-app,
dashboard, workflow-launcher, run-console, or control-surface result, the claim
owes Product Cohesion proof. A runtime CLI receipt alone is not enough.
Joined lowercase forms of multi-word product terms are product text. Examples
include `webapp`, `runconsole`, `controlsurface`, `controlpanel`,
`customerfacing`, `clientfacing`, `desktopapp`, `localapp`, `taskboard`,
`chatinterface`, `userjourney`, `mobileapplication`, and
`workflowlauncher`.

The package validator also runs a deterministic natural-language pass over
claim titles and descriptions. It is intentionally cheap and auditable, not an
LLM call: claim text is normalized for punctuation, camelCase, compact terms,
and simple plurals. A claim is treated as product-applicable when the text
contains known product terms or combines a user/operator actor, a product
action such as start/monitor/review/recover, and a surface word such as
workspace/place/experience/tool. Generic "surface" wording is not enough by
itself; non-product proof or runtime surfaces can pass only when explicitly
classified as non-product and the claim text does not describe a user journey.

Do not make this a default ceremony for backend-only work. Product cohesion is
claim-dependent, like observability.

## Contract

A product-cohesion claim needs both product reasoning and UI proof.

- Product reasoning names the primary user, job, promise, business or user
  outcome, surface map, human-attention policy, and withheld claims.
- UI proof shows the critical journey through real screens or a real local
  control surface.
- Runtime proof shows the engine, API, or command behavior behind the journey.
- Claim-ceiling proof says what the product can and cannot claim after the
  observed journey.

Engine proof alone cannot close this gate. Browser screenshots alone cannot
close this gate. The receipt must connect the user's intent, the product
surface, the engine state, and the proof owed.

## Portable Target Shape

When installed into a target repo, the optional product-cohesion setup should
produce:

```text
docs/product-cohesion.md
validation_artifacts/product-cohesion/journey-receipt.json
```

The repo's gate should emit `harness-check:product-cohesion pass` only after
the receipt exists, has a non-zero evidence digest, and names the journey under
review.

## What Downbeat Taught This Gate

The `codex-workflow-rs` Downbeat surface demonstrates the right tension. The
engine has durable runs, contracts, receipts, claim ceilings, reviews, and
mock/live execution modes. The product has an Issues flow, Conduct surface,
Runs detail, Library, and local-only shell. The risk is that each screen can be
locally correct while the overall experience still feels like separate tools.

The product-cohesion gate asks the missing question before claiming success:
can the user travel from intent to route decision to execution truth to proof
ceiling without needing repo lore?

It also asks whether the product is preserving the harness-engineering promise
that the agent should rarely need the human. A "Needs you" surface is valid only
when it represents a real permission, spending, ambiguity, safety, credential,
or irreversible-action boundary. If most work lands there, the product is
leaking harness failure into the user's workflow.

## Product Lenses

Use existing product, design, and strategy skills only to produce the concrete
journey receipt, UI evidence, accessibility/runtime proof, and claim ceiling the
gate requires. Do not expand this gate into a broad design manifesto.

## Human Attention Policy

Every product-cohesion receipt must classify human attention as one of:

- `not_needed`: the agent should continue without user intervention;
- `permission_required`: the user must approve spend, external mutation,
  destructive action, credential access, or similar boundary;
- `ambiguity_blocker`: the agent lacks enough product or technical context and
  cannot safely infer the answer;
- `external_blocker`: the agent needs unavailable auth, service state, hardware,
  or human-only environment access;
- `safety_blocker`: continuing would cross a safety, privacy, legal, or security
  boundary.

The receipt must say how often interruption is expected. "Most issues need the
human" is a product-cohesion failure unless the product itself is a human review
queue.

The receipt must also point to evidence that harness-owned routes, retries,
receipt lookups, blocker classification, or other autonomous recovery paths were
exhausted before human attention was requested.
