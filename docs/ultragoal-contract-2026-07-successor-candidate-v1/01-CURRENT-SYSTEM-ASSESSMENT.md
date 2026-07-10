# Current System Assessment

## Executive judgment

The repository contains unusually broad thinking, meaningful source research, a substantial Rust validator, strong instincts about fail-closed proof, and many useful adversarial artifacts. It does not yet behave as one coherent product.

The current system is dominated by accumulated gates, static inventories, generated receipts, and overlapping validator commands. Plugin acquisition, discovery, fitting, routine use, diagnosis, and real-user journeys receive less executable authority than the validator itself. Many laws are verified by checking a declared field or receipt rather than independently reproducing the claimed behavior. The result is a high-volume evidence system whose claim authority is often weaker than its surface area suggests.

The successor should retain the hard-won product knowledge while replacing the architecture that makes truth difficult to locate.

## Repository inventory

Read-only inventory on 2026-07-09 found:

| Class | Current observation | Interpretation |
|---|---:|---|
| Tracked files | 5,011 | Large product and evidence surface |
| Rust files | 1,178 / about 141,102 lines | A substantial authority kernel, not a thin plugin helper |
| Fixture tree | about 3,499 tracked files | Broad adversarial intent, with significant declarative duplication |
| Validator tree | about 1,180 tracked files / 5.05 MB source | Many overlapping commands and checks |
| Active contract | about 10,050 lines across routed modules | Detailed but history-heavy and gate-oriented |
| Schemas | 84 | Useful type intent, uneven identity and ownership |
| Templates | 68 | Rich setup/proof material, several oversized generated catalogs |
| Skills | 14 | Broad lifecycle coverage, overlapping responsibilities and historical pins |
| Agent definitions | 7 Markdown plus 7 custom-agent TOML definitions | Useful roles, duplicated persona surfaces |
| Plugin manifest resources | 1,382 | Exhaustive packaging intent, high drift cost |
| Validation artifacts | about 9.6 GB on disk | Observations and mutable history, not source authority |
| Rust build cache | about 32 GB on disk | Performance substrate and maintenance concern, not product evidence |

Counts are an authoring-time snapshot, not a durable requirement. Generated content, fixtures, receipts, caches, and runtime observations are deliberately not treated as equivalent to tracked product source.

## Source classes

### Tracked product source

- plugin manifests, skills, agents, templates, install guidance, scripts, schemas, Rust source, and observability configuration;
- active builder-contract modules and source-law registries;
- package-owned source snapshots and research cards.

### Generated or derived content

- large manifest inventories, red-fixture catalogs, generated rows, package lists, coverage manifests, and report aggregates;
- any data reproducible from canonical source by a pinned generator.

Generated content may be checked for reproducibility, but must not become a second hand-edited authority.

### Fixtures

- red, green, and tamper cases;
- fixture catalogs, generators, schedulers, and expected diagnostics.

Fixtures are test inputs. Their existence and passing reports do not prove real product behavior.

### Mutable receipts and runtime evidence

- `validation_artifacts/**`, workflow reports, local telemetry, current-state files, timing records, and package observations.

These are observations bound to inputs, tools, environment, and time. They are never contract source.

### Caches

- Cargo output, node-local query results, downloaded plugin state, and any verified-reuse store.

Caches accelerate work. They do not establish authority unless the cache key, producer, dependencies, candidate identity, integrity, and replay equivalence are independently verified.

### Migration history

- `LANE_REGISTRY.json`, `.codex/lane-registry/**`, historic gate identifiers, thread IDs, old model pins, and progress narratives.

These explain how the current system arose. They are not an architecture to preserve.

## Product assessment

### What is worth retaining

- The insistence that claims fail closed and name an exact repair.
- Mandatory repository fitting rather than assuming every project begins empty.
- Separate attention to source, package, installation, cache, application, and runtime surfaces.
- Typed fixture classes, isolated fixture execution, and adversarial personas.
- A Rust kernel for fast, deterministic local decisions.
- Product-journey, observability, standards, security, research, and improvement intent.
- The distinction between routine development and strict completion proof.
- Current-state and next-action concepts.

### What must be consolidated

- Source audit, mandatory-law, foundational-law, source-obligation, and trace checks must join one canonical law graph.
- Product, standards, review, final packet, audit, and completion commands must use one claim engine instead of separate top-level authorities.
- Package inventory, digest, source list, cohesion inventory, and manifest views must derive from one immutable package snapshot.
- Current-state, `next`, observe explanation, and repair output must compile from the same findings graph.
- Agent Markdown and TOML roles must have one canonical role schema with generated presentations.
- Fixture catalogs must be generated from canonical fixture metadata rather than maintained as parallel giant JSON authorities.

### What must be rewritten

- Skills and agent guidance that encode fixed macro-lanes, historical model names, or duplicate command rituals.
- The hand-written CLI argument parser and loosely typed string/path wrappers.
- Setup and retrofit verification that accepts receipts without reconciling installed behavior.
- Coverage tooling whose routine path invokes strict/global work or hides cache/source mutation.
- Checklists that mix binding contract, mutable history, generated status, and claims.
- Research cards that summarize primary sources but are sometimes treated as the source itself.

### What must be replaced

- Static lane orchestration with adaptive product-semantic work packages.
- Receipt-presence proof with independent behavior reconciliation.
- Boolean-only “law-specific” fixtures with behavioral red/green/tamper cases.
- Embedded Python and shell policy logic with Rust-owned Harness semantics or clearly non-authoritative substrate adapters.
- Read-shaped commands with hidden writes with explicit command-effect types and explicit `--emit` or `apply` operations.

### What must be retired

- `LANE_REGISTRY.json` and `.codex/lane-registry/**` as active runtime architecture.
- Fixed historical lane IDs as implementation units.
- Duplicate claim-bearing commands and secondary hand-maintained inventories.
- Dead fallback branches, coverage-only code, generic helpers without a named product owner, and mandatory external telemetry for local correctness.
- Generated receipt history inside contract modules.
- Product laws whose only executable check is that a matching declaration exists.

## Plugin assessment

The plugin contains the right broad ingredients: discovery metadata, fitting skills, runtime guidance, research and improvement roles, templates, and custom agents. Its weakness is closure. A user cannot yet follow one authoritative, observable journey from marketplace source through installed plugin, discovered entry point, repository fit, routine work, diagnosis, proof, and completion.

The plugin’s 1,382-resource manifest creates packaging coverage but also a large drift surface. Resource enumeration is not proof that the application downloaded the same bytes, registered the plugin, exposed its entry skills, or executed them in a real task. The successor must make those proof levels explicit and independently test each one.

Several skills overlap, hard-code historical orchestration concepts, or pin superseded model behavior. Progressive disclosure exists in form but is weakened by duplicated concepts and large initial guidance surfaces. The successor should expose a small front door and route internally to product-owned capabilities.

## CLI assessment

The Rust CLI has become a collection of product subsystems: audit, product fitness, standards, coverage, package inventory, current state, next actions, observability, fixtures, improvement, and many specialized legacy commands. The breadth is valuable; the command topology is not.

Problems found:

- multiple top-level paths can make overlapping readiness or completion claims;
- custom parsing increases ambiguity and maintenance load;
- path and text wrappers reject emptiness but do not encode domain legality;
- several read-like commands generate artifacts as a side effect;
- command output, evidence generation, and claim authority are interleaved;
- specialized gate-era commands expose internal architecture to users;
- exact repair guidance is inconsistent across surfaces.

During this authoring audit, invoking the read-shaped `ultragoal --root . package digest` wrote `validation_artifacts/observability/package-digest.json`. That artifact is excluded from assessment evidence and left in place to respect the no-mutation/no-cleanup boundary. This is direct evidence for HUL-CLI-001: query behavior must be side-effect-free by construction and mutation must be explicit.

## Custom-tool assessment

Most requested successor capabilities exist in partial or fragmented form. The deficiency is not absence of code; it is unclear ownership, duplicate derivations, weak input identity, and insufficient independent reconciliation.

- Audit-context and current-state structures exist but are not one immutable snapshot used by every command.
- Package truth is broadly inventoried but source/package/install/cache/app/runtime identity is not one reconciled chain.
- Coverage and impacted-test machinery exists, but routine and strict authority are still entangled.
- Fixture scheduling is typed and isolated, but many fixtures validate declarations rather than behavior.
- Observability tooling is extensive, yet local proof is often stale or dependent on a heavy optional stack.
- `next` and observe repair concepts exist, but they do not yet compile exclusively from the canonical findings graph.
- Agent-quality and improvement loops are rich in adapters and receipts but need validated evaluation data and promotion gates.

The definitive classification appears in `CUSTOM_TOOL_INVENTORY.json`.

## Proof assessment

An existing audit receipt dated 2026-07-08 reported failures across fitting, journey digests, performance, observability, development experience, standards, product fitness, and other surfaces. A routine coverage receipt reported roughly 14.28% with 761 uncovered items and a routine-repair-only claim ceiling. These are useful observations, but both are stale relative to current inputs and cannot support a present product claim.

The most common false-pass pattern is:

1. a contract declares a required field;
2. a fixture sets the field to `true`;
3. the validator checks the field;
4. a generated report says the law passed;
5. the report is treated as evidence that the product behavior exists.

The successor rejects that chain. At least one independent product exerciser must observe the actual behavior or a deliberately isolated equivalent, and the claim guard must verify input identity and surface congruence.

## Observability assessment

The development stack includes OpenTelemetry Collector, VictoriaMetrics, VictoriaLogs, VictoriaTraces, Vector, and Grafana. It is a useful optional integration environment. It is too heavy to be a prerequisite for routine correctness.

Current topology gaps include split ingestion paths, incomplete logs visibility in the dashboard stack, and a risk that local paths or high-cardinality values enter telemetry. The successor needs two planes:

- a zero-dependency local structured event spool and query path used for deterministic repair;
- optional OpenTelemetry export and live-system adapters used for interoperability and roundtrip tests.

Both planes must be reconciled. A local JSON row does not prove export; an external dashboard does not prove the candidate that produced it.

## Fixture and evaluation assessment

The repository’s fixture volume shows serious adversarial intent. The scheduler uses typed task classes and temporary isolation, which should be retained. The current leak check is too narrow, and many law fixtures are declarative toggles. Test quantity therefore overstates behavioral coverage.

The successor requires:

- fixture metadata with law, surface, behavior, mutation, expected diagnostic, and isolation class;
- generated catalogs from that metadata;
- behavioral fixtures for real command, package, install, plugin, and journey surfaces;
- negative-control and false-positive tests;
- evaluation-data review before an agent-quality score can block or promote product behavior;
- harvested failures to become proposed tests only after independent triage.

## Security and supply-chain assessment

Positive foundations include local execution, pinned development images, package digest intent, and strong suspicion of stale evidence. Missing or incomplete authority includes:

- a uniform command-effect and permission model;
- path confinement and symlink/race handling across every writer;
- secret and private-path redaction before both local and remote telemetry;
- package provenance connected to exact source and build inputs;
- install/cache/application byte identity and integrity verification;
- dependency and external-tool trust policy;
- robust worker isolation for shared-authority and external-state actions.

The optional development observability stack may use development credentials, but those credentials must never be promoted to a production default or product proof surface.

## Performance assessment

Cargo, nextest, incremental compilation, llvm-cov, an impacted-test mapper, and local caches are appropriate substrates. Performance proof is weakened when a “fast” path reports bookkeeping time, reuses an unverifiable cache, silently performs global work, or writes unrelated receipts.

Routine work must target changed surfaces and remain useful with a dirty tree and partial optional evidence. Strict proof may be slower, but must measure executed work or verified same-candidate reuse. Routine latency is a product behavior; a static threshold row is not proof of it.

## Usability assessment

The current product asks operators to understand internal gate names, receipt locations, command families, and proof distinctions that the product itself should resolve. A successful successor exposes:

- one plugin front door;
- a small, predictable CLI;
- a truthful current-state view;
- exactly one recommended next action for each blocking finding;
- stable diagnostic codes and copyable reruns;
- explicit command effects before mutation;
- separate routine and strict modes;
- graceful operation when optional services are absent.

## Bottom line

This repository is not a failed prototype. It is a knowledge-rich system that accumulated too many representations of authority. The successor should preserve its research, failure lessons, typed Rust core, and adversarial ambition while sharply reducing user-visible concepts and forcing every claim through one current-input, independently reconciled authority graph.
