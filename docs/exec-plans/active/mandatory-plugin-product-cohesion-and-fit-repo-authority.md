# ExecPlan: Mandatory Plugin Product Cohesion And Fit-Repo Authority

Contract status: REQUIRED by
`harness-ultragoal-plans-and-orchestrator-automation-hardening.md`. This file is
not advisory. It is incorporated by reference into the active goal contract and
every initialized or retrofitted Harness Ultragoal repo that inherits the plugin.

## Purpose

Force Harness Ultragoal to behave as one strict product instead of a collection
of skills, templates, validators, reviewers, and memories.

A user invokes the plugin. The agent follows the plugin's mandatory entry
contract. The repo becomes harnessed through typed setup, deterministic gates,
receipts, standards rows, validator proof, package/cache alignment, and claim
ceilings. The user must not have to remember the correct skill sequence, remind
the agent to install enforcement surfaces, remind the agent to run checks, or
remind the agent that package/static proof is not live product proof.

## Non-Negotiable Result

Every repo initialized or retrofitted by Harness Ultragoal must pass through one
mandatory product entry contract:

```text
harness-ultragoal:fit-repo
```

No initialized or retrofitted repo can claim:

- harnessed
- initialized
- retrofitted
- agent-first
- standards-compliant
- coverage-enforced
- ultragoal-ready
- product-cohesive
- ready for material review
- ready for phase advancement

unless `fit-repo` emits a schema-valid receipt proving the required setup
surfaces exist, the required validators ran, every missing surface is blocked or
backlogged with affected claims withheld, and the current package/source/cache
candidate is named.

There is no agent discretion. There is no "close enough." There is no "I read the
docs, so the repo is harnessed." There is no reviewer override. There is no
prose-only completion path.

## Mandatory Fit-Repo Entry Contract

The plugin must provide one visible top-level entrypoint for fit-repo activation. That
entrypoint must run this sequence:

1. Discover repo root, package root, installed plugin root, cache root, and app
   registry exposure when available.
2. Classify the target as `fresh_repo`, `retrofit_repo`, or
   `blocked_unclassified_repo`.
3. Classify runtime surfaces: `none`, `cli`, `service`, `workflow_engine`,
   `local_app`, `browser_app`, `dashboard`, `run_console`, `plugin`, `package`,
   `marketplace`, and `external_integration`.
4. Classify product surfaces: `none`, `developer_tool`, `operator_control`,
   `consumer_user_facing`, `internal_user_facing`, and
   `ambiguous_requires_blocker`.
5. Install or verify all required template surfaces.
6. Install or verify deterministic standards enforcement.
7. Install or verify coverage authority and coverage scope authority.
8. Install or verify Codex worktree environment setup.
9. Install or verify ultragoal bundle surfaces when the goal is multi-lane or
   claim-bound.
10. Install or verify orchestrator automation template when the goal will use
    multi-lane Ultragoal orchestration.
11. Run the fast setup gate.
12. Run the target-repo setup validator.
13. Emit a fit-repo receipt.
14. Update claim ceiling.

If any step cannot run, the receipt must record a blocker with owner, reason,
affected claim ids, required repair, and claim ceiling. Silent omission fails.

## Mandatory Fit-Repo Receipt

Every fit-repo run must emit:

```text
validation_artifacts/harness/fit-repo-receipt.json
```

The receipt must bind to:

- repo root;
- git commit or package digest;
- plugin source path;
- installed plugin path when present;
- cache package path when present;
- app registry exposure status when available;
- plugin version;
- entrypoint contract id and version;
- target classification;
- runtime surface classification;
- product surface classification;
- templates installed or verified;
- scripts installed or verified;
- standards rows installed or verified;
- coverage manifest and coverage receipt surfaces;
- worktree environment surfaces;
- ultragoal bundle surfaces;
- orchestrator automation surfaces;
- checks run with command, exit code, stdout/stderr artifact path, and digest;
- blockers;
- withheld claims;
- claim ceiling;
- generated_at timestamp;
- producer actor id;
- receipt digest.

The validator must reject a fit-repo receipt when any required field is missing,
stale, mismatched, hand-written without tool provenance, or detached from the
current target repo.

Required error codes:

- `fit_repo_receipt_missing`
- `fit_repo_receipt_malformed`
- `fit_repo_receipt_stale`
- `fit_repo_receipt_wrong_repo`
- `fit_repo_receipt_wrong_plugin_version`
- `fit_repo_receipt_missing_check_result`
- `fit_repo_receipt_unclassified_target`
- `fit_repo_receipt_unclassified_runtime_surface`
- `fit_repo_receipt_unclassified_product_surface`
- `fit_repo_receipt_claim_ceiling_missing`
- `fit_repo_receipt_blocker_without_owner`
- `fit_repo_receipt_digest_mismatch`

## Mandatory Init Protocol

Fresh repo initialization must not finish until the fit-repo receipt proves:

- `AGENTS.md` routes to all required law surfaces;
- `AGENT_STANDARDS.md` is a router, not a mega-document;
- `agent-standards/enforcement.json` exists;
- `agent-standards/enforcement.tsv` exists;
- `agent-standards/enforcement-audit.tsv` exists;
- `scripts/check-agent-standards` exists and runs;
- `PLANS.md` is stable ExecPlan law;
- active project state is absent from `PLANS.md`;
- root operational docs exist or explicitly state the absent surface and trigger;
- `docs/exec-plans/active/` and `docs/exec-plans/completed/` exist;
- `VERIFICATION_BACKLOG.json` exists;
- `COMPLETION_MANIFEST.json` exists;
- `.harness/coverage-manifest.json` exists;
- `.harness/coverage-command` exists;
- `scripts/check` exists and calls standards before coverage;
- `scripts/check-coverage-fast` exists;
- `scripts/check-coverage-full` exists;
- `.codex/setup-worktree-env.sh` exists;
- `.codex/environments/environment.toml` exists;
- `.gitignore` ignores `.codex-worktree/`;
- `validation_artifacts/` exists;
- the setup gate ran once;
- every blocker is recorded with claim ceiling impact.

If any required surface is missing, initialization fails. If the repo has no
coverage command yet, initialization records a setup blocker and withholds
completion/readiness claims. It does not call the repo harnessed.

## Mandatory Retrofit Protocol

Retrofit must not finish until the fit-repo receipt proves:

- current repo commands, build surface, test surface, coverage surface, runtime
  surface, package surface, and product surface are classified;
- existing instructions and docs are inventoried;
- conflicting or stale guidance is either removed or routed to an owner/blocker;
- purposeless and archive-only files are removed from active repo scope;
- every retained file has an operational purpose;
- active project state is moved out of `PLANS.md`;
- existing workflows are preserved unless a named blocker requires repair;
- missing standards enforcement surfaces are installed or blocked;
- missing coverage authority surfaces are installed or blocked;
- missing worktree environment surfaces are installed or blocked;
- missing runtime/product proof surfaces are installed or blocked when the repo
  owns runtime/product claims;
- `scripts/check` runs or emits a setup blocker;
- every unmechanized standard row is classified as blocked/backlogged with
  affected claims withheld;
- retrofit receipt names every withheld claim.

Retrofit must fail when it cannot classify repo-owned source, runtime surfaces,
product surfaces, or proof surfaces. It must not generate a permissive manifest.
It must not declare success from documentation edits alone.

## Mandatory Plugin Flow Graph

The plugin must include a machine-readable flow graph:

```text
docs/plugin-cohesion-manifest.json
```

The flow graph must declare:

- every skill;
- every authorable template;
- every setup-required script;
- every setup-required schema;
- every setup-required fixture group;
- every setup-required receipt;
- every validator check id;
- every standards row id;
- every custom agent used for material review;
- every package/cache/install surface;
- every required edge between those surfaces;
- entrypoint that owns each flow;
- completion receipt for each flow;
- claim ceiling for each flow.

The validator must reject the plugin when a setup-required file exists in source
but is absent from the plugin manifest, cache package, or installed package
surface being claimed.

Required error codes:

- `plugin_flow_manifest_missing`
- `plugin_flow_manifest_malformed`
- `plugin_flow_entrypoint_missing`
- `plugin_flow_required_edge_missing`
- `plugin_flow_setup_file_not_packaged`
- `plugin_flow_skill_missing_template_dependency`
- `plugin_flow_script_references_unshipped_file`
- `plugin_flow_validator_references_unshipped_fixture`
- `plugin_flow_manifest_cache_mismatch`
- `plugin_flow_claim_surface_unproven`

## Mandatory Package Dependency Closure

Every shipped skill, script, template, schema, fixture, custom agent, and prompt
must have all local references closed inside the claimed package surface.

The validator must walk:

- skill Markdown references;
- template references;
- script references;
- schema references;
- fixture references;
- manifest paths;
- package/cache/install paths;
- custom agent prompt file references;
- automation prompt references.

Any reference to a missing local file fails the package. Any file required by a
setup script but missing from the authorable template list fails the package.

## Mandatory Source/Install/Cache/App Registry Alignment

The plugin must distinguish these proof surfaces:

- source repo;
- installed plugin root;
- cache package root;
- app custom-agent registry;
- app plugin registry;
- marketplace/public listing.

A source check cannot prove installed plugin availability. An installed package
check cannot prove app registry exposure. App registry exposure cannot prove
marketplace publication.

Every claim must name the exact surface it proves. The validator must fail any
claim that substitutes one surface for another.

Required error codes:

- `plugin_source_used_as_installed_proof`
- `plugin_installed_used_as_cache_proof`
- `plugin_cache_used_as_app_registry_proof`
- `plugin_registry_used_as_marketplace_proof`
- `plugin_surface_claim_missing_receipt`
- `plugin_surface_receipt_digest_mismatch`

## Mandatory Plugin Product Journey Receipt

The plugin itself must be treated as a product surface. It must produce:

```text
validation_artifacts/harness/plugin-product-journey-receipt.json
```

The receipt must prove the agent journey:

1. Invoke the plugin entrypoint.
2. Discover whether the target is fresh or retrofit.
3. Install or verify required setup surfaces.
4. Run standards enforcement.
5. Run coverage authority.
6. Run worktree environment setup.
7. Run target-repo setup validation.
8. Emit fit-repo receipt.
9. Produce claim ceiling.
10. Refuse unsupported live/install/marketplace/runtime claims.

The product journey receipt must include command evidence, artifact digests,
error-path evidence, and claim ceiling. It must not rely on chat memory.

## Mandatory Standards-Gardener Promotion

Every severe blocker, repeated friction, reviewer finding, user correction, setup
failure, package/cache drift, app-registry drift, stale-doc issue, worktree setup
failure, coverage-theater attempt, or proof-surface substitution must create a
standards-gardener candidate row before completion.

Each candidate row must be resolved into exactly one state:

- `mechanized`
- `blocked`
- `backlogged`
- `rejected_with_reason`

Unknown status fails. Reviewer agreement does not close the row. Chat memory does
not close the row. A row with affected claims must withhold those claims until
mechanized or explicitly blocked/backlogged.

Required error codes:

- `standards_gardener_candidate_missing`
- `standards_gardener_candidate_unclassified`
- `standards_gardener_repeated_friction_not_promoted`
- `standards_gardener_reviewer_finding_not_dispositioned`
- `standards_gardener_claim_not_withheld`

## Mandatory Orchestrator Lifecycle

For multi-lane Ultragoal work, the orchestrator lifecycle is mandatory:

1. Bind active setup goal.
2. Prepare repo.
3. Write and validate macro-lane ExecPlans.
4. Create or verify branches.
5. Create or verify Codex app worktree threads.
6. Launch first-wave lane owners.
7. Install or verify thread-bound heartbeat automation.
8. Emit transition receipt.
9. Go idle.
10. Wake by automation tick and inspect evidence cursors.
11. Steer, merge, verify, launch unblocked lanes, close, or escalate.

The orchestrator must not claim setup complete before the transition receipt
exists. The automation must be thread-bound. Detached reminder automation fails.

## Mandatory Red Fixtures

The plugin must include red fixtures proving rejection for:

- missing `fit-repo` entrypoint;
- fit-repo receipt missing;
- fit-repo receipt stale;
- fit-repo receipt wrong repo;
- init claim without fit-repo receipt;
- retrofit claim without fit-repo receipt;
- initialized repo missing standards enforcement surface;
- initialized repo missing coverage authority surface;
- initialized repo missing worktree environment surface;
- retrofit with unclassified existing source;
- retrofit with permissive generated coverage manifest;
- setup script references file not listed in plugin manifest;
- source file exists but is absent from shipped package;
- installed/cache/app-registry proof substitution;
- plugin flow graph missing required edge;
- product journey receipt missing;
- standards-gardener candidate missing after severe blocker;
- reviewer finding not promoted or rejected with reason;
- multi-lane ultragoal setup complete without transition receipt;
- detached automation substituted for thread-bound heartbeat.

Each red fixture must fail for the intended error. Wrong-error failure does not
count.

## Mandatory Green Fixtures

The plugin must include green fixtures proving acceptance for:

- fresh repo fit-repo receipt with all required surfaces;
- retrofit fit-repo receipt with classified existing surfaces and blockers;
- package dependency closure with all setup files shipped;
- source/install/cache alignment at package/static scope;
- app-registry unavailable with claim ceiling withheld;
- standards-gardener severe blocker promoted to mechanized row;
- multi-lane ultragoal transition receipt with thread-bound heartbeat;
- plugin product journey receipt at package/static/fixture scope.

## Mandatory Standards Enforcement

The standards surface must include `plugin-product-cohesion-authority`.

The row must be mechanized. It must name:

- fit-repo entry contract;
- fit-repo receipt schema;
- plugin flow graph validator;
- package dependency closure validator;
- source/install/cache/app-registry surface validator;
- plugin product journey receipt;
- standards-gardener promotion gate;
- orchestrator lifecycle gate;
- red fixtures;
- green fixtures;
- repair action.

If the row is missing, non-mechanized, stale, unclassified, or reviewer-memory
based, the standards gate fails. Reviewer approval cannot satisfy this row.

## Mandatory Verification

Run and record:

```bash
bash -n templates/scripts/check \
  templates/scripts/check-agent-standards \
  templates/scripts/check-coverage-fast \
  templates/scripts/check-coverage-full

cargo fmt --check
cargo test --offline

cargo run --offline -- --root . audit \
  --receipt validation_artifacts/ultragoal-audit/validator-receipt.json \
  --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json
```

Also prove:

- fit-repo red fixtures fail for intended errors;
- fit-repo green fixtures pass;
- package dependency closure rejects missing setup dependencies;
- plugin flow graph rejects missing skill/template/script/schema edges;
- initialized target fixtures inherit fit-repo enforcement;
- retrofit target fixtures inherit fit-repo enforcement;
- source/install/cache proof surfaces remain separated;
- plugin product journey receipt exists and is digest-bound;
- standards row `plugin-product-cohesion-authority` is mechanized;
- material reviewer packet includes this ExecPlan.

## Completion Standard

This ExecPlan is complete only when deterministic proof shows:

- the plugin has one mandatory fit-repo entrypoint;
- init cannot complete without fit-repo receipt;
- retrofit cannot complete without fit-repo receipt;
- source/install/cache/app-registry surfaces cannot be substituted;
- plugin flow graph is schema-valid and validator-enforced;
- package dependency closure is validator-enforced;
- standards-gardener promotion is validator-enforced;
- orchestrator lifecycle is validator-enforced for multi-lane goals;
- plugin product journey receipt exists;
- red fixtures pin every bypass;
- green fixtures prove valid flows;
- fresh validator receipt proves the current candidate;
- package/cache/install surfaces point to the same candidate when claimed;
- reviewer team signs off after deterministic proof.

## Claim Ceiling

Before all mandatory verification passes: plugin product cohesion authority is
incomplete.

After package/static/fixture verification passes: plugin package/static/fixture
proof only.

This does not prove live Codex app registry visibility, marketplace visibility,
launcher behavior, runtime behavior, or arbitrary downstream repo quality. Each
surface requires fresh same-surface receipts.
