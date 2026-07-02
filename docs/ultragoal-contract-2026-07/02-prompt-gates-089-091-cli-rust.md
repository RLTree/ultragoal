## Gate 89: CLI Control Plane Authority And Non-Bypassable Harness Law Execution

This gate is additive to Gates 1-88. It does not replace, narrow, weaken, summarize, defer, or supersede any existing Harness Ultragoal law, validation requirement, update_goal() stop condition, packet requirement, Product Fitness requirement, source/install/cache/app-registry separation requirement, coverage requirement, typed-boundary requirement, line-cap requirement, standards requirement, foundational traceability requirement, or claim-ceiling requirement.

The Harness Ultragoal plugin must become a CLI-governed enforcement product. The CLI is not merely a convenience wrapper, audit runner, receipt generator, or optional validator. The CLI is the mandatory control plane and authority kernel for Harness Ultragoal law execution. Agents, reviewers, markdown files, TOML prompts, JSON fixtures, generated packets, checklists, receipts, reports, session summaries, Chronicle summaries, issue comments, and human-written status text may provide inputs to the CLI or projections from the CLI, but they may not grant authority, satisfy laws, mint evidence, raise claim ceilings, mark review readiness, or make update_goal() eligibility decisions without CLI verification.

The core rule is:

> A Harness Ultragoal claim is unsupported unless it is computed, minted, or verified by the installed Harness Ultragoal CLI from typed inputs, current source state, current package state, current proof surfaces, current law graph, current schema catalog, current standards rows, current source obligations, current fixtures, and current receipts.

Any path that allows an agent to satisfy a law by prose, row shape, checklist text, stale receipt reuse, copied evidence, generic reviewer approval, packet existence, install success, fixture count, smoke test, source-only proof, cache-only proof, package presence, or claim-ceiling apology is non-compliant.

### 89.1 Authority Model

The CLI must implement a typed authority model. Authority may not be inferred from freeform strings, file names, fixture names, markdown headings, receipt paths, reviewer names, or agent-written summaries.

The CLI must parse all external inputs into closed authority types before using them. Inputs that cannot be parsed into closed authority types must fail before validation logic runs.

The required authority types include the full list below:

- `GateId`
- `LawId`
- `SourceObligationId`
- `FoundationalRequirementId`
- `StandardsRowId`
- `CheckId`
- `FixtureId`
- `FixtureKind`
- `ReceiptKind`
- `ReceiptSchemaVersion`
- `CandidateVersion`
- `CandidateDigest`
- `PackageDigest`
- `SourceDigest`
- `SchemaCatalogDigest`
- `LawGraphDigest`
- `StandardsDigest`
- `SourceObligationDigest`
- `FixtureCatalogDigest`
- `ProofSurface`
- `PackageSurface`
- `RuntimeSurface`
- `ClaimClass`
- `ClaimCeiling`
- `CapabilityId`
- `CapabilityAuthority`
- `CapabilitySurface`
- `ReviewRoundState`
- `ReviewerRole`
- `ProductDisposition`
- `ProductFitnessDisposition`
- `ProductCohesionDisposition`
- `ProductSuccessDisposition`
- `FailureDisposition`
- `FailurePromotionState`
- `ExecutionMode`
- `InstallMode`
- `TargetRepoMode`
- `RegistryMode`
- `PacketMode`
- `IssueLifecycleState`
- `UpdateGoalEligibilityState`

The CLI must reject:

- unknown enum values;
- unknown JSON keys;
- missing required fields;
- nullable authority fields;
- duplicate IDs;
- normalized ID collisions;
- case-only ID collisions;
- Unicode-normalization collisions;
- path traversal;
- absolute private local proof paths in package-owned inventory;
- stale schema versions;
- stale law graph digests;
- stale standards digests;
- stale source-obligation digests;
- stale fixture catalog digests;
- stale receipt schema versions;
- stale candidate versions;
- stale package digests;
- stale source digests;
- wrong proof surface;
- wrong package surface;
- wrong runtime surface;
- missing capability authority;
- unverified generated artifacts;
- hand-authored authority artifacts;
- receipts whose issuer cannot be verified;
- receipts whose input graph does not match the current repo;
- artifacts whose claims cannot be derived from current CLI-verified receipts.

No validator check may accept unparsed `serde_json::Value`, raw strings, raw paths, or untyped maps as authority after boundary parsing. Boundary modules may parse raw input, but they must return typed domain objects or typed failures. Downstream law execution must operate on typed authority objects only.

### 89.2 CLI As Sole Completion Authority

The following outcomes must be impossible to emit, satisfy, or mark complete without CLI authority:

- source audit pass;
- installed plugin audit pass;
- cache package audit pass;
- active registry proof;
- app-surface proof;
- plugin UI proof;
- marketplace proof;
- launcher runtime proof;
- reviewer exposure proof;
- review-target receipt;
- candidate archive receipt;
- final packet;
- claim ceiling;
- Product Fitness proof;
- Product Cohesion proof;
- Product Success proof;
- coverage proof;
- line-cap proof;
- typed-boundary proof;
- standards enforcement proof;
- foundational traceability proof;
- source-obligation parity proof;
- namespace/progressive-disclosure proof;
- package inventory proof;
- red fixture report;
- green fixture report;
- tamper fixture report;
- clean-room rebuild proof;
- agent-authored artifact provenance proof;
- source/install/cache digest comparison;
- package sync proof;
- issue lifecycle/update_goal eligibility proof;
- final completion decision.

Agents may run the CLI and repair failures. Agents may not reinterpret CLI failures. Agents may not manually mark a gate as complete after CLI failure. Agents may not claim that a CLI failure is "only a receipt issue," "only stale evidence," "only documentation," "not material," "reviewer acceptable," "blocked but okay," "claim-ceiling handled," or "safe to finish anyway" unless the CLI has parsed that disposition as a typed non-goal exclusion and blocked all related claims.

A reviewer may add judgment, lower claims, require repairs, or identify new failure modes. A reviewer may not override deterministic CLI failure or raise a claim ceiling above the CLI-computed ceiling.

### 89.3 Mandatory CLI Command Surface

The plugin must expose a stable CLI command surface. Command names may be refined during implementation, but equivalent authority operations must exist, be documented, be tested, be included in the installed plugin package, and be discoverable from a clean checkout.

The CLI must include commands equivalent to:

```text
ultragoal --version
ultragoal help
ultragoal doctor
ultragoal init
ultragoal retrofit
ultragoal classify-target
ultragoal law graph --strict
ultragoal law check --gate <gate-id>
ultragoal law check --all
ultragoal standards check --strict
ultragoal source-obligations check --strict
ultragoal foundational-trace check --strict
ultragoal namespace check --strict
ultragoal typed-boundaries check --strict
ultragoal line-caps check --strict
ultragoal coverage prove
ultragoal product init
ultragoal product prove-fitness
ultragoal product prove-cohesion
ultragoal product prove-success
ultragoal product check-claims
ultragoal capability discover
ultragoal capability prove
ultragoal source audit
ultragoal install audit
ultragoal cache audit
ultragoal registry probe
ultragoal app-surface probe
ultragoal target-repo audit
ultragoal package digest
ultragoal package inventory
ultragoal package verify
ultragoal review-target build
ultragoal review-target verify
ultragoal archive build
ultragoal archive verify
ultragoal review-round verify
ultragoal packet build
ultragoal packet verify
ultragoal receipts verify
ultragoal fixtures red
ultragoal fixtures green
ultragoal fixtures tamper
ultragoal fixtures all
ultragoal clean-room rebuild
ultragoal claim-ceiling compute
ultragoal explain <failure-id>
ultragoal failure capture
ultragoal failure promote
ultragoal issue check-lifecycle
ultragoal update-goal eligibility
```

Existing commands may remain only as compatibility wrappers. A compatibility wrapper must call the same typed authority kernel as the canonical command. A compatibility wrapper may not bypass parsing, law graph closure, receipt verification, fixture enforcement, claim-ceiling computation, or update_goal eligibility rules.

The CLI must expose machine-readable output for every command that affects evidence. Human-readable output may exist, but it may not be the authority format. Machine-readable output must be schema-versioned JSON, validated by the CLI before writing, and re-verifiable by the CLI after writing.

### 89.4 Canonical Law Graph

The CLI must build one canonical law graph for the candidate. This graph is the source of truth for whether Harness Ultragoal laws are connected to enforcement.

For every material law, the law graph must join:

- parent prompt gate ID;
- checklist item ID;
- source-obligation law ID;
- foundational article requirement ID;
- standards row ID;
- enforcement TSV row ID when applicable;
- enforcement JSON row ID when applicable;
- schema ID;
- schema version;
- validator check ID;
- validator module path;
- red fixture ID;
- green fixture ID when applicable;
- tamper fixture ID when applicable;
- receipt kind;
- receipt schema version;
- package inventory inclusion rule;
- claim classes blocked by failure;
- claim classes supported by success;
- proof surface required;
- same-surface restrictions;
- freshness requirements;
- candidate digest binding;
- install/cache/app/registry propagation requirements.

The CLI must fail law graph validation if any material law is:

- missing from the source-obligation matrix;
- missing from foundational traceability;
- missing from agent-standards enforcement rows;
- missing from schemas;
- missing from validator checks;
- missing from red fixtures;
- missing from green fixtures where a green path is possible;
- missing from tamper fixtures where artifact forgery is possible;
- missing from receipt requirements;
- missing from package inventory requirements;
- missing from claim-ceiling impact;
- represented only by prose;
- represented only by a row;
- represented only by a fixture name;
- represented only by a reviewer prompt;
- represented only by a checklist item;
- represented only by a claim-ceiling downgrade;
- represented only by a packet section;
- represented only by a non-actionable "blocker" note;
- represented only by historical evidence;
- represented only by a stale session summary;
- represented only by source proof when install/cache/app proof is required;
- represented only by install/cache proof when app/registry/reviewer proof is required;
- represented only by red fixtures with no valid green path;
- represented only by green fixtures with no red proof;
- represented by a check that is not actually executed by the strict audit command.

The law graph must include a digest. Every law receipt must bind to the law graph digest. If the law graph changes, prior law receipts become stale unless explicitly revalidated by the CLI.

### 89.5 Standards And Governing Repo Docs

CLI authority must be reflected in governing repo documents and package-controlled standards surfaces. The repo may not rely on informal agent memory or side-thread instructions to require CLI usage.

The following surfaces must be updated to require CLI-governed enforcement:

- `templates/agent-standards/enforcement.json`
- `templates/agent-standards/enforcement.tsv`
- `templates/agent-standards/enforcement-audit.tsv`
- `docs/source-obligation-matrix.md`
- foundational trace registry
- validator check registry
- red fixture catalog
- green fixture catalog
- tamper fixture catalog
- receipt schema catalog
- plugin manifest surfaces
- init templates
- retrofit templates
- ExecPlan templates
- review-round schemas
- review-round fixtures
- Product Fitness reviewer prompts
- Product Cohesion reviewer prompts
- Product Success reviewer prompts
- final packet schema
- review-target schema
- candidate archive schema
- update_goal eligibility schema if present
- package inventory schema if present

At least one explicit standards law must state:

> Harness Ultragoal completion, review readiness, package readiness, product readiness, release readiness, registry readiness, and update_goal eligibility must be computed or verified by the Harness Ultragoal CLI. Manual checklist updates, generated packet text, reviewer agreement, stale receipts, substituted proof, and agent-written summaries are not authority.

The standards law must fail closed. It must not be advisory, backlogged, blocked, documentation-only, reviewer-only, or claim-ceiling-only. It must have a standards row, source-obligation row, foundational trace entry, schema entry, validator check, red fixture, valid fixture, receipt requirement, and claim-ceiling guard.

### 89.6 Init And Retrofit Enforcement Hooks

The plugin must install or generate enforcement hooks during `init` and `retrofit` so downstream repos cannot treat CLI usage as optional.

The init and retrofit flows must create or verify a repo-local enforcement layout equivalent to:

```text
.harness/
  ultragoal.toml
  law-graph.lock.json
  schema-catalog.lock.json
  standards.lock.json
  source-obligations.lock.json
  fixture-catalog.lock.json
  receipts/
  reports/
  hooks/
  commands/
  product/
  coverage/
  packets/
  failures/
```

The CLI must install or generate command hooks equivalent to:

```text
.harness/commands/pre-completion
.harness/commands/pre-review-packet
.harness/commands/pre-update-goal
.harness/commands/pre-package
.harness/commands/pre-install
.harness/commands/pre-release
```

If the host ecosystem supports git hooks, task-runner hooks, CI hooks, plugin lifecycle hooks, or Codex/harness command hooks, the plugin must wire them to the CLI. If a hook surface is unavailable, the CLI must emit a typed hook-unavailable receipt and block only the claims that require that hook surface. It may not silently skip hook installation.

The required hook behavior:

- `pre-completion` runs strict law graph, strict standards, strict source obligations, strict audit, fixture reports, coverage, line caps, typed boundaries, Product Fitness/Cohesion/Success gates, and claim-ceiling computation.
- `pre-review-packet` refuses to build or verify a packet unless all packet claims derive from current CLI receipts.
- `pre-update-goal` refuses eligibility unless every mandatory stop condition has a current CLI receipt bound to the same candidate digest.
- `pre-package` refuses packaging if package inventory, private-path checks, bundled component graph, schemas, fixtures, receipts, and CLI binary inclusion fail.
- `pre-install` refuses installation if source compliance is incomplete or if installing would create source/install/cache divergence.
- `pre-release` refuses readiness or release claims unless same-surface proof exists for every released claim.

Generated hooks must be package-owned or repo-owned according to typed ownership metadata. The CLI must verify that hook files have not drifted from the installed plugin version unless the drift is represented by a typed local policy extension that cannot weaken mandatory laws.

### 89.7 Agent Standards Enforcement

The agent-standards surfaces must explicitly forbid agents from bypassing the CLI.

The standards must include fail-closed rules equivalent to:

1. Agents must use the Harness Ultragoal CLI for all Harness Ultragoal law claims.
2. Agents must not manually mark Harness Ultragoal checklist items complete without current CLI evidence.
3. Agents must not hand-author final packets, review targets, archives, claim ceilings, source/install/cache receipts, registry receipts, Product Fitness receipts, Product Cohesion receipts, Product Success receipts, coverage receipts, fixture reports, or update_goal eligibility records.
4. Agents must not treat reviewer agreement as a substitute for CLI law satisfaction.
5. Agents must not treat install success, package publication, smoke tests, first run, fixture pass counts, packet existence, source audit pass, or cache proof as substitutes for same-surface claims.
6. Agents must not downgrade a missing law to "not material" unless the CLI records a typed non-goal exclusion and blocks related claims.
7. Agents must not copy receipts across candidate versions, package digests, source digests, schema versions, law graph versions, or proof surfaces.
8. Agents must not claim current registry, app, plugin UI, marketplace, launcher, reviewer exposure, product usage, daily-driver, or release readiness without same-surface CLI proof.
9. Agents must promote newly observed material failure modes through the CLI failure-promotion path before completion.
10. Agents must treat CLI failure as authoritative until the code, schema, fixture, receipt, or proof surface is repaired and the CLI passes.

Each agent-standard rule must have a validator check and red fixture. A standards row that lacks CLI-backed enforcement must fail strict audit.

### 89.8 Receipt Authority And Anti-Fabrication

The CLI must be the receipt issuer and receipt verifier.

A receipt is valid only if it includes all required authority metadata:

- receipt kind;
- receipt schema version;
- CLI command name;
- CLI command argv;
- CLI binary digest;
- plugin version;
- source root;
- command cwd;
- execution mode;
- created-at timestamp;
- run ID;
- source digest;
- candidate version;
- candidate package digest;
- schema catalog digest;
- law graph digest;
- standards digest;
- source-obligation digest;
- fixture catalog digest;
- input artifact digests;
- output artifact digests;
- proof surface;
- package surface when applicable;
- runtime surface when applicable;
- capability authority when applicable;
- claim classes supported;
- claim classes blocked;
- stale-after rule;
- issuer identity;
- machine-readable result;
- human-readable explanation;
- failure IDs when failing.

Receipts must be rejected if:

- hand-authored;
- edited after minting;
- missing issuer data;
- missing command data;
- missing input digests;
- missing output digests;
- missing proof surface;
- missing candidate digest;
- missing law graph digest;
- missing schema catalog digest;
- stale by time;
- stale by source digest;
- stale by package digest;
- stale by law graph digest;
- stale by schema catalog digest;
- stale by standards digest;
- stale by source-obligation digest;
- stale by fixture catalog digest;
- produced by a different CLI binary without compatibility proof;
- produced by a different plugin version without compatibility proof;
- copied from source to install/cache/app surface;
- copied from install/cache to app/registry/reviewer surface;
- minted against a different target repo;
- minted against a different mode;
- minted against a different package version;
- referencing files outside allowed evidence roots;
- referencing private local paths in package-owned inventory;
- claiming support for a claim class not computed by the CLI.

A passing validator receipt is not sufficient unless the receipt itself passes receipt verification.

### 89.9 Packet Authority

The final packet must be built or verified by the CLI.

The CLI must reject a packet if:

- the packet was hand-authored without CLI packet provenance;
- any packet claim cannot be traced to a current CLI receipt;
- any packet evidence path is missing;
- any packet evidence path is stale;
- any packet evidence path points to a private local package-owned proof path;
- any packet claim exceeds the CLI-computed claim ceiling;
- any packet says reviewer-ready without same-surface reviewer exposure proof;
- any packet says registry-ready without same-surface registry proof;
- any packet says app-ready without same-surface app proof;
- any packet says release-ready without release-surface proof;
- any packet says Product Fitness passed without current Product Fitness proof;
- any packet says Product Success passed without current Product Success proof;
- any packet says source/install/cache synced without current digest comparison;
- any packet says all red fixtures pass without current red fixture report;
- any packet omits unsupported claims;
- any packet hides deterministic failures in prose;
- any packet presents blockers as acceptable completion.

The packet must include a CLI-generated claim ceiling. Agents may not hand-write the claim ceiling. Agents may add explanatory text only if the CLI verifies that the text does not raise the claim ceiling or imply unsupported claims.

### 89.10 Same-Surface Proof And Claim Separation

The CLI must encode proof surfaces as closed typed values. At minimum:

- `source`
- `package-archive`
- `installed-plugin`
- `versioned-cache`
- `app-registry`
- `plugin-ui`
- `marketplace`
- `launcher-runtime`
- `reviewer-exposure`
- `target-repo`
- `product-live-surface`
- `release-surface`

A proof from one surface must not satisfy another surface unless a typed same-surface equivalence rule exists. Equivalence rules must be explicit, schema-validated, red-fixtured, and claim-limited. No equivalence rule may allow disk install/cache proof to imply app registry, plugin UI, marketplace, launcher runtime, reviewer exposure, product live-surface, or release readiness.

The CLI must block these substitutions:

- source audit as install proof;
- source audit as cache proof;
- source audit as app-registry proof;
- source audit as reviewer exposure proof;
- installed-plugin audit as cache proof;
- cache audit as installed-plugin proof;
- install/cache proof as app-registry proof;
- install/cache proof as plugin UI proof;
- install/cache proof as marketplace proof;
- install/cache proof as launcher runtime proof;
- install/cache proof as reviewer exposure proof;
- registry proof as Product Fitness proof;
- reviewer exposure proof as Product Fitness proof;
- package publication proof as Product Success proof;
- smoke-test proof as product-readiness proof;
- fixture pass proof as user value proof.

If a live proof surface cannot be probed on the current machine, the CLI must emit a typed unsupported-surface result and block only the claims that require that surface. It may not replace unavailable live proof with adjacent proof.

### 89.11 Product Fitness, Product Cohesion, And Product Success Control Plane

The CLI must own product gates. Product claims must not be left to reviewer prose, template existence, report language, or packet text.

The product command group must enforce:

- Product Success contract creation at goal initiation or retrofit;
- Product Fitness proof tied to same candidate digest;
- Product Cohesion proof tied to same candidate digest;
- Product Success proof tied to user/job/value/release evidence;
- explicit product reviewer ownership;
- explicit Product Fitness falsifier disposition;
- explicit Product Cohesion disposition;
- explicit Product Success disposition;
- current source inspiration map;
- source-card freshness;
- product research/eval artifact disposition;
- strategy/positioning artifact disposition;
- template generation artifact disposition when relevant;
- claim class mapping for product-impacting claims.

The CLI must reject Product Fitness, Product Cohesion, or Product Success claims based on:

- documentation-only proof;
- install success;
- package publication;
- first run;
- first use;
- smoke test;
- fixture pass;
- source audit pass;
- cache audit pass;
- reviewer agreement;
- generic Product/Simplicity approval;
- Product Cohesion alone;
- Product Fitness alone when Product Success is claimed;
- happy-path demo alone;
- stale Product Fitness receipt;
- stale Product Cohesion receipt;
- stale Product Success receipt;
- proof from the wrong package digest;
- proof from the wrong source digest;
- proof from the wrong live surface;
- proof with no target user;
- proof with no job-to-be-done;
- proof with no first-value statement;
- proof with no daily-driver or release-surface evidence when those claims are made;
- proof with no falsifier disposition;
- proof with no claim-ceiling effect.

Product proofs must use typed schemas. Product proof schemas must reject unknown keys and unsupported claim classes. Product proof reports must be receipt-bound and package-included when the package makes product enforcement claims.

### 89.12 Coverage, Line Caps, Typed Boundaries, And Namespace Enforcement Through CLI

Coverage, line caps, typed boundaries, and namespace/progressive-disclosure laws must be CLI-governed.

The CLI must expose first-class commands for:

- coverage proof;
- line-cap proof;
- typed-boundary proof;
- namespace/progressive-disclosure proof.

These commands must not merely call external tools and trust output text. They must parse external output into typed receipts, bind receipts to source digest and candidate digest, and reject stale or malformed output.

Coverage proof must reject:

- test pass counts as substitute;
- smoke tests as substitute;
- fixture pass counts as substitute;
- reviewer approval as substitute;
- old coverage reports;
- coverage reports for the wrong source digest;
- coverage reports for the wrong package version;
- coverage reports with uncovered records;
- coverage reports not produced by the declared coverage command;
- coverage reports that omit repo-owned scope;
- coverage reports that exclude source files without typed exclusions.

Line-cap proof must reject:

- prose assertion that files are small enough;
- partial scans;
- missing generated-file classification;
- generated-file exceptions without typed justification;
- stale line-cap reports;
- line-cap reports not bound to source digest.

Typed-boundary proof must reject:

- ad hoc string validation at authority boundaries;
- raw JSON value use beyond boundary parser modules;
- unknown JSON keys;
- unchecked path strings;
- unchecked environment variables;
- unchecked CLI args;
- unchecked receipt fields;
- unchecked schema references;
- unchecked runtime capability names.

Namespace/progressive-disclosure proof must reject:

- generic utility buckets for law-bearing modules;
- umbrella names that hide authority boundaries;
- missing progressive-disclosure metadata for skills;
- law IDs buried only in prose;
- source-obligation rows without first-class law IDs;
- package surfaces whose namespace does not reveal authority role.

### 89.13 Red, Green, Tamper, Stale, Wrong-Surface, And Miswire Fixture Requirements

Every CLI-controlled law must include fixture coverage sufficient to prove both acceptance and rejection.

For each law, the fixture catalog must include or explicitly type why non-applicable:

- direct red fixture;
- minimal valid green fixture;
- realistic valid green fixture;
- stale receipt red fixture;
- wrong source digest red fixture;
- wrong package digest red fixture;
- wrong candidate version red fixture;
- wrong proof surface red fixture;
- wrong schema version red fixture;
- missing binding red fixture;
- substitute proof red fixture;
- hand-authored receipt red fixture;
- tampered receipt red fixture;
- validator miswire red fixture;
- orphaned law red fixture;
- row-shape-only red fixture;
- prose-only red fixture;
- reviewer-only red fixture.

A law with only red fixtures is not complete. A law with only green fixtures is not protective. A law with no tamper path for generated artifacts is gameable. A law with no miswire fixture is vulnerable to validator theater. A law with no stale fixture is vulnerable to receipt laundering.

The CLI must produce a fixture report that proves:

- every required red fixture fails for the intended reason;
- every required green fixture passes for the intended reason;
- tampered artifacts are rejected;
- stale artifacts are rejected;
- wrong-surface artifacts are rejected;
- wrong-digest artifacts are rejected;
- miswired checks are detected;
- fixture counts are current;
- fixture catalog digest matches the law graph;
- fixture reports are receipt-bound.

### 89.14 Failure Capture And Promotion

The CLI must provide a mandatory failure capture and promotion path.

Any newly observed material failure mode must become a typed failure record. Sources include:

- parent session logs;
- Chronicle summaries;
- side-thread audits;
- reviewer feedback;
- validator failures;
- stale receipt discoveries;
- stale red fixture discoveries;
- product proof substitutions;
- registry/app proof overclaims;
- source/install/cache drift;
- package inventory holes;
- packet correctness gaps;
- standards prose-only rows;
- foundational trace gaps;
- manually checked checklist items;
- attempted update_goal() before eligibility;
- agent attempts to finish after blockers;
- claims made from unsupported surfaces;
- external plugin/runtime capability mismatches;
- deep-research or source-card misses;
- target repo audit misses;
- trust-boundary abuse paths;
- malicious or accidental artifact tampering.

Each failure record must include:

- failure ID;
- source artifact path or source session ID;
- observed timestamp if available;
- law ID affected;
- gate ID affected;
- claim classes affected;
- affected proof surface;
- observed behavior;
- exploit path;
- required repair type;
- required validator check;
- required schema change if applicable;
- required red fixture;
- required green fixture if applicable;
- required tamper fixture if applicable;
- receipt impact;
- package impact;
- claim ceiling impact;
- disposition;
- promotion evidence path.

Allowed dispositions:

- `promoted_to_law`
- `promoted_to_validator`
- `promoted_to_schema`
- `promoted_to_fixture`
- `promoted_to_receipt_requirement`
- `promoted_to_claim_guard`
- `promoted_to_package_inventory_rule`
- `typed_non_goal_with_claims_blocked`

Disallowed dispositions:

- `ignored`
- `future`
- `backlog`
- `blocked_but_ok`
- `reviewer_accepted`
- `documented_only`
- `claim_ceiling_only`
- `not_material` without typed non-goal and claim blocking.

Completion must fail while any material failure record is unpromoted.

### 89.15 Update Goal Eligibility

The CLI must own update_goal() eligibility.

The parent session may not call update_goal() until `ultragoal update-goal eligibility` returns a passing receipt for the same candidate digest and source/install/cache/app-surface status required by the current claim ceiling.

The eligibility command must verify:

- all mandatory gates pass;
- all stop conditions pass;
- checklist state matches CLI receipts;
- all checked checklist items cite current evidence;
- all evidence is same-candidate;
- all evidence is same-digest where required;
- all red fixture reports pass;
- all green fixture reports pass;
- all tamper fixture reports pass;
- source audit passes;
- installed audit passes if installation occurred;
- cache audit passes if cache package exists;
- app/registry/reviewer claims have same-surface proof or are blocked;
- Product Fitness is current;
- Product Cohesion is current;
- Product Success is current where claimed;
- coverage is 100 percent for declared repo-owned scope;
- line caps pass;
- typed boundaries pass;
- standards rows are mechanized;
- foundational trace is complete;
- source obligations are first-class or typed parent/child enforced;
- package inventory has no private local proof paths;
- version bump and package sync are complete if required;
- final packet is CLI-built or CLI-verified;
- no unpromoted failure records remain;
- claim ceiling does not exceed proof.

If the eligibility command fails, parent session completion must fail. The final packet may describe the failure only as an unsupported claim state; it may not call the goal complete.

### 89.16 Clean Checkout And Installed Plugin Discovery

The CLI must be discoverable and runnable from a clean checkout and from the installed plugin package.

The plugin must prove:

- CLI binary/source is included in package inventory;
- CLI command discovery works from source checkout;
- CLI command discovery works from installed plugin;
- CLI command discovery works from versioned cache package;
- init/retrofit setup can locate the CLI;
- generated hooks call the correct CLI path or command alias;
- command paths do not depend on private local source paths;
- clean checkout bootstrap can install or build the CLI using documented commands;
- source/install/cache command versions agree;
- source/install/cache command digests agree where expected;
- version mismatch fails audit;
- missing CLI fails package readiness;
- missing CLI fails review readiness;
- missing CLI fails release readiness;
- missing CLI fails update_goal eligibility.

### 89.17 Config, Defaults, Environment, And Secret Boundaries

CLI configuration must be typed and fail closed.

The CLI must reject:

- unknown config keys;
- unsupported config versions;
- implicit defaults for authority-bearing settings;
- environment variables used as direct authority;
- secret values written into receipts;
- token values written into packets;
- private local paths written into package-owned manifests;
- host-specific paths in portable package inventory;
- unredacted command environment in evidence;
- config precedence ambiguity;
- conflicting config layers;
- config that weakens mandatory laws;
- local override files that weaken mandatory laws.

Allowed configuration layers must be explicitly ordered. The CLI must emit a config-resolution receipt showing which values came from package defaults, repo config, environment indirection, command-line flags, and generated lockfiles. Authority-bearing values must be lockfile-bound or receipt-bound.

### 89.18 Capability Discovery And Same-Surface Capability Authority

The CLI must own connector and capability discovery.

The CLI must distinguish:

- capability not installed;
- capability installed but not configured;
- capability configured but not authenticated;
- capability authenticated but not authorized for required action;
- capability authorized but not same-surface;
- capability same-surface but stale;
- capability same-surface and current.

A capability from one connector, plugin, app, registry, cache, or source package must not authorize claims for another surface.

The CLI must reject:

- source package proof as active connector capability;
- installed plugin proof as app capability;
- cache proof as registry capability;
- registry listing as reviewer exposure;
- reviewer packet existence as reviewer exposure;
- app plugin visibility as product usage;
- connector presence as same-surface proof;
- authenticated connector as authorized action proof;
- stale capability discovery as current proof.

### 89.19 Productized CLI User Experience Without Weakening Strictness

The CLI must be usable enough that strictness does not become an excuse for bypass.

Every failing command must produce:

- stable failure ID;
- law ID;
- gate ID;
- check ID;
- failing input path;
- expected authority shape;
- actual parsed result;
- claim classes blocked;
- minimal repair guidance;
- command to rerun;
- related fixture ID if applicable.

The CLI must include an `explain` command. `explain` must not weaken any law. It exists to make deterministic failure actionable.

The CLI must provide a fast focused mode for local repair and a full strict mode for completion. Focused mode may help repair but cannot satisfy final completion, review readiness, release readiness, or update_goal eligibility.

### 89.20 Required Validation Evidence For This Gate

This gate is not satisfied unless all of the following evidence exists and is current for the same candidate digest:

- CLI authority kernel tests pass.
- CLI command discovery proof exists for source.
- CLI command discovery proof exists for installed plugin if installed.
- CLI command discovery proof exists for cache package if cache package exists.
- Strict law graph receipt passes.
- Standards CLI authority row is present and mechanized.
- Source-obligation CLI authority row is present and mechanized.
- Foundational trace entry for CLI authority is present and mechanized.
- Schema catalog includes CLI authority schemas.
- Validator includes CLI authority checks.
- Red fixture report proves CLI bypass attempts fail.
- Green fixture report proves valid CLI-governed evidence passes.
- Tamper fixture report proves forged/edited/generated artifacts are rejected.
- Packet verification rejects hand-authored unsupported packets.
- Checklist verification rejects manually checked items without CLI evidence.
- Receipt verification rejects stale, copied, wrong-surface, wrong-digest, wrong-schema, and hand-authored receipts.
- Product proof commands reject all forbidden substitutes.
- Same-surface proof commands reject disk/cache proof for app/registry/reviewer claims.
- update_goal eligibility command fails when any mandatory gate lacks CLI evidence.
- update_goal eligibility command passes only when all mandatory gates and stop conditions pass.
- Final packet is CLI-built or CLI-verified.
- Claim ceiling is CLI-computed.
- No unpromoted material failure records remain.

### 89.21 CLI Self-Law Compliance And Self-Hosting

The CLI is not exempt from any Harness Ultragoal law. The CLI tool, validator source, schemas, receipts, fixtures, reports, package inventory, generated hooks, command wrappers, init/retrofit outputs, configuration, product surfaces, and documentation it ships must obey the same laws the CLI enforces against target repos and plugin packages.

The CLI may not act as a privileged root of trust that bypasses its own standards. The CLI may be the authority kernel only after it proves that the authority kernel itself is law-governed, typed, covered, line-capped, product-bound, receipt-bound, source/install/cache honest, package-included, clean-room reproducible, tamper-resistant, same-surface disciplined, and claim-ceiling constrained.

The CLI must include a self-law compliance path equivalent to:

```text
ultragoal self audit --strict
ultragoal self law-graph --strict
ultragoal self fixtures red
ultragoal self fixtures green
ultragoal self fixtures tamper
ultragoal self update-goal eligibility
```

Command names may be refined during implementation, but equivalent self-law authority operations must exist. A generic source audit is insufficient unless it explicitly covers CLI self-law scope and emits a typed self-law receipt.

Self-law scope must include the full list below:

- CLI command parser and command dispatch;
- CLI boundary parsers for JSON, TOML, Markdown, schemas, receipts, fixture catalogs, environment variables, paths, command output, package inventory, and runtime capability data;
- validator modules;
- schema catalog;
- law graph builder;
- standards/source-obligation/foundational-trace join logic;
- receipt issuer;
- receipt verifier;
- packet builder;
- packet verifier;
- archive builder;
- archive verifier;
- review-target builder;
- review-target verifier;
- Product Fitness, Product Cohesion, and Product Success commands;
- source/install/cache/app/registry/reviewer proof commands;
- coverage command integration;
- line-cap command integration;
- namespace/progressive-disclosure command integration;
- typed-boundary command integration;
- init/retrofit command outputs;
- generated hooks;
- config resolution;
- secret redaction;
- capability discovery;
- failure capture;
- failure promotion;
- update_goal eligibility computation;
- focused repair mode;
- strict completion mode.

The CLI must prove its own compliance with every law it enforces, including:

- 100 percent coverage for declared repo-owned CLI/validator scope, with uncovered records empty;
- typed parsing at every CLI boundary;
- line-cap adherence for CLI/validator source;
- namespace/progressive-disclosure law for CLI/validator modules, commands, and packaged skills;
- Product Fitness, Product Cohesion, and Product Success law for the CLI as a product surface;
- source/install/cache/app-registry separation for CLI package claims;
- standards fail-closed behavior for CLI authority;
- foundational traceability for CLI authority;
- source-obligation parity for CLI authority;
- red, green, stale, wrong-surface, wrong-digest, tamper, substitute-proof, and miswire fixtures for CLI authority;
- generated/proof artifact provenance for CLI-generated artifacts;
- clean-room rebuild for CLI artifacts and receipts;
- config precedence, unknown-key rejection, environment indirection, and redaction for CLI config;
- trust-boundary abuse-path and failure-path coverage for CLI inputs, outputs, runtime calls, filesystem access, registry/app probes, secrets, and destructive operations;
- runtime feasibility, cost, strict-gate usability, and agent-actionable remediation output for CLI commands;
- schema evolution and stale-version invalidation for CLI schemas and receipts.

Bootstrap enforcement may exist only as a typed transitional state. A bootstrap validator, pre-self-hosted CLI, or compatibility wrapper may generate `bootstrap_untrusted` or `transition_only` receipts, but those receipts cannot support completion, package readiness, review readiness, product readiness, release readiness, registry readiness, app readiness, reviewer exposure, or update_goal eligibility. Final compliance requires a self-hosted CLI self-law receipt generated by the same candidate CLI and bound to the same candidate digest.

The CLI must reject every self-exemption path:

- CLI command claims outside the law graph;
- CLI source excluded from coverage without typed exception and claim blocking;
- CLI source excluded from line-cap checks without typed generated/mechanical exception;
- CLI boundary parser accepting raw authority strings after parse;
- CLI receipts accepted without self-law issuer verification;
- CLI-generated packets accepted without packet verifier self-law proof;
- CLI hooks accepted without hook self-law proof;
- CLI config accepted with unknown authority keys;
- CLI package inventory omitting CLI law-bearing artifacts;
- CLI fixtures proving target-repo behavior but not CLI self-behavior;
- CLI source audit used as substitute for installed/cache CLI proof;
- pre-self-hosted validator receipts used for final completion;
- final packet claiming CLI authority without current CLI self-law proof.

The CLI self-law gate must have governing documentation and enforcement, not only this prompt text. It must be represented in agent-standards enforcement rows, source-obligation rows, foundational trace entries, schema catalog, validator checks, red fixtures, green fixtures, tamper fixtures, receipt requirements, package inventory, claim-ceiling guards, validation evidence, final packet, and update_goal eligibility.

Add red fixtures for CLI self-exemption, CLI source excluded from coverage, CLI parser raw-string authority escape, CLI line-cap omission, CLI namespace omission, CLI receipt accepted without self-law proof, CLI package inventory missing CLI law artifact, bootstrap receipt used for completion, CLI source proof substituted for installed/cache CLI proof, CLI generated packet accepted without self-law proof, and update_goal eligibility passing without CLI self-law receipt.

### 89.22 CLI Performance, Latency, Speed, And Iteration Fitness

The CLI must be fast enough to be used constantly. A dictatorship-level control plane that is too slow for routine agent iteration is not a real control plane; it becomes a burden that agents will avoid, defer, summarize around, or replace with stale proof. Performance, latency, cache honesty, concurrency, and speed are therefore Harness Ultragoal laws, not polish.

The foundational trace for this law must explicitly include the "AI Is Forcing Us To Write Good Code" requirements for fast, ephemeral, concurrent dev environments, fast automated guardrails, short change-check-fix loops, cheap repeated test/check execution, high-concurrency isolated runs, cache-backed third-party calls with no-cache verification, one-command setup, and conflict-free concurrent environments. It must also bind those article requirements to standards rows, source-obligation rows, validator checks, schemas, red fixtures, green fixtures, receipts, package inventory, claim-ceiling guards, and final packet evidence.

The CLI must define hard performance budgets for every command class. Budgets may be made stricter by repo policy, but they may not be absent, advisory, prose-only, or hidden in documentation. A command without a typed budget cannot support completion, review readiness, package readiness, product readiness, release readiness, registry readiness, app readiness, or update_goal eligibility.

Required default budget classes:

- `instant`: `ultragoal --version`, `ultragoal help`, command discovery, schema version lookup, and static command metadata. Cold p95 must be <= 2 seconds. Warm p95 must be <= 500 milliseconds.
- `interactive`: `ultragoal explain`, single failure lookup, single receipt verification, single law status lookup, single claim-ceiling query, and single checklist evidence lookup. Cold p95 must be <= 5 seconds. Warm p95 must be <= 1 second.
- `hot_edit_check`: sub-file or affected-surface edit check. Cold p95 must be <= 5 seconds and the command must be safe to run constantly while editing.
- `focused`: one gate, one law, one fixture, one source-obligation row, one standards row, one product proof join, one typed-boundary class, or one package-inventory check. Cold p95 must be <= 15 seconds. Warm p95 must be <= 5 seconds.
- `standard_source_local`: commands expected after a small code change, including focused audit, focused fixture run, focused receipt verification, focused law graph closure, focused package inventory check, and focused claim-ceiling recomputation. Cold p95 must be <= 30 seconds. Warm p95 must be <= 10 seconds.
- `strict_local`: full local strict audit excluding coverage, live external probes, and clean-room rebuild. p95 target must be <= 60 seconds on the declared baseline machine and declared repository size class.
- `strict_fixtures`: full red, green, stale, wrong-surface, wrong-digest, tamper, substitute-proof, and miswire fixture suite excluding live external probes. p95 target must be <= 60 seconds on the declared baseline machine and declared fixture count class.
- `strict_coverage`: full exact coverage proof. p95 target must be <= 60 seconds and hard ceiling must be <= 180 seconds. It must not be unbounded, and it must not rely on coverage substitutes. If the repo cannot meet its declared strict coverage budget, product/readiness/update_goal claims are blocked until source structure, test structure, concurrency, caching honesty, command granularity, or instrumentation is repaired.
- `strict_final`: full final source-local proof, including source audit, red/green/tamper fixtures, coverage, package inventory, source/install/cache comparison when applicable, product proofs, packet verification, and claim-ceiling computation. p95 target must be <= 60 seconds and hard ceiling must be <= 180 seconds on the declared baseline machine unless a stricter repo policy applies. Any exception requires typed performance debt, command splitting, parallelization, honest digest-keyed caching, claim blocking, and a standards-gardener repair path; it cannot support product readiness, routine usability, release readiness, or update_goal eligibility.
- `external_live`: registry, app, marketplace, launcher, reviewer-exposure, connector, network, or third-party live probes. Each live probe must have a typed timeout, retry/backoff policy, same-surface capability authority, offline fallback behavior, claim-blocking behavior, and redacted telemetry. External slowness may block only dependent live-surface claims, but it may not excuse local CLI slowness.

The CLI must emit performance receipts for every evidence-affecting command. Performance receipts must include:

- command name;
- command argv;
- command class;
- budget version;
- budget threshold;
- source digest;
- candidate digest;
- CLI binary digest;
- schema catalog digest;
- law graph digest;
- standards digest;
- fixture catalog digest;
- input size metrics;
- output size metrics;
- file count scanned;
- fixture count executed;
- receipt count read;
- receipt count written;
- cache mode;
- cache key;
- cache hits;
- cache misses;
- no-cache mode result when required;
- concurrency level;
- worker count;
- queue depth when applicable;
- start timestamp;
- end timestamp;
- wall-clock duration;
- CPU duration when available;
- peak memory when available;
- relevant IO bytes when available;
- external call count;
- external wait duration;
- timeout count;
- retry count;
- exit code;
- claim classes supported;
- claim classes blocked by performance failure.

The CLI must support fast iterative use and honest final proof at the same time:

- Focused and repair-loop commands may use verified caches only when cache keys include source digest, candidate digest, CLI binary digest, schema catalog digest, law graph digest, standards digest, source-obligation digest, fixture catalog digest, config digest, and command arguments.
- Final strict proof must include no-cache execution or cache-validation execution sufficient to prove no hidden stale cache dependency.
- A focused command may help repair a law but cannot satisfy final completion, review readiness, package readiness, product readiness, release readiness, registry readiness, app readiness, or update_goal eligibility.
- A cache hit may reduce runtime only when the CLI proves the cache entry is same-candidate, same-digest, same-schema, same-law-graph, same-fixture-catalog, same-config, and same-command.
- A cache miss may not silently downgrade proof; it must either compute fresh proof within budget or fail with a typed performance/availability claim impact.
- A live external timeout may not be replaced by stale local proof or source/install/cache proof.

The CLI must be scalable and bounded:

- Parallelism, multi-threading, and concurrency are the default posture for every
  safe CLI, plugin, validator, fixture, receipt, package, setup, retrofit,
  observability, shell-helper, and proof path. Serial execution is allowed only
  for typed authority-write, destructive/mutating, or externally constrained
  phases that declare why serial execution is required. Agents must not be able
  to skip available safe parallelism by omission, convention, local wrapper,
  shell script, or hidden global lock.
- Safe phases must use typed task classes: `pure_read_parallel`,
  `isolated_temp_write_parallel`, `external_live_bounded_parallel`,
  `shared_authority_write_serial`, and `destructive_or_mutating_serial`.
  Every law-bearing command or helper that performs multiple independent units
  of work must either execute through the scheduler/executor or emit a typed
  fail-closed reason showing why no parallelization is possible.
- The default worker count must be `available_parallelism - 1`, minimum `1`,
  with an explicit bounded `--jobs N` or equivalent for supported commands.
  Unbounded worker counts, hidden serial global locks, nondeterministic result
  ordering, shared `validation_artifacts/**` writes from workers, stale shared
  caches across workers, and fixture temp-state leaks are hard failures.
- Scheduler/performance receipts must record worker count, task count, queue
  depth, wall time, CPU time when available, memory and IO when available, cache
  mode, resource-measurement status, candidate digest, and claim impact. Missing
  duration, fake placeholder timing, or absent concurrency metadata blocks
  speed, routine-usability, product-readiness, release, and update_goal claims.
- Every law-bearing scan must declare its input size model and expected complexity class.
- Full strict commands must reject unbounded recursion, unbounded globbing, unbounded network calls, unbounded subprocess fan-out, unbounded model/tool loops, global locks that serialize independent work, and hidden shared mutable cache state.
- Concurrent execution must allocate ports, temp directories, cache namespaces, database names, log paths, receipt paths, and worker IDs without cross-talk.
- Performance baselines must be measured on a declared baseline machine and declared repository size class. Claims about speed must not be made without this baseline.
- Performance regressions must be detected against committed baselines. A regression beyond the typed tolerance blocks routine-usability, product-readiness, release-readiness, and update_goal claims.

The CLI must be product-fit as a tool:

- `ultragoal init` and `ultragoal retrofit` must provide one-command setup paths.
- Check-only init/retrofit must complete within the `interactive` or `focused` budget class.
- Local no-network init/retrofit must complete within the declared `repair_loop` budget unless package installation or compilation is explicitly required and separately budgeted.
- Manual multi-step setup, undocumented environment tinkering, hidden local state, and slow setup that causes agents to avoid fresh environments are product failures, not acceptable inconvenience.

Mandatory Product Usage Fitness and CLI Discoverability:

- Harness Ultragoal must be usable as a product for plugin-activated repositories,
  not only as a self-audit tool for its own package.
- The CLI must provide one obvious routine entrypoint for ordinary required local
  validation, while preserving leaf commands for advanced/debug usage.
- Top-level and subcommand help must be self-contained enough for an un-oriented
  agent or user to discover what to run, when to run it, why it matters, which
  proof surface it affects, and which claims it cannot support.
- `fit-repo` must be visible as the first plugin-activated repository path from
  the executable CLI surface, not only from plugin metadata, skill text, or docs.
- Target-repo/plugin-activated usage must be visible from CLI help and routine
  command flow, not hidden behind source-audit trivia or static fixture proof.
- `scripts/check` must either delegate to the routine CLI validation path or
  explicitly declare itself a narrow helper whose pass cannot satisfy routine,
  product-readiness, readiness, release, final-packet, or update_goal claims.
- Product/routine usability claims are blocked when required validation exists
  only as scattered manual leaf commands, help output is a dense usage line
  without command groups/examples/next-step guidance, `fit-repo` is clearer in
  plugin metadata than in executable CLI surfaces, target-repo validation lacks
  an obvious operator path, or focused checks/source audit/coverage/product
  receipts/red reports can be substituted for final or routine proof without
  explicit claim ceilings.
- Add focused tests and red fixtures for missing routine entrypoint,
  non-navigable help, leaf-only validation substitution, hidden fit-repo path,
  omitted target-repo routine path, and `scripts/check` substituting for the
  routine CLI path without a narrow-helper claim ceiling.

Mandatory Builder-Contract And Package-Boundary Separation:

- The parent-session full-compliance prompt, checklist, and execution spine are
  agent-governing builder contracts for this work session. They are not package
  resources, plugin product surfaces, coverage targets, package digest inputs,
  shipped law evidence, valid fixture dependencies, product receipts,
  review/archive contents, install inputs, cache inputs, registry inputs, or
  update_goal evidence.
- The package may contain reusable laws, schemas, templates, fixtures, skills,
  docs, receipts, and generated artifacts produced from the work, but it must not
  depend on session-specific parent prompt/checklist/spine files.
- Editing the parent prompt/checklist/spine may change agent instructions,
  execution order, or checklist progress state, but it must not stale package
  digest, coverage, source audit, Product/Fit/Journey, review target, archive,
  install/cache, registry, final packet, or update_goal receipts.
- The CLI must fail closed if package inventory, plugin manifests, coverage
  manifests, package digest logic, package closure, valid fixtures, receipts,
  source audit checks, final packet proof, install/cache proof, or update_goal
  eligibility treat the parent prompt/checklist/spine as package-owned evidence.
- Add focused tests and red fixtures proving parent-session contract files are
  excluded from package digest, package inventory closure, coverage changed-file
  coupling, plugin manifest resources, plugin cohesion resources, valid fixture
  evidence, receipt dereferencing, final packet proof, and update_goal proof.

Mandatory Control-Loop Discipline, Validation Budget, and Receipt Boundaries:

- Receipts are claim-bound artifacts, not progress journal entries. Checklist
  rows must use concise progress statuses only and must not become receipt
  ledgers, progress logs, or stale checked boxes.
- Receipt minting or refresh is allowed only at claim-bearing slice closure, a
  phase gate requiring same-candidate evidence, final source-local proof
  assembly, or package/install/cache/final-packet/update_goal proof that is
  actually in scope.
- During implementation, agents must prefer focused tests, direct source
  inspection, stdout, logs, metrics, traces, and explain output over broad
  receipt churn. Broad source audit, red report, coverage, Product/Fit/Journey,
  Rust/GC, self-law, update_goal, and final-packet receipts must not be
  regenerated after every small edit.
- Inner-loop checks may be tool-driven: fmt/build, focused unit tests, line-cap,
  package digest, schema validation, targeted receipts, and focused
  red/green/tamper tests.
- Slice-boundary closure requires current digest, focused tests, touched
  red/green/tamper proof, current same-candidate receipts, claim guard, targeted
  manual source/runtime inspection of the changed claim path, checklist status
  updates only, and source-local/not-readiness commit when coherent.
- Broad-boundary closure requires exact coverage, source audit, red fixture
  report, standards, source-obligation, and foundational trace closure on the
  same candidate.
- Completion-boundary closure requires full E2E/manual dogfood, CLI self-law,
  update_goal eligibility, final packet, install/cache/app-registry, and reviewer
  surfaces. Full manual E2E is not required between deterministic inner-loop
  checks.
- Manual validation is mandatory only at claim-boundary points: gate completion,
  new or changed validator/check/schema/claim guard, changed red/green/tamper
  semantics, final packet/update_goal/readiness/install/cache/app-registry
  surfaces, Product Fitness/Product Success claims, OpenAI/promptfoo/HALO
  authority, and suspicious CLI passes. It must inspect real source/runtime
  behavior and tune the validator rather than creating universal
  manual-receipt theater.
- Repeated broad audit loops are forbidden unless implementation or evidence
  semantics changed. Use Gate 92 style repair: run the narrow failing command
  once, query logs/metrics/traces by run_id/correlation_id/current digest,
  explain the failure, repair the smallest production cause, rerun the narrow
  command, verify changed telemetry, then run broad audit once.
- Worktree lanes must not launch until the in-process validator/CLI parallelism,
  observability timing, current scheduler slice, Product Usage Fitness slice, and
  Phase 4 same-candidate source-local graph are closed and committed.
- No install/cache refresh, version bump, final packet finalization,
  registry/reviewer exposure claim, readiness/release/completion claim, or
  update_goal is allowed until same-candidate source/install/cache/app-registry/
  reviewer/final-packet/update_goal evidence supports that exact surface.

The CLI must reject performance theater:

- no performance budget;
- budget declared only in prose;
- performance receipt missing;
- stale performance receipt;
- performance receipt from wrong candidate digest;
- performance receipt from wrong CLI binary digest;
- performance receipt from wrong schema/law/fixture digest;
- command over budget without claim blocking;
- focused check substituted for strict final proof;
- cached proof substituted for no-cache proof where no-cache is required;
- hidden stale cache pass;
- unbounded concurrency;
- serial global lock blocking independent work;
- network call in local-only command;
- missing timeout;
- missing retry/backoff policy for live probe;
- missing external-call telemetry;
- missing input-size telemetry;
- missing fixture-count telemetry;
- missing cache-key telemetry;
- missing performance regression baseline;
- final packet omitting CLI performance status.

The CLI performance law must have governing documentation and enforcement, not only this prompt text. It must be represented in agent-standards enforcement rows, source-obligation rows, foundational trace entries, schema catalog, validator checks, red fixtures, green fixtures, tamper/stale-cache fixtures where applicable, receipt requirements, package inventory, claim-ceiling guards, validation evidence, final packet, and update_goal eligibility.

Add red fixtures for missing command budget, prose-only budget, over-budget focused command accepted, over-budget strict command accepted, stale performance receipt accepted, wrong-candidate performance receipt accepted, wrong-CLI-digest performance receipt accepted, focused check substituted for final proof, cached proof substituted for required no-cache proof, hidden network call in local command, unbounded worker fan-out, global lock serialization, missing timeout on live probe, cache key missing law graph digest, performance regression ignored, init requiring manual setup, and final packet claiming routine usability without current CLI performance receipt.

### 90. Validator Source Namespace Topology And Semantic Repo-Law Enforcement

This gate is additive to Gates 1-89.22. It does not replace, narrow, weaken, summarize, defer, or supersede namespace/progressive-disclosure law, semantic domain-type naming law, line caps, typed parsing, coverage, CLI self-law compliance, package inventory closure, standards fail-closed behavior, foundational traceability, source-obligation parity, validator-theater resistance, green-path adequacy, source/install/cache/app-surface separation, review packet correctness, version bump, or any update_goal() stop condition.

The validator source tree is itself an agent-facing product surface. It is not exempt from the laws it enforces. A validator that accepts broad top-level `internal_*` and `internal_coverage_*` file clusters while claiming namespace and semantic repo-law enforcement is demonstrating validator theater: the law exists, but the law is not binding on the repo's own most important enforcement surface.

Current side-thread evidence that must be verified live before repair:

- `validator/src` currently has 155 top-level Rust files.
- `validator/src` currently has 105 top-level `internal_*.rs` files.
- `validator/src` currently has 16 top-level `internal_coverage*.rs` files.
- `validator/src/internal_test_modules.rs` currently routes many top-level internal test modules from one flat namespace.
- `plugin-manifest-draft.json` currently lists the top-level `validator/src/internal_*.rs` files as package resources.
- `docs/namespace-law-exceptions.json` currently contains `repeated-prefix-validator-src-internal`, with `directory = "validator/src"`, `prefix = "internal"`, `kind = "external_contract"`, and `applies_to = ["validator/src/internal*"]`.
- The broad exception reason says the flat package contract exposes related public surfaces by stable names and the plugin manifest is the routing owner. That is not acceptable for repo-owned validator source/test topology. A package manifest may route shipped public artifacts, but it cannot justify a broad hand-authored source namespace escape hatch.

Foundational article and repo-law basis:

- `docs/source-article-synthesis.md` records the foundational requirements that repository knowledge is the system of record, invariants should be mechanically enforced, strong tests, clear docs, scoped modules, static types, reproducible dev environments, and small well-scoped files are essential for agents, and the filesystem is an interface for agents.
- `templates/agent-standards/01-namespace-and-progressive-disclosure.md` states that directory structure and filenames must explain domain responsibility before a file is opened, repeated prefixes across more than two files usually mean a missing subdirectory with the prefix removed, and compatibility exceptions must name the external contract that makes a less-ideal name worth keeping.
- `docs/source-obligation-matrix.json`, `docs/source-obligation-matrix.md`, `docs/foundational-law-traceability.json`, `templates/agent-standards/enforcement.json`, `templates/agent-standards/enforcement.tsv`, and `templates/RED_FIXTURES.json` must all represent this as enforceable same-law authority. A row, trace entry, fixture catalog entry, or claim-ceiling sentence is not enough unless non-compliant validator source topology fails through the same authority path used for completion, review, package, readiness, release, and update_goal eligibility.

Required source topology repair:

- Move validator self-tests and internal test surfaces out of flat `validator/src/internal_*.rs` names and into semantically routed directories with the repeated prefix removed. Acceptable shape includes paths such as:
  - `validator/src/self_tests/coverage/...`
  - `validator/src/self_tests/claim/...`
  - `validator/src/self_tests/cli/...`
  - `validator/src/self_tests/review/...`
  - `validator/src/self_tests/schema/...`
  - `validator/src/self_tests/target_repo/...`
  - `validator/src/self_tests/package/...`
  - `validator/src/self_tests/audit/...`
  - `validator/src/self_tests/product/...`
  - `validator/src/self_tests/boundaries/...`
- Remove `internal_` from final validator test filenames where the directory already communicates test/internal scope.
- Remove coverage-wave history from final filenames where it is only evidence of how a file was created. Names such as `coverage_wave82c_tests.rs` or `internal_coverage_waveNN_tests.rs` must be replaced with domain-behavior names such as `receipt_authority.rs`, `schema_dispatch.rs`, `target_fixture_boundaries.rs`, or `semantic_receipt_boundaries.rs`.
- Split large test clusters by domain responsibility, not by chronological wave, coverage chase, or implementation history.
- Preserve line caps while moving modules. The repair must not create a new over-cap file or hide over-cap behavior behind generated/mechanical exceptions.
- Update Rust module routing (`mod.rs`, `#[path]`, or equivalent) so tests remain discoverable by domain. A single mega-router that merely reintroduces an opaque flat list is not enough.
- Update `plugin-manifest-draft.json`, `.codex-plugin/plugin.json` if applicable, package inventory surfaces, component graph, schema catalog, review target/archive surfaces, and source/install/cache package surfaces so moved files are listed exactly once and stale top-level paths disappear.
- Preserve or improve 100 percent coverage after the topology repair. A moved test file cannot become an excuse for coverage regression, fixture drift, or package inventory drift.

Required namespace-law enforcement changes:

- Delete the broad `repeated-prefix-validator-src-internal` exception from `docs/namespace-law-exceptions.json`.
- Forbid broad repo-owned source exceptions for `validator/src/internal*`, `validator/src/*_wave*`, `validator/src/*coverage*` where the prefix is standing in for a directory, and typo variants such as `iinternal_*`.
- Tighten namespace parsing so exceptions are typed authority, not raw string loopholes. Exception records must parse into closed kinds such as:
  - generated fixture/catalog exception;
  - public distribution surface exception;
  - external compatibility surface exception;
  - narrow source-layout exception.
- Narrow source-layout exceptions are allowed only when they name a specific file or narrow file family, name the external contract or generator that requires the layout, prove no semantic directory can express the responsibility better, and lower or preserve claim ceilings. They cannot apply to broad hand-authored source globs.
- Namespace validation must inspect actual repo-owned source files as well as package manifest resources. A missing manifest entry must not let source topology escape the law, and a manifest entry must not bless non-compliant source topology.
- Repeated-prefix validation must distinguish generated fixture catalogs from hand-authored source. Generated red fixtures may be catalog-routed when `templates/RED_FIXTURES.json` or another generator is the real route. Hand-authored validator source/test modules must route through semantic source directories.
- Namespace validation must fail when a directory has more than two hand-authored files sharing a non-semantic prefix and no typed narrow exception. Prefixes such as `internal`, `coverage`, `claim`, `review`, `schema`, `package`, `plugin`, `target`, `audit`, and `semantic` are allowed only when the directory is the corresponding semantic domain or when the repeated prefix is unavoidable and narrowly excepted.
- Namespace validation must reject path names whose first useful token is implementation history rather than domain responsibility: `internal`, `wave`, `coverage_wave`, `tmp`, `old`, `misc`, `helpers`, `utils`, `common`, `shared`, `lib`, `services`, and typo variants.
- Namespace validation must produce agent-remediating failures that include directory, offending prefix, count, representative paths, why the prefix indicates a missing directory, required repair class, affected claim classes, and whether a typed exception could ever be valid.

Required semantic domain-type naming changes:

- Treat file names, module names, test module names, schema file names, receipt names, fixture ids, check ids, and authority object names as semantic authority surfaces.
- Fail generic source/module names that encode storage status or implementation history instead of domain responsibility. Examples that must fail when used as authority surfaces include `internal_coverage_waveNN_tests`, `internal_claim_tests` when it should be `claim/identity.rs` or `claim/evidence.rs`, `internal_review_tests` when it should be `review/anchors.rs` or `review/materiality.rs`, and typo variants such as `iinternal_*`.
- Require semantic source modules to name the law, domain, authority, boundary, receipt, fixture, product surface, or workflow they govern. Domain directories must carry the repeated concept; filenames inside those directories must drop redundant prefixes.
- Typed exceptions for generated code, local generic algorithms, or external compatibility contracts must be narrow, parsed, package-included, claim-limited, and independently red-fixtured.

Required red, green, and tamper fixtures:

- Add red fixtures proving a top-level `validator/src/internal_coverage_wave99_tests.rs` style file fails namespace law.
- Add red fixtures proving top-level `validator/src/internal_claim_tests.rs`, `validator/src/internal_review_tests.rs`, and `validator/src/internal_schema_tests.rs` style clusters fail when more than two files share the prefix.
- Add red fixtures proving typo variants such as `validator/src/iinternal_coverage_tests.rs` fail.
- Add red fixtures proving a broad source exception for `validator/src/internal*` fails even when it names `plugin-manifest-draft.json` as a contract.
- Add red fixtures proving source exceptions with `applies_to = ["validator/src/*"]`, `["validator/src/internal*"]`, or equivalent broad globs fail.
- Add red fixtures proving a generated/catalog exception cannot be used for hand-authored validator source.
- Add red fixtures proving a package manifest listing cannot satisfy namespace compliance for a non-compliant source path.
- Add red fixtures proving a renamed file that keeps coverage-wave history but moves directories still fails semantic repo-law when the name remains non-semantic.
- Add red fixtures proving namespace errors cannot be hidden by capped reporting, row presence, foundational trace presence, source-obligation presence, coverage pass, line-cap pass, package inventory pass, Product Fitness pass, reviewer approval, or lowered claim ceiling.
- Add green fixtures proving semantically routed validator test directories pass with names such as `validator/src/self_tests/coverage/receipt_authority.rs` and `validator/src/self_tests/claim/evidence_boundaries.rs`.
- Add green fixtures proving generated fixture catalogs can still use repeated prefixes only when a generator/catalog route owns the family and the exception is narrow, typed, and claim-limited.
- Add tamper fixtures proving that widening a narrow namespace exception after receipt generation invalidates the receipt and blocks completion/review/package/readiness/release claims.

Required standards, trace, source-obligation, and claim-ceiling integration:

- Add or tighten agent-standards rows for validator source namespace topology and semantic repo-law enforcement. These may point to Gate 8 and Gate 24, but they must name this concrete source-topology law explicitly and must not rely on generic `namespace-progressive-disclosure` or `semantic-domain-type-naming` row presence alone.
- Add foundational trace entries mapping filesystem-as-agent-interface and scoped-module article requirements to validator source topology enforcement, the new red fixtures, valid fixtures, receipt requirements, package inventory, and claim ceilings.
- Add source-obligation parity entries so this law is not bundled under broad namespace, documentation freshness, architecture, line-cap, coverage, or semantic-domain rows without independent child-law failure proof.
- Add claim-ceiling guards so completion, review, package, readiness, release, product-readiness, CLI self-law, source audit, final packet, and update_goal eligibility all fail while validator source topology violates namespace or semantic repo-law.
- Add feedback-to-rule and historical regression corpus rows for the side-thread signal that found the `internal_*`/`internal_coverage_*` sprawl and the broad exception loophole. Include source artifact, timestamp/session id if available, observed failure, affected surfaces, implemented repair, red fixtures, valid fixtures, validator ids, receipt ids, claim ids, and claim impact.

Required validation:

- Before editing, verify current state live with commands equivalent to:
  - count top-level `validator/src/*.rs`;
  - count top-level `validator/src/internal_*.rs`;
  - count top-level `validator/src/internal_coverage*.rs`;
  - search for `validator/src/iinternal_*.rs`;
  - inspect `docs/namespace-law-exceptions.json` for broad validator source exceptions;
  - inspect `plugin-manifest-draft.json` for stale top-level validator internal paths.
- After repair, run and capture:
  - `cargo fmt --check`;
  - `cargo test --offline`;
  - namespace law proof through the CLI;
  - semantic domain-type naming proof through the CLI;
  - package inventory closure and exactly-once proof;
  - line-cap proof;
  - coverage proof with 100 percent and `uncovered_records = []`;
  - source audit with red fixture report;
  - red fixtures for the new namespace/semantic topology failures;
  - green fixtures for the routed source topology;
  - tamper fixture for widened namespace exception;
  - source/install/cache package evidence after source passes and only after source passes.
- Validation must prove that no top-level `validator/src/internal_*.rs`, `validator/src/internal_coverage*.rs`, `validator/src/iinternal_*.rs`, or equivalent prefix-as-directory source cluster remains accepted by the law.
- Validation must prove that moved files are package-included exactly once, no stale manifest entries remain, review target/archive/package inventory references are current, and source/install/cache digests align after final sync.

Required confidence calculation:

- The final packet and final response must include an explicit confidence value for this repair, but the value must be calculated from evidence rather than asserted. Use all listed scored components:
  - root cause observed directly: 25 points;
  - foundational/source-law alignment: 20 points;
  - direct enforcement path implemented: 20 points;
  - Rust/source topology refactor feasibility validated: 15 points;
  - red/green/tamper proof completeness: 15 points;
  - residual integration risk bounded: 5 points.
- The expected target confidence is at least 99 percent only if the broad exception is removed, source topology is physically repaired, recurrence fails mechanically, red/green/tamper fixtures pass, package inventory is current, coverage remains 100 percent, and source audit passes on the same candidate.
- A validator-only patch, a standards-row-only patch, a claim-ceiling-only patch, a packet-only patch, or a broad exception rewrite without physical source topology repair cannot claim 99 percent confidence. It must report lower confidence and block completion.
- The rationale must explain why the chosen solution was selected over alternatives:
  - Chosen solution: physical source topology repair plus typed exception tightening plus red/green/tamper enforcement.
  - Rejected alternative: keep flat files and add more prose or rows, because that preserves the agent-facing smell.
  - Rejected alternative: keep broad exceptions and rely on package manifest routing, because package inventory cannot justify hand-authored source namespace drift.
  - Rejected alternative: validator-only prefix check without moving files, because the repo would still fail its own product/namespace laws.
  - Rejected alternative: coverage-only or line-cap-only repair, because passing those gates does not make filesystem topology semantic.

### 91. Rust Developer Experience, Runtime Resource Discipline, And Workspace Garbage Collection

This gate is additive to Gates 1-90. It does not replace, narrow, weaken, summarize, defer, or supersede CLI authority, CLI self-law compliance, CLI performance, namespace law, typed-boundary law, coverage law, line-cap law, source/install/cache/app-surface separation, clean-checkout command discovery, one-command bootstrap, package inventory, Product Fitness, Product Cohesion, Product Success, source-obligation parity, foundational traceability, red/green/tamper fixture proof, version bump, final packet correctness, or any update_goal() stop condition.

Harness Ultragoal's Rust developer experience is a governed product surface. It is not "developer preference." A slow, ad hoc, locally magical Rust loop is a product failure and a Harness Ultragoal law failure. The Rust toolchain, Cargo, nextest, llvm-cov, Clippy, rustfmt, dependency/security tools, profiling tools, caches, watchers, linkers, test fixtures, runtime resource management, and cleanup flows may produce observations. They are not claim authority.

The governing doctrine is mandatory:

- Raw tools may produce observations.
- Only the `ultragoal` CLI may convert observations into typed, digest-bound, same-surface receipts and claim ceilings.
- Cargo is the canonical Rust substrate, but Cargo is not claim authority.
- Accelerators are governed infrastructure, not optional suggestions. If a tool materially improves speed, correctness, repeatability, supply-chain safety, memory/resource discipline, cleanup safety, or professional Rust workflow quality, Harness Ultragoal must either adopt it in a declared class or explicitly reject it with evidence.
- Hidden local state, warm caches, watcher state, editor diagnostics, global Cargo configuration, local aliases, untracked scripts, stale target dirs, unbounded logs, abandoned worktrees, partial receipts, and cleanup done outside the CLI cannot support claims.
- Rust has no built-in tracing garbage collector. Harness Ultragoal must use Rust-native ownership, RAII, bounded resources, explicit cleanup, leak detection, and governed cleanup receipts rather than treating "Rust" as automatic memory/resource correctness.

Adopt this Rust governance boundary:

- `rust-toolchain.toml`, `Cargo.lock`, Cargo workspace metadata, `cargo check`, `cargo build`, `cargo fmt`, `cargo clippy`, `cargo test` for doctests/compatibility, `cargo nextest` for standard/release normal test execution, `cargo llvm-cov`, Cargo profiles, feature strategy, and MSRV policy are required Rust substrate surfaces.
- `cargo metadata --format-version=1 --locked`, `cargo tree --workspace --duplicates`, `rustup show active-toolchain`, `rustc --version --verbose`, `cargo --version --verbose`, and installed component inventory are required toolchain/workspace observation inputs.
- `cargo nextest` is required for standard and release test execution after bootstrap. If missing, the CLI must emit a missing-tool failure with bootstrap repair instructions, not silently downgrade to weaker proof. Doctests still require Cargo.
- `cargo llvm-cov` is the canonical Rust coverage proof path for this repo's declared Rust scope. Exact 100 percent coverage with `uncovered_records = []`, candidate digest binding, source digest binding, toolchain binding, and anti-gaming checks remain mandatory.
- `rustfmt` and `clippy` are required. Clippy failures must include lint code, span, suggestion when machine-applicable, affected claim classes, and rerun command.
- `cargo-deny` and `cargo-audit` are required supply-chain/security gates for standard/release proof. `cargo-vet` is required for release/supply-chain maturity when release readiness or external distribution is claimed; it is not required for every fast local repair loop.
- CycloneDX SBOM generation, checksums, signing, and release provenance are required for official release/distribution claims. `cargo-dist` is required only when Harness Ultragoal claims binary release/distribution readiness; it is not source compliance, review readiness, or product success proof by itself.
- Secret scanning with a governed primary scanner such as Gitleaks is required for release/package readiness. Secondary deep scanners such as TruffleHog are governed adapters.
- `serde`, `schemars`, `jsonschema`, `serde_path_to_error`, `clap` derive/value enums, `camino`, `thiserror`, `miette`, and `tracing` are required typed-boundary/diagnostic/observability substrates for the Rust control plane unless a narrower same-law replacement is explicitly adopted and proven.
- `anyhow` is allowed only at binary/application edges. Law-core APIs must expose typed error enums and parse results, not opaque catch-all errors.
- `proptest`, `cargo-fuzz`/libFuzzer, `trybuild`, `insta`, `snapbox` or equivalent CLI snapshot harness, `assert_cmd`, and `tempfile` are required for parser, path, fixture, CLI, compile-fail, diagnostic, and temp-resource proof where the corresponding code surface exists.
- Criterion and Hyperfine are required for performance proof surfaces: Criterion for critical library paths and Hyperfine for CLI command budget proof. Flamegraph, Samply, Instruments, Heaptrack, Valgrind, sanitizers, Miri, Divan, and iai-callgrind are governed adapters for diagnosis or scheduled scoped proof, not universal claim authority.
- `sccache`, Cargo incremental compilation, linker acceleration, Cargo target dirs, nextest recordings, rust-analyzer target dirs, Cargo registry/git caches, Docker/CI caches, plugin caches, install caches, and remote caches are legal only when declared in cache/no-cache receipts. Cache use is legal. Cache concealment is illegal. Warm-cache proof may never support cold/no-cache claims.
- `sccache` is default-on acceleration when available and declared; absence must be reported as acceleration unavailable, not as correctness failure. Remote cache is forbidden unless explicitly declared with endpoint identity redaction, cache key, policy, and claim limitation.
- `lld` is default-on where platform policy permits. `mold` is a governed adapter because it is valuable but platform-dependent. Linker acceleration supports timing/iteration claims only, never correctness/product/release claims by itself.
- `cargo-binstall` is default-on for bootstrap speed only when version/digest policy is satisfied; fallback must be `cargo install --locked`. Tool installation receipts support tool/bootstrap claims only.
- `watchexec` is the default low-level watch substrate for `ultragoal rust watch`; Bacon is default-on or governed adapter for high-quality human/agent Rust feedback if bootstrap can install it safely. Watchers emit feedback events, not completion receipts. `cargo-watch` must not be standardized; if present, it is local-only or rejected for claim support.
- `rust-analyzer` is default-on for editor diagnostics and navigation, but editor state, test lenses, and check-on-save are optional local observations only. They cannot support claims.
- `cargo xtask` is required for bootstrap/control tasks before installed `ultragoal` exists. `just` may exist only as a governed alias layer that delegates to `ultragoal` or `xtask`; it is not authority. Makefiles and unwrapped shell scripts are rejected as authority.
- Nix, mise, devcontainers, cross, cargo-zigbuild, cargo-release, cargo-public-api, cargo-semver-checks, cargo-msrv, cargo-udeps, cargo-geiger, cargo-machete, cargo-outdated, cargo-chef, grcov, and alternate profilers are governed adapters unless this repo declares a stricter same-law requirement. They may not support claims without declared config, version/tool identity, output digests, and claim limits.
- Raw `cargo`/tool output as final proof is rejected. Raw tool commands may be used for investigation only. Final claims must route through `ultragoal` wrappers and receipts.

Required Rust command classes:

- FAST loop: `ultragoal rust fast`.
  - Goal: fastest meaningful local feedback for frequent agent iteration.
  - Required internal checks: quick toolchain verification, changed-scope workspace topology, changed-scope namespace law, changed-scope line-cap law, changed-scope typed-boundary law, `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked --message-format=json`, and focused `cargo nextest` profile when nextest is installed/bootstrap-proven.
  - Allowed acceleration: declared sccache, incremental compilation, platform-safe linker acceleration, and watch-mode routing.
  - Claim support: fast feedback and changed-source structural observations only.
  - Claim exclusions: review ready, release ready, Product Fitness, Product Cohesion, Product Success, package ready, install verified, cache coherent, final packet, and update_goal eligibility.
- STANDARD loop: `ultragoal rust standard`.
  - Goal: serious local proof before claiming a repair.
  - Required internal checks: toolchain receipt, cargo metadata receipt, workspace topology receipt, namespace receipt, line-cap receipt, typed-boundary receipt, `cargo fmt`, `cargo clippy --workspace --all-targets --all-features --locked --message-format=json -- -D warnings`, `cargo check --workspace --all-targets --all-features --locked --message-format=json`, `cargo nextest list`, `cargo nextest run --profile standard`, doctests, required red/green/tamper fixtures for touched validators, current receipt verification, and claim ceiling computation.
  - Claim support: source compliance, typed-boundary compliance, standard test verification, fixture triad verification, and repair-claim candidate only.
  - Claim exclusions: coverage complete, package ready, install verified, runtime product success, release ready, final packet, and update_goal eligibility unless other gates independently pass.
- RELEASE loop: `ultragoal rust release`.
  - Goal: full law proof for the claim requested.
  - Required internal checks: standard loop, exact coverage proof, dependency/security proof, supply-chain proof, feature-matrix proof, performance budget proof, memory/resource proof, package inventory proof, clean install proof, cache separation proof, Product Fitness/Cohesion/Success proofs only when the requested claim requires those product surfaces, GC dry-run proof, current receipt verification, claim ceiling computation for the requested claim, and CLI-built packet when packet claim is requested.
  - Claim support: release, product, final packet, and update_goal claims only when every applicable same-surface receipt is current and same-candidate.
  - Product Success is not a default substitute for every release loop; it is mandatory when a product success, daily-driver, user outcome, product readiness, external-user, marketplace, or release claim depends on product outcome.
- COLD/CLEAN loop: `ultragoal rust clean-proof`.
  - Goal: prove clean-checkout and no hidden local cache dependency.
  - Required behavior: fresh checkout or clean worktree snapshot, isolated `CARGO_HOME`, isolated `CARGO_TARGET_DIR`, `RUSTC_WRAPPER` unset unless explicitly testing an accelerator, `CARGO_INCREMENTAL=0`, no editor/watcher state, no global Cargo config unless inventoried, locked fetch or offline cache verification, standard loop, and optional release subset.
  - Claim support: clean-checkout onboarding, no-hidden-local-magic, and no-cache proof.
- WATCH loop: `ultragoal rust watch`.
  - Goal: continuous feedback.
  - Required behavior: invoke the declared watcher and route to `ultragoal rust fast`.
  - Claim support: watch observations only. Completion/review/release/update_goal claims require subsequent receipt verification and claim ceiling computation.
- MEMORY/RESOURCE loop: `ultragoal rust memory prove`.
  - Goal: bounded memory, bounded queues, bounded file descriptors, cleanup on cancellation/error/panic, no leaked temp resources, child processes reaped, and long-running stability.
  - Required behavior: static resource audit, async-task audit, resource-lifecycle tests, memory-budget performance proof, selected leak checks, long-running scenario with telemetry where applicable, and receipt verification.
- GARBAGE-COLLECTION/CLEANUP loop: `ultragoal gc plan`, `ultragoal gc dry-run`, `ultragoal gc apply`, `ultragoal gc verify`.
  - Goal: identify and remove stale workspace artifacts safely with protected-artifact rules, deletion receipts, and active-claim preservation.
  - Required behavior: classify every artifact, compute protected set, compute stale set, produce deletion plan, dry-run plan, apply only with plan digest, emit deletion receipt, verify protected artifacts remain, verify active claims still have supporting receipts, and verify workspace locks/pids/ports/tempdirs are clean.
  - Deletion without a receipt is artifact destruction, not cleanup.

Required Rust receipt models:

- Every Rust loop receipt must include schema version, receipt id, law ids, command argv, working directory digest, toolchain identity, `rust-toolchain.toml` digest, `Cargo.lock` digest, `Cargo.toml`/workspace metadata digest, source/candidate digest, law graph digest, schema catalog digest, fixture catalog digest when applicable, environment class, OS/arch, env allowlist digest, Cargo home class, target dir class, cache mode, tool observations with tool versions and output digests, proof surface, subject digest, issued/expiry timestamps, result, claim support, claim exclusions, and staleness policy.
- A Rust receipt is stale if any bound source digest, Cargo lock/manifest digest, toolchain file, law registry, schema registry, validator binary, fixture suite, tool version, tool config, feature matrix, proof surface, package digest, install tree digest, runtime config digest, environment equivalence class, or command identity changes outside the allowed policy.
- Cache receipt fields must include cache mode, Cargo incremental state, `RUSTC_WRAPPER`, target dir, Cargo home, sccache stats before/after when used, remote cache endpoint hash or none, cache policy digest, cache hit/miss where available, and timing class.
- Timing classes must distinguish cold no-cache timing, cold declared-cache timing, warm local-cache timing, warm remote-cache timing, editor-warm timing, and CI-cache timing. A warm-cache result may never support a no-cache claim.

Required runtime memory/resource discipline:

- Rust ownership, borrowing, RAII/drop, owned values, local references, `Box<T>` for large/recursive/trait-object ownership, `Arc<T>` for shared concurrent immutable/config state, and `Weak<T>` for back edges are required memory-discipline tools where applicable.
- `Rc<T>` is allowed only in single-threaded internal graphs; cycles require `Weak`.
- `RefCell`, `Mutex`, `RwLock`, `DashMap`, `arc-swap`, crossbeam epoch tools, arenas, object pools, memory-mapped files, and alternate allocators are governed adapters. They require reason, scope, budget, metrics, drop/reset policy, tests, and receipts.
- Tracing garbage-collection crates are rejected for the correctness-critical core. They may not become the memory model for Harness Ultragoal's CLI/control plane.
- Bounded caches are required wherever caching exists. Allowed cache shapes include LRU, TTL, size-bound, entry-count-bound, and digest-addressed immutable caches. Global unbounded maps, static lazy caches without max size, non-canonical path string keys, caches without invalidation, and caches used as proof without digest binding are forbidden.
- Streaming parsers are required for large artifacts unless a size-bound whole-file read is justified and receipt-bound. Whole-file loading of package inventories, coverage reports, logs, session transcripts, source cards, generated packets, or audit outputs must have explicit size bounds or streaming behavior.
- Every long-running task must have owner, cancellation token or shutdown channel, bounded queue, tracked join handle, shutdown timeout, drop cleanup path, panic/error reporting, and memory/queue telemetry.
- Spawn-and-forget tasks, unbounded channels, unbounded join sets, unmanaged background watchers, leaked tempdirs on cancellation, child processes without kill/reap policy, and long-running loops without shutdown proof are forbidden.
- Memory/resource receipts must include scenario, binary digest, runtime config digest, duration, max RSS budget and observed RSS, max open file descriptors, max queue depth, max child processes, max temp bytes, created/removed tempdirs, spawned/reaped child processes, shutdown signal, cancellation delivery, all-tasks-joined status, shutdown duration, leak-check adapters and results, and claim impact.
- Leak detection must exist as scheduled/scoped proof: selected Miri checks, long-running RSS stability scenarios, resource lifecycle tests, tempdir/child-process/file-descriptor cleanup tests, and governed adapters for Valgrind, Heaptrack, sanitizers, and platform profilers where applicable.

Required workspace/artifact/cache garbage collection:

- Harness Ultragoal must implement both runtime resource cleanup and workspace/artifact/cache cleanup. Rust has no built-in workspace garbage collector, and Cargo cache/target cleanup does not satisfy Harness proof hygiene by itself.
- Every non-source workspace artifact must be classified before cleanup. Artifact classes must include Cargo target artifacts, Cargo registry cache, Cargo git cache, sccache entries, nextest recordings, coverage artifacts, validation artifacts, current receipts, stale receipts, final packets, review packets, generated source, generated non-source, package artifacts, install copies, plugin cache copies, worktree lanes, temp dirs, lock files, pid files, port reservations, trace logs, heap profiles, flamegraphs, and performance baselines.
- Unclassified artifacts cannot be deleted by automated cleanup and cannot support claims.
- Protected artifacts include current receipts supporting active claims, current final packets, current failure receipts needed for repair, release SBOM/provenance/signature/checksum artifacts, declared performance baselines, package/install/cache inventory receipts, Product Fitness/Cohesion/Success evidence, law/schema/source-obligation/standards registries, fixtures, source files, `Cargo.lock`, `rust-toolchain.toml`, and active review target/archive evidence.
- Protected artifacts may not be deleted unless replacement proof exists or the active goal is explicitly retired with typed disposition.
- GC commands must be plan/dry-run/apply/verify. `apply` must reference the plan digest. Post-delete verification must prove active claims remain supported, protected artifacts are present, locks/pids/ports are clean, and workspace artifact state matches the deletion receipt.
- Cleanup after failed or interrupted agents must inspect locks, pids, ports, child process records, temp dirs, partial receipts, partial package/install/cache copies, watcher state, and abandoned worktree lanes. Blind `rm -rf` cleanup is forbidden.

Required Rust workspace and topology direction:

- The repo must move toward a workspace/module shape that mirrors authority boundaries. Acceptable crate families include CLI, core/domain types, law registry/execution, namespace, receipt, claim ceiling, fixture execution, package inventory, install verification, cache verification, product proof, diagnostics, observability, plugin integration, and xtask/bootstrap.
- The CLI crate may parse and dispatch but must not hide law logic. Law-core crates must expose typed APIs and typed errors.
- The Rust module tree must remain maximally factored with no residual prefix encoding, no `utils`/`common`/`misc`/`shared` buckets, no coverage-wave/history names, and no unclassified generated/test/cache paths.
- Suggested line caps are governed defaults, not excuses: production Rust source file 300 non-comment LOC, test source file 400, fixture manifest 200, CLI command module 250, validator module 300, schema file 300 unless generated with receipt. The active repo line-cap law may be stricter and prevails.

Required migration path for the current repo:

- Phase 0: inventory with Cargo metadata, dependency tree, workspace topology report, namespace report, module-tree violations, generic bucket names, line-cap violations, toolchain gaps, dependency risks, test classes, and artifact directories.
- Phase 1: toolchain/bootstrap with `rust-toolchain.toml`, `.cargo/config.toml`, Cargo lock policy, xtask bootstrap, `.ultragoal/tool-inventory.toml`, and governed aliases only.
- Phase 2: command routing through `ultragoal rust fast`, `standard`, `coverage prove`, and `dependency audit`. Raw command output remains observation only.
- Phase 3: namespace and line-cap repair into maximally factored module tree.
- Phase 4: typed-boundary repair from `serde_json::Value` authority, stringly ids, open enum strings, and `anyhow` in law-core APIs into newtypes, enums, schemas, parsers, and typed errors.
- Phase 5: fixture triads for every validator.
- Phase 6: exact coverage and anti-gaming with declared source classes and no unclassified exclusions.
- Phase 7: package/install/cache separation.
- Phase 8: Product Fitness, Product Cohesion, and Product Success proof where applicable.
- Phase 9: release and GC with `ultragoal rust release`, `ultragoal gc plan`, `dry-run`, `apply`, `verify`, and CLI-built final packet.

Required standards, trace, source-obligation, and claim-ceiling integration:

- Add or tighten agent-standards rows for Rust Developer Experience, Rust toolchain/substrate authority, Rust command loops, Rust cache/no-cache honesty, Rust memory/resource discipline, and workspace/artifact garbage collection.
- Add foundational trace entries mapping AI Is Forcing Us To Write Good Code requirements for fast guardrails, fast feedback loops, clean checkout, concurrent isolated environments, and filesystem-as-interface; Parse, Don't Validate requirements for typed tool observations and receipt authority; Harness Engineering requirements for repo-local system of record, mechanical invariants, observability, and entropy cleanup; Symphony requirements for isolated workspaces and long-running orchestration; and ExecPlan requirements for restartable documented proof.
- Add source-obligation parity entries so Rust DevX, memory/resource discipline, and GC cleanup are not bundled under generic CLI performance, runtime feasibility, cleanup, or namespace rows without independent child-law failure proof.
- Add schema catalog entries, validator check ids, red fixtures, green fixtures or valid receipts, tamper/stale/wrong-surface fixtures, package inventory entries, receipt schemas, claim-ceiling guards, and final packet fields for each new law surface.
- Claim-ceiling guards must block completion, review, package, readiness, release, Product Fitness/Cohesion/Success, CLI self-law, source audit, final packet, and update_goal eligibility when applicable Rust DevX, Rust cache/no-cache, Rust memory/resource, or GC cleanup proof is missing, stale, wrong-surface, hidden-cache-dependent, unbounded, destructive, or non-CLI-built.

Required red, green, and tamper fixtures:

- Red fixtures proving raw Cargo/nextest/llvm-cov/deny/audit output cannot satisfy claims without `ultragoal` receipt conversion.
- Red fixtures proving `cargo test` pass cannot satisfy coverage, namespace, package, install, runtime, product, release, or update_goal claims.
- Red fixtures proving hidden global `RUSTC_WRAPPER`, hidden `CARGO_TARGET_DIR`, hidden Cargo config, editor-only green status, watcher-only pass, and warm-cache timing substituted for no-cache proof all fail.
- Red fixtures proving missing `rust-toolchain.toml`, missing Cargo lock policy, missing tool inventory, missing machine-readable output, missing cache mode, stale toolchain receipt, stale Cargo metadata, stale feature matrix, and changed tool version invalidate Rust loop receipts.
- Red fixtures proving `cargo-nextest` absence is a missing-tool/bootstrap failure for standard/release proof rather than silent fallback.
- Red fixtures proving `cargo-watch`, Makefile authority, unwrapped shell script authority, and raw local aliases cannot support claim pathways.
- Red fixtures proving `anyhow`/freeform error authority in law-core APIs, unparsed `serde_json::Value` authority, stringly law ids, open proof surfaces, and nullable/catch-all authority fail typed-boundary law.
- Red fixtures proving unbounded caches, unbounded queues, spawn-and-forget tasks, child process without kill/reap policy, temp resource without owner/drop policy, reference cycle without `Weak`, large unbounded whole-file load, and long-running loop without shutdown proof all fail memory/resource discipline.
- Red fixtures proving tracing GC crate adoption as core memory model fails.
- Red fixtures proving deletion without plan, deletion without dry-run, apply with wrong plan digest, protected artifact deletion, active-claim receipt deletion, unclassified artifact deletion, blind `rm -rf`, interrupted-agent cleanup without lock/pid/port/tempdir inspection, and deletion receipt without post-verify all fail GC law.
- Green fixtures/valid receipts proving canonical fast, standard, release, clean-proof, watch observation, memory/resource, and GC dry-run/apply/verify flows can pass on a compliant minimal fixture.
- Tamper fixtures proving changed tool version, changed tool config, changed cache mode, changed source digest, changed Cargo lock digest, changed law graph digest, changed plan digest, removed protected artifact, or altered deletion receipt invalidates proof and blocks claims.

Required validation:

- Run `ultragoal rust toolchain verify --emit-receipt` or equivalent once implemented.
- Run `ultragoal rust fast`, `ultragoal rust standard`, `ultragoal rust coverage prove --exact`, `ultragoal rust dependency audit`, `ultragoal rust performance prove`, `ultragoal rust memory prove`, `ultragoal rust clean-proof`, `ultragoal rust workspace topology check`, and `ultragoal gc plan/dry-run/apply/verify` as applicable to the candidate.
- Run focused red fixtures for raw-tool substitution, cache/no-cache dishonesty, missing toolchain/tool inventory, hidden local state, memory/resource leaks, and unsafe cleanup.
- Run source audit and red fixture report after implementing these surfaces.
- Do not refresh install/cache or launch reviewers until source-level Rust DevX/memory/GC enforcement passes on the same candidate.

Required confidence calculation:

- The final packet and final response must include calculated confidence for Gate 91 using this 100-point model:
  - doctrine/root cause alignment observed directly: 15 points;
  - Rust ecosystem maturity and adoption for required substrate: 15 points;
  - direct CLI receipt enforcement path implemented: 20 points;
  - cache/no-cache and clean-checkout honesty proven: 15 points;
  - memory/resource and long-running cleanup proof implemented: 15 points;
  - workspace/artifact GC protected-deletion proof implemented: 10 points;
  - red/green/tamper fixture completeness: 10 points.
- The expected target confidence is at least 96 percent only if the command loops, cache/no-cache receipts, memory/resource receipts, GC plan/dry-run/apply/verify receipts, standards/source-obligation/foundational trace entries, red/green/tamper fixtures, package inventory, source audit, and coverage proof all pass on the same candidate.
- A raw-tool-only patch, Cargo-only patch, performance-prose patch, watcher-only patch, cache-only speed claim, cleanup script without deletion receipt, memory-prose-only patch, or claim-ceiling-only patch cannot claim high confidence and must block completion.

Validation requirements:

Run and capture exact commands/output for:

- `cargo fmt --check`
- `cargo test --offline`
- full source audit with receipt and red fixture report
- full installed plugin audit
- full cache package audit
- coverage command proving 100%
- Rust toolchain/substrate receipt
- Rust fast loop receipt
- Rust standard loop receipt
- Rust release loop receipt for requested release/package/product claims
- Rust clean-proof/no-hidden-local-magic receipt
- Rust cache/no-cache honesty receipt
- Rust dependency/security/supply-chain receipt
- Rust performance budget receipt
- Rust memory/resource discipline receipt
- workspace/artifact/cache GC plan, dry-run, apply, verify receipts where cleanup is performed
- Rust DevX red, green, and tamper fixtures
- namespace law proof
- namespace red fixtures
- validator source namespace topology proof
- semantic repo-law source topology proof
- validator source namespace red, green, and tamper fixtures
- line-cap command
- runtime-tool identity proof and red fixtures
- product live-surface receipt proof and red fixtures
- transcript-quality receipt proof and red fixtures
- clean-checkout command-discovery proof and red fixtures
- restartable ExecPlan validator proof and red fixtures
- source-card freshness proof
- memory/wiki/Chronicle context-only proof and red fixtures
- Product Fitness proof
- Product Fitness review-team ownership proof
- Product Fitness review-round red fixtures
- standards enforcement proof
- foundational-law trace proof
- architecture dependency topology proof and red fixtures
- Quality Score/taste gate proof and red fixtures
- feedback-to-rule promotion receipt and red fixtures
- autonomy-loop proof receipts and red fixtures
- orchestrator state-machine proof and red fixtures
- scheduler/runner/tracker-boundary proof and red fixtures
- subagent/custom-agent sandbox and approval-inheritance proof and red fixtures
- skill progressive-disclosure metadata proof and red fixtures
- plugin install-surface metadata/cache/enable-state proof and red fixtures
- ExecPlan no-handback/prototype promotion-discard proof and red fixtures
- semantic domain-type naming proof and red fixtures
- agent-remediating validator failure-message proof and red fixtures
- third-party dependency legibility/typed-adapter proof and red fixtures
- repo knowledge index/core-beliefs proof and red fixtures
- workflow template parsing/rendering/reload proof and red fixtures
- workspace command confinement/lifecycle cleanup proof and red fixtures
- plugin bundled component graph and hook/app/MCP safety proof and red fixtures
- instruction precedence/nested AGENTS routing proof and red fixtures
- ExecPlan plain-language/expected-output/interface-completeness proof and red fixtures
- guardrail speed/isolation/cache-honesty proof and red fixtures
- secret/token boundary proof and red fixtures
- generated/proof artifact provenance and anti-fabrication proof and red fixtures
- review feedback disposition and same-round satisfaction proof and red fixtures
- behavior-example coverage and coverage anti-gaming proof and red fixtures
- one-command fresh environment bootstrap/concurrency proof and red fixtures
- agent-queryable observability proof and red fixtures
- subagent orchestration explicitness/token-model-cost/reconciliation proof and red fixtures
- skill catalog context-budget/omission-warning proof and red fixtures
- distribution and sharing-surface claim-separation proof and red fixtures
- total authority types and impossible-state elimination proof and red fixtures
- CLI self-law compliance/self-hosting proof and red fixtures
- CLI performance/latency/speed/iteration-fitness proof and red fixtures
- agent-authored source/tooling/docs provenance proof and red fixtures
- stable identifier/normalization/collision proof and red fixtures
- agent session telemetry/token/rate-limit proof and red fixtures
- config precedence/default/env-indirection proof and red fixtures
- fresh-init versus retrofit mode proof and red fixtures
- issue/tracker lifecycle/eligibility/terminal-state proof and red fixtures
- targeted refactor/debt-removal/standards-gardener cadence proof and red fixtures
- plugin flow graph/package dependency closure/plugin product journey proof and red fixtures
- portable non-prescriptive adapter proof and red fixtures
- derived authority recomputation/named-authority fallback proof and red fixtures
- offline schema catalog/resolver portability proof and red fixtures
- batch fan-out/custom-agent job discipline proof and red fixtures
- raw-private artifact handling/category-only evidence proof and red fixtures
- active setup-to-idle orchestration/thread-bound heartbeat proof and red fixtures
- connector capability discovery/same-surface capability proof and red fixtures
- target-repo audit capability/target-scope support proof and red fixtures
- trust-boundary abuse-path/failure-path coverage proof and red fixtures
- source-obligation parity/anti-bundling proof and red fixtures
- human-audit disposition decomposition/judgment-only claim-blocking proof and red fixtures
- capability-gap extraction/harness-capability promotion proof and red fixtures
- goal-contract amendment authority/closed-required-claim-id proof and red fixtures
- forward-only state transition/silent-reopen prevention proof and red fixtures
- initiation-time Product Success Contract authority proof and red fixtures
- product success goal/lane/ExecPlan binding proof and red fixtures
- product success lineage/amendment/closed-claim-id proof and red fixtures
- product proof joins/substitution-blocking proof and red/green fixtures
- Product Success Contract packet/review-team/skill-routing proof and red fixtures
- product-success inspiration-source provenance/disposition proof and red fixtures
- product strategy/positioning/research/eval pre-lane proof and red fixtures
- template-generation governance/Template Creator boundary proof and red fixtures
- value/adoption/continuance evidence hierarchy proof and red fixtures
- current product discovery/audit/quality-in-use evidence proof and red fixtures
- product-success lifecycle transition/no-afterthought proof and red fixtures
- validator-theater/miswire resistance proof and red/green/stale/wrong-surface fixtures
- green-path adequacy and satisfiable strictness proof
- clean-room rebuild/author-memory independence proof
- historical regression corpus proof from session logs, Chronicle, reviewers, and side-thread signals
- cross-artifact consistency solver/authority graph closure proof
- authority exhaustiveness/closed-enum/impossible-state elimination proof
- non-E2E claim ceiling and confidence-bound proof
- adversarial packet tampering/forged-proof rejection proof
- runtime feasibility/cost/strict-gate usability proof
- schema evolution/receipt migration/stale-version invalidation proof
- failure remediation quality/agent-actionable validator output proof
- review disagreement/override/judgment-boundary governance proof
- full local observability stack, CLI queryability, telemetry binding, redaction, boundedness, and non-opaque failure proof
- source/install/cache digest comparison
- review-target receipt regeneration
- candidate archive receipt regeneration
- final packet/successor packet validation
