# Boundaries, Validation, And Enforcement

## Coverage And Test Law

Coverage is an accountability mechanism, not a badge.

- Every behavior change needs executable evidence.
- If the repo adopts 100 percent line, branch, function, region, UI-state, or
  artifact coverage, that means exactly 100 percent. Uncovered records are work
  owed now or mechanically documented tooling limits.
- If the repo is below 100 percent, name the current floor, the ratchet rule,
  and the report that decides whether the floor was met.
- Do not park "proven unreachable" code as a resting state. Delete it,
  restructure it out of existence, or add fault-injection coverage.
- Unit tests, fixtures, mocks, generated examples, and smoke tests prove only
  the surface they execute. They do not prove live runtime, UI, external
  integration, or product usefulness.

## Proof Freshness And Operation Identity

Proof must bind to a fresh operation, not just to plausible metadata.

- Runtime or workflow proof names the operation id, command id, started and
  completed time, input artifact digest, output artifact digest, workspace, and
  claim ids it proves.
- Stop, drain, timestamp, status, or queue metadata alone does not prove the
  operation happened correctly.
- Regenerate manifests after live proof. Stale claim rows are blockers, not
  harmless leftovers.
- Split large gates into independently runnable receipts when a monolithic
  check is slow, hangs, or hides which proof surface failed.
- If a command needs isolated cache or target state, declare that environment
  in the receipt or setup contract; do not rely on the operator's memory.

## Parse At Boundaries

External inputs are untrusted until parsed into trusted types or structures.
External inputs include user input, CLI args, environment variables, file
imports, browser or IPC payloads, database or state reads, model output, MCP or
tool output, memory retrieval, logs, search results, and generated tests.

- Parse at the boundary and preserve the parsed knowledge. Do not scatter
  ad-hoc validation through business logic.
- Make illegal states unrepresentable where practical.
- Treat `Result<(), E>` or boolean checkers with suspicion when they could
  return a proven type instead.
- Do not build on guessed data shapes. Use typed SDKs, schemas, parsers, or
  explicit adapters.
- Duplicated facts whose sync would be an illegal state need a single source of
  truth or a mechanical cross-check.

## Mechanical Enforcement

Repeated review comments become checks, lints, schemas, validators, fixtures,
or documented blockers. Prose is the fallback, not the mechanism.

- Enforce invariants, not implementation style. Require the what; leave the how
  flexible unless the how is part of the contract.
- Check failures should prescribe the repair or point to the owning doc.
- Repeated failures never stay as reminders in chat. Promote them into
  executable gates, negative fixtures, evals, or tracked blockers.
- Repeated authority lists, ids, digests, counts, and schema enums need a
  generator or cross-check so one stale copy cannot silently pass.

## State Transition Integrity

State machines need explicit forward-only transitions for approval, closure,
dependency release, and queue membership.

- Approval or closure cannot silently re-enter `queued`, `active`, or
  needs-review states unless a typed reopen transition records actor, reason,
  source state, destination state, and fresh proof obligations.
- Guards check provenance and transition history, not status strings alone.
- Replay paths are fixtures when stale state previously re-entered a queue.

## Feedback Loops

Every meaningful change needs a loop an agent can run alone:

1. Act.
2. Capture state through tests, receipts, logs, screenshots, traces, audits, or
   generated artifacts.
3. Compare against explicit criteria.
4. Repeat or stop with a named blocker.

Subjective review is the last gate, not the only gate. Runtime, UI, API,
workflow, and product claims require proof on the same surface when feasible.
