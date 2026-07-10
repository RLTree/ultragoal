# Research and Law Authority

## Source classes

| Class | Meaning | May directly bind product law? |
|---|---|---|
| `primary_authoritative` | Maintainer specification, official product documentation, standard, or first-party engineering report | Yes, after product translation and compatibility review |
| `primary_empirical` | First-party experiment, evaluation, incident, or product measurement | Yes for observed boundary; generalization requires product evidence |
| `secondary_advisory` | High-quality synthesis, book, or practitioner guidance | Advisory until independently supported |
| `repository_observation` | Current source, behavior, artifact, or runtime observation | Yes for current system assessment; not external product truth |
| `experimental_hypothesis` | Plausible design to test | No; must be labeled and evaluated |
| `rejected` | Considered and declined with reason | No; retained to prevent unexamined recurrence |

A package-authored source card or snapshot inherits neither primary status nor binding authority merely because it summarizes a primary source. The registry MUST preserve the canonical URL, access date, source class, relevant version, extracted fact, and local snapshot relationship.

## Research-to-law process

Every adopted requirement follows this chain:

```text
source
  -> source fact
  -> product principle
  -> stable law id
  -> product owner
  -> implementation surface
  -> validator
  -> behavioral fixtures
  -> independent product proof
  -> claim guard
  -> completion blocker
```

No missing link may be represented as `pass`. Before implementation it is `planned`; after implementation without independent proof it is `observed` at most.

## Current primary-source findings

### OpenAI Harness Engineering

The first-party Harness Engineering article treats repository structure, agent legibility, application behavior, logs, metrics, traces, architecture constraints, and entropy control as parts of the harness. It supports HUL-PRODUCT-001, HUL-OBSERVE-001, and HUL-MAINTENANCE-001. It does not prescribe this repository’s exact commands or prove they work.

### OpenAI Codex goals

Current Codex guidance describes a durable goal with a clear outcome, constraints, stop condition, validation loop, checkpoints, and pause/resume/clear controls. Harness Ultragoal should integrate with that platform capability while retaining repository product-state lineage. It should not create a repository receipt that pretends to be platform goal state. This supports HUL-GOAL-001.

### OpenAI GPT-5.6, Ultra, and subagents

Current first-party material describes GPT-5.6 Sol as the flagship coding model and Ultra as a multi-agent coordination mode. Current subagent guidance emphasizes explicit delegation conditions, parallel read-heavy work, conflict risk for write-heavy work, and integration by the main agent. These support adaptive orchestration, disjoint ownership, and root integration. They do not justify a fixed number of workers as product law, because availability and task structure vary.

### OpenAI skills and plugins

Current first-party guidance describes skills as progressively disclosed instruction packages and plugins as bundles that may contain skills and MCP/app capabilities. It documents marketplace/install locations, application restart/discovery behavior, and new-task testing. This supports HUL-PLUGIN-001 and HUL-DISCOVERY-001. Exact host behavior remains a versioned product surface to test.

### OpenAI Symphony

The first-party Symphony release describes task/workspace orchestration and blocked dependency graphs, while also warning that rigid state machines become limiting and favoring objectives plus tools. This supports state-derived work packages and typed dependencies, not preservation of the repository’s static lanes.

### OpenAI agent improvement and evaluation audit

First-party tax-agent work connects production evidence, corrections, findings, evaluations, and scoped changes. A current coding-evaluation audit reports that a material fraction of benchmark tasks can be broken through overly strict tests, underspecified prompts, low coverage, or misleading setup. Together they support HUL-EVAL-001 and HUL-IMPROVEMENT-001: evaluation data must be validated and an improving agent cannot self-certify.

### Google Site Reliability Engineering monitoring

Google’s SRE text distinguishes dashboarding, ad hoc analysis, trends, and simple robust paging, and centers latency, traffic, errors, and saturation as golden signals. Harness Ultragoal adopts the principle of behavior-linked signals and actionable alerts, but maps them to local agent/product operations rather than copying a service dashboard wholesale.

### OpenTelemetry semantic conventions

OpenTelemetry defines stable semantic names and types and recommends prototyping conventions before stability. Its baggage documentation warns that propagated baggage has no built-in integrity and can reach downstream services. These support stable product event schemas, bounded attributes, and a strict privacy/trust boundary.

### Rust and Cargo tooling

Cargo build-cache and JSON-message documentation, nextest partition/archive behavior, rustc coverage instrumentation, and `cargo llvm-cov` provide suitable compilation, test, and coverage substrate. They do not own Harness affected-set legality, cache identity, or claim authority. Current `clap` documentation supports a conventional typed parser. The Rust CLI book supports predictable output and exit behavior.

### SLSA and Sigstore

SLSA provenance and build requirements define useful integrity, build-isolation, and cache-poisoning boundaries. Sigstore documents signed-bundle verification for release artifacts. These support HUL-SUPPLY-001. Signatures are an optional release-integrity control and cannot prove plugin discovery or product behavior.

## Binding requirement table

The complete machine join is `REQUIREMENT_TRACE.json`. This table names the primary source families and adoption posture.

| Law | Source families | Product principle | Adoption |
|---|---|---|---|
| HUL-PRODUCT-001 | OpenAI Harness Engineering; repository observation | Product behavior and legibility are one system | binding candidate |
| HUL-PLUGIN-001 | OpenAI skills/plugins | Small front door and progressive closure | binding candidate |
| HUL-DISCOVERY-001 | OpenAI plugins; SLSA; repository observation | Truth is surface-specific | binding candidate |
| HUL-FIT-001 | OpenAI harness; repository observation | Inspect, preserve, reconcile | binding candidate |
| HUL-STATE-001 | SLSA provenance; repository observation | Claims bind immutable inputs | binding candidate |
| HUL-INCREMENTAL-001 | Cargo/nextest; repository observation | Fast reuse requires dependency truth | binding candidate |
| HUL-AUTHORITY-001 | repository observation | One semantic owner per decision | binding candidate |
| HUL-PROOF-001 | SLSA; evaluation audit; repository observation | Evidence needs independent congruent reconciliation | binding candidate |
| HUL-COVERAGE-001 | rustc; llvm-cov; nextest | Coverage dimensions and scopes stay explicit | binding candidate |
| HUL-OBSERVE-001 | Google SRE; OpenTelemetry; Harness Engineering | Semantic, actionable, queryable operations | binding candidate |
| HUL-REPAIR-001 | Rust CLI guidance; Harness Engineering | Findings must close through exact repair | binding candidate |
| HUL-GOAL-001 | OpenAI goals | Integrate platform durability without simulation | binding candidate |
| HUL-ORCHESTRATION-001 | OpenAI Ultra/subagents/Symphony | Adaptive graph, root integration | binding candidate |
| HUL-WORKER-001 | OpenAI subagents; repository observation | Bounded ownership and root acceptance | binding candidate |
| HUL-REVIEW-001 | evaluation audit; SLSA | Independent falsification before claim | binding candidate |
| HUL-SECURITY-001 | SLSA; OTel baggage; system safety guidance | Confinement and least authority | binding candidate |
| HUL-SUPPLY-001 | SLSA; Sigstore; OpenAI plugin install | Reconcile source through installed identity | binding candidate |
| HUL-CLI-001 | clap; Rust CLI guidance; repository observation | Typed predictable commands and effects | binding candidate |
| HUL-EVAL-001 | OpenAI evaluation audit | Validate evaluation data before authority | binding candidate |
| HUL-IMPROVEMENT-001 | OpenAI tax agents; Harness Engineering | Failure-to-eval loop with independent promotion | binding candidate |
| HUL-PERFORMANCE-001 | Cargo/nextest; Google SRE | Measure real scoped work and verified reuse | binding candidate |
| HUL-PRIVACY-001 | OTel conventions/baggage | Minimize, classify, redact, retain deliberately | binding candidate |
| HUL-MAINTENANCE-001 | Harness Engineering; repository observation | Measured entropy removal | binding candidate |
| HUL-COMPLETION-001 | all above; repository observation | Minimum reconciled ceiling governs completion | binding candidate |

## Advisory practices

These are recommended but not binding until measured in this product:

- warm `status`/`next` p95 targets in the CLI contract;
- `clap` as the specific parser rather than an equivalent maintained parser;
- nextest as the scheduler for every fixture class;
- Sigstore signing for every local development package;
- a particular metrics/logs/traces backend;
- four concurrent workers merely because one runtime defaults to four;
- any universal coverage percentage or mutation-score threshold.

## Experimental hypotheses

The implementation program should test:

- whether a content-addressed query graph can keep routine planning below the target across repository-size tiers;
- whether semantic compiler/Cargo data materially reduces impacted-test false negatives compared with conservative path mapping;
- whether local event-spool repair explanation improves operator time-to-recovery;
- whether generated skill/agent presentations reduce drift without harming usability;
- whether package-to-application discovery can be verified through a stable host API instead of UI automation;
- whether traces can safely improve test selection without becoming an opaque authority.

Hypotheses have no claim authority until promoted through evidence and independent review.

## Rejected ideas

| Idea | Reason rejected |
|---|---|
| Preserve the fixed historical lane list | Encodes migration history, cannot adapt to current dependencies, and splits product semantics |
| Treat every mandatory-law row as implemented because its schema validates | Declaration is not behavior |
| Make the external observability stack a routine prerequisite | Adds fragility and wrong-surface proof; local deterministic operation must survive absence |
| Treat a package signature as plugin correctness | Integrity does not prove installation, discovery, or behavior |
| Let workers issue completion claims | Violates independent integration and minimum-ceiling authority |
| Use model name or agent count as a correctness condition | Runtime capabilities and availability change; behavior must be tested |
| Keep multiple top-level readiness commands for convenience | Creates duplicate authority; filtered views should share one graph |
| Automatically promote harvested failures into laws | Traces and evaluations can be wrong, contaminated, or non-generalizable |
| Require a clean tree for routine checks | Conflicts with the actual development lifecycle and hides impact errors |
| Count receipts, fixtures, or generated rows as proof | They are observations or inputs until independently reconciled |

## Research maintenance

Every source record MUST include `checked_at`, canonical URL, class, version/date when relevant, law links, and local capture provenance. Drift-prone product documentation MUST be refreshed before a release claim. Removed or changed guidance triggers a law-impact review; it does not silently rewrite current product law.

Research updates are proposed changes. They require review, migration analysis, validator/fixture updates, product proof, and explicit adoption before becoming binding.

## Existing corpus disposition

`RESEARCH_SOURCE_REGISTRY.json` explicitly dispositions all 15 entries from the active `docs/research-source-registry.json`. Current primary sources are refreshed or consolidated; provider-specific and practitioner material remains advisory; attached Rust/TypeScript guides become detected-stack fitting inputs; and the two gold-standard synthesis archives become routing/history rather than external fact authority. No existing source is silently dropped, and no local synthesis is allowed to prove current vendor, tool, installation, or runtime behavior.
