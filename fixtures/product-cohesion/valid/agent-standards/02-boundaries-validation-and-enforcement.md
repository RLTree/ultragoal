# Boundaries, Validation, And Enforcement

## Coverage And Test Law

Coverage is an accountability mechanism, not a badge.

- Every behavior change needs executable evidence.
- If the repo adopts 100 percent line, branch, function, region, UI-state, or
  artifact coverage, that means exactly 100 percent. Uncovered records are work
  owed now or mechanically documented tooling limits.
- If the repo is below 100 percent, name the current floor, the ratchet rule,
  owner, reason, blocker or tech-debt record, and the report that decides
  whether the floor was met.
- Lower-than-100 coverage cannot support complete, ready, done,
  production-ready, or release claims. Use a withheld claim ceiling until the
  ratchet debt is closed.
- Coverage receipts name the command, tool, target paths, measured dimensions,
  percentage, floor, uncovered records, timestamp, exclusions, and claim ceiling.
- Generated, vendor, and external exclusions require a mechanical exclude list
  with rationale. Exclusions are not counted as covered.
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
- If the user or contract names a full plugin, process, installed package, app
  registry, or runtime authority, the proof receipt names the authority source.
  Silent fallback to source files, local clones, installed caches, reference
  packages, or alternate tools claim-limits the result unless the fallback
  boundary is explicit and accepted.
- Plugin availability claims need separate source, installed-plugin,
  cache-package, package-sync, package-hygiene, and per-surface receipts. A
  source-tree check does not prove installed plugin, app registry, sidebar,
  multi-agent launcher, marketplace, or runtime visibility.
- Browser, UI, and runtime proof records exact tool identity, version, binary
  path when relevant, artifact digests, workspace, and claim ceiling. A
  screenshot or trace without runtime identity is weak context, not strong
  product proof.
- CLI checks and fixtures do not prove live recording, live UI, replay,
  transcript alignment, video alignment, or parent-operation coordination.
  Those claims need same-surface coordination or parent-operation receipts.
- Proof-bearing generated artifacts with derived authority values must be
  recomputed from canonical current inputs or bound to a digest derived from
  those inputs. Matching path, id, title, privacy, or content digest is not
  enough when generated fields affect ranking, install, resolver, archive,
  package, source-card, moving-value, or runtime-authority behavior.

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
- Reviewer agreement is not closure when the finding exposes a recurring proof
  boundary. Convert it into a script, test, schema, standards row, fixture,
  blocker, or claim ceiling before treating the lesson as durable.
- Repeated authority lists, ids, digests, counts, and schema enums need a
  generator or cross-check so one stale copy cannot silently pass.
- Repeated generated-artifact integrity misses need domain validators that
  recompute behavior-affecting derived values from canonical inputs whenever
  that is practical.

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
