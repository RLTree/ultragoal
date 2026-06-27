# ExecPlan: Mandatory Coverage Scope Authority And Anti-Theater Enforcement

Contract status: REQUIRED by `harness-ultragoal-plans-and-orchestrator-automation-hardening.md`. This file is not advisory. It is incorporated by reference into the active goal contract and every initialized or retrofitted Harness Ultragoal repo that inherits the plugin.

## Mandatory Coverage Scope Authority And Anti-Theater Enforcement

This ExecPlan defines mandatory coverage scope authority and anti-theater
enforcement. It is separate from the coverage-percentage enforcement plan. That
plan defines what counts as 100% coverage. This plan prevents agents from gaming
the scope, freshness, source binding, or proof authority behind that coverage
claim.

Coverage is not agent-negotiated. The repo contract defines the required
surface. The validator enforces the required surface. Agents execute the
contract or withhold the claim.

#### Purpose

Eliminate coverage theater.

A repo must not pass coverage by measuring a narrow target, omitting changed
files, excluding repo-owned code, using stale receipts, copying receipts across
workspaces, parsing prose, or treating a fast/smoke gate as completion proof.

This plan forces coverage scope, source discovery, changed-file coupling,
receipt freshness, and coverage-policy mutation into deterministic enforcement.

#### Non-Negotiable Result

Every repo initialized or retrofitted by Harness Ultragoal must contain a
coverage scope authority.

The coverage scope authority is mandatory.

The agent does not decide the strictness.
The agent does not choose a convenient target.
The agent does not drop dimensions.
The agent does not downgrade policy.
The agent does not substitute fast checks.
The agent does not hand-write proof.
The agent does not copy proof from another workspace.

The repo contract dictates the required coverage surface. The validator blocks
every unsupported claim.

#### Mandatory Coverage Scope Authority

Every Harness Ultragoal repo must contain:

```text
.harness/coverage-manifest.json
```

The coverage manifest is the source of truth for coverage scope.
The manifest must classify every repo file that can affect behavior, proof,
packaging, runtime, product state, or claim authority.
A file is repo-owned by default.
A file stops being repo-owned only when the manifest classifies it as
generated, vendor, or external with reviewed rationale.
Unclassified source files fail the gate.
Repo-owned source files missing from coverage target paths fail the gate.
Changed repo-owned source files missing from fresh coverage receipts fail the
gate.

#### Mandatory Coverage Manifest Schema

The manifest must include:

- schema id
- manifest version
- repo root digest
- generated_at timestamp
- owner
- policy
- coverage command id
- coverage command path
- coverage receipt output path
- source discovery rules
- repo-owned source roots
- generated roots
- vendor roots
- external roots
- required measured dimensions per root
- required target paths
- exclusion list
- changed-file coupling policy
- claim ceiling when incomplete

The validator must reject the manifest when any required field is missing.

#### Mandatory Manifest File Classification

Every matched file must land in exactly one classification:

- repo_owned
- generated
- vendor
- external

No file can be unclassified.
No file can have two classifications.
No repo-owned source can be reclassified as generated, vendor, or external
without a reviewed manifest row.
If the validator cannot classify a file, it fails with
`coverage_source_unclassified`.

#### Mandatory Repo Walk

The validator must walk the repository before accepting coverage proof.
The walk must inspect all non-ignored files under the repo root.
The walk must compare discovered files against `.harness/coverage-manifest.json`.
The walk must fail when:

- repo-owned source is missing from the manifest;
- source file classification is ambiguous;
- repo-owned source is absent from coverage target paths;
- generated/vendor/external classification lacks rationale;
- generated/vendor/external classification is unreviewed;
- generated/vendor/external classification tries to count as covered;
- ignored local/cache/build files are being counted as coverage targets;
- coverage target paths reference nonexistent files.

Required error codes:

- `coverage_source_unclassified`
- `coverage_source_missing_from_manifest`
- `coverage_source_classification_ambiguous`
- `coverage_manifest_target_gap`
- `coverage_manifest_exclusion_mismatch`
- `coverage_target_path_nonexistent`
- `coverage_target_path_ignored_local_state`

#### Mandatory Changed-File Coupling

Every lane must bind coverage to the changed repo-owned files.

For every changed repo-owned file, the validator must prove one of these states:

- the file appears in the current coverage target paths;
- the file appears in the current coverage receipt;
- the file is excluded by a reviewed manifest exclusion with
  `counts_as_covered = false`;
- the claim is withheld with a blocker that names the file.

There is no agent discretion.

If a lane changes repo-owned source and the changed file is absent from coverage
proof, the lane cannot claim ready, complete, done, release-ready, or material
sign-off.

Required error codes:

- `coverage_changed_file_not_measured`
- `coverage_changed_file_without_receipt`
- `coverage_changed_file_missing_from_manifest`
- `coverage_changed_file_claim_not_withheld`

#### Mandatory Behavioral Coverage Mapping

Coverage dimensions must match the behavior class.
The manifest must map every repo-owned surface to required dimensions.

Required mappings:

- CLI/tooling work requires command behavior coverage.
- Library/API work requires function or entrypoint coverage.
- Branching/error behavior requires branch coverage.
- Product/UI/control-surface work requires ui_state coverage and Product
  Cohesion evidence.
- Generated-authority work requires artifact coverage and recomputation or
  digest proof.
- Workflow/orchestration work requires state-transition coverage.
- Security/trust-boundary work requires abuse-path and failure-path coverage.
- Install/cache/package work requires package/artifact coverage and same-surface
  receipts.

The validator must fail if a required behavior class lacks its required measured
dimension.

Required error codes:

- `coverage_behavior_dimension_missing`
- `coverage_ui_state_missing_for_product_surface`
- `coverage_artifact_dimension_missing_for_generated_authority`
- `coverage_state_transition_missing_for_workflow`
- `coverage_abuse_path_missing_for_trust_boundary`
- `coverage_install_surface_missing_same_surface_receipt`

#### Mandatory Coverage Policy Mutation Gate

Coverage policy is a material contract.
Agents must not weaken it.
Any change to `.harness/coverage-manifest.json` that lowers strictness is a
material policy mutation.

The validator must detect and block:

- removing a required measured dimension;
- narrowing target paths;
- reclassifying repo-owned code as generated/vendor/external;
- lowering a required floor;
- changing `100_percent_required` to `ratchet_floor`;
- deleting uncovered records without fresh receipt proof;
- deleting an exclusion rationale;
- changing `counts_as_covered = false` to true;
- changing claim ceiling upward without proof.

Weakening requires:

- blocker or explicit decision record;
- owner;
- reason;
- affected claims;
- fresh validator receipt;
- full-scope material review.

Without all required mutation proof, the validator fails.

Required error codes:

- `coverage_policy_weakened_without_review`
- `coverage_dimension_removed_without_blocker`
- `coverage_target_paths_narrowed_without_blocker`
- `coverage_floor_lowered_without_blocker`
- `coverage_repo_owned_reclassified_without_review`
- `coverage_claim_ceiling_raised_without_proof`

#### Mandatory Receipt Freshness Binding

Every coverage receipt must bind to current source state.

The receipt must include:

- source tree digest
- coverage manifest digest
- coverage command digest
- changed files digest
- coverage tool name
- coverage tool version
- workspace root
- git commit or package digest
- generated_at timestamp
- command started_at
- command completed_at
- command exit code
- machine-readable report path
- machine-readable report digest

The validator must recompute every digest it can recompute locally.
The validator must fail when a receipt is stale, copied, detached, mismatched,
or not bound to the current workspace.

Required error codes:

- `coverage_receipt_source_digest_mismatch`
- `coverage_receipt_manifest_digest_mismatch`
- `coverage_receipt_command_digest_mismatch`
- `coverage_receipt_changed_files_digest_mismatch`
- `coverage_receipt_workspace_mismatch`
- `coverage_receipt_commit_or_package_mismatch`
- `coverage_receipt_tool_version_missing`
- `coverage_receipt_report_digest_mismatch`

#### Mandatory Tool-Generated Proof

Coverage receipts must be generated by the repo-authoritative coverage command.
The validator must reject:

- hand-written coverage receipts;
- receipts edited after command execution;
- receipts copied from another workspace;
- receipts with no machine-readable source report;
- receipts whose percent was parsed from prose;
- receipts whose report path does not exist;
- receipts whose report digest does not match bytes;
- receipts whose command did not produce the report;
- receipts whose command exited nonzero.

Required error codes:

- `coverage_receipt_not_tool_generated`
- `coverage_receipt_edited_after_command`
- `coverage_receipt_workspace_mismatch`
- `coverage_report_missing`
- `coverage_report_not_machine_readable`
- `coverage_percent_from_prose`
- `coverage_report_digest_mismatch`
- `coverage_command_failed`

#### Mandatory Fast And Full Gate Separation

Repos must split coverage gates when speed matters.

Required commands:

- `scripts/check-coverage-fast`
- `scripts/check-coverage-full`

Fast coverage is a blocker gate.
Full coverage is the authority gate.
Fast coverage can reject work.
Fast coverage cannot support completion, readiness, release, production-ready,
phase advancement, or material sign-off.
Only full coverage can support those claims.

`scripts/check` must call the correct gate for the current claim:

- progress claim: fast gate plus explicit claim ceiling;
- completion/readiness/release claim: full gate;
- material sign-off: full gate;
- missing claim context: fail closed.

Required error codes:

- `coverage_fast_gate_used_for_completion`
- `coverage_full_gate_missing`
- `coverage_claim_context_missing`
- `coverage_check_did_not_run_required_gate`

#### Mandatory Script Behavior

Every Harness Ultragoal repo must contain:

- `scripts/check-coverage-fast`
- `scripts/check-coverage-full`

Both scripts must:

- read `.harness/coverage-manifest.json`;
- execute the repo-authoritative coverage command or fail;
- validate the generated coverage receipt;
- exit nonzero on any missing scope, stale receipt, invalid exclusion, target
  gap, changed-file gap, or policy mismatch.

Scripts must not:

- warn and continue;
- skip coverage because tooling is missing;
- ask the agent whether coverage matters;
- infer a lower strictness;
- substitute tests or smoke checks;
- emit prose-only proof.

#### Mandatory Green Fixtures

The plugin must include green fixtures proving validator acceptance for:

- complete coverage manifest with all source classified;
- changed repo-owned file present in coverage receipt;
- generated exclusion reviewed and not counted as covered;
- vendor exclusion reviewed and not counted as covered;
- external exclusion reviewed and not counted as covered;
- UI/product surface with ui_state coverage plus Product Cohesion evidence;
- generated authority artifact with artifact coverage plus digest proof;
- workflow state-transition coverage;
- security/trust-boundary abuse-path coverage;
- fast coverage gate supporting progress claim only;
- full coverage gate supporting completion claim.

#### Mandatory Red Fixtures

The plugin must include red fixtures proving validator rejection for:

- missing `.harness/coverage-manifest.json`;
- unclassified repo-owned source file;
- repo-owned source absent from coverage target paths;
- changed repo-owned file absent from coverage receipt;
- changed repo-owned file absent from manifest;
- target path references nonexistent file;
- target path references ignored local/cache/build state;
- generated exclusion without rationale;
- vendor exclusion without rationale;
- external exclusion without rationale;
- unreviewed exclusion;
- exclusion counted as covered;
- repo-owned source reclassified as generated;
- repo-owned source reclassified as vendor;
- repo-owned source reclassified as external;
- product/UI surface without ui_state coverage;
- generated authority without artifact coverage;
- workflow/orchestration without state-transition coverage;
- trust-boundary change without abuse/failure-path coverage;
- install/cache/package claim without same-surface coverage receipt;
- coverage policy weakened without full-scope material review;
- measured dimension removed without blocker;
- target paths narrowed without blocker;
- floor lowered without blocker;
- claim ceiling raised without proof;
- stale source tree digest;
- stale manifest digest;
- stale coverage command digest;
- stale changed-files digest;
- workspace mismatch;
- missing tool version;
- report digest mismatch;
- hand-written receipt;
- receipt edited after command;
- copied receipt from another workspace;
- missing machine-readable report;
- prose-only coverage percent;
- coverage command failed;
- fast gate used for completion;
- missing full coverage gate;
- missing claim context;
- script warns and continues after coverage failure.

Each red fixture must fail for its intended error. Wrong-error failure does not
count.

#### Mandatory Standards Enforcement

The standards surface must include `coverage-scope-authority`.

The row must be mechanized. It must name:

- `.harness/coverage-manifest.json`
- repo-walk validator
- changed-file coupling validator
- receipt freshness validator
- policy mutation validator
- fast/full gate split
- red fixtures
- repair action

If the row is missing, stale, non-mechanized, unclassified, or reviewer-memory
based, the standards gate fails. Reviewer approval cannot satisfy this row.

#### Mandatory Plugin Setup Behavior

Fresh init and retrofit must install:

- `.harness/coverage-manifest.json`
- `scripts/check-coverage-fast`
- `scripts/check-coverage-full`
- coverage schema
- coverage receipt template
- coverage standards rows
- coverage red/green fixture examples
- coverage backlog/claim-ceiling wiring

Fresh init and retrofit must fail if they cannot classify existing repo-owned
source.
Retrofit must not silently generate a permissive manifest.
If classification is incomplete, retrofit records blockers and withholds
completion/readiness/release claims.

#### Mandatory Verification

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

- every coverage-scope red fixture fails for the intended error;
- every coverage-scope green fixture passes;
- fresh target repo fixtures inherit scope authority;
- retrofit target repo fixtures inherit scope authority;
- missing manifest blocks setup;
- unclassified source blocks setup;
- changed-file gap blocks lane readiness;
- stale receipt blocks completion;
- fast gate cannot support completion;
- full gate supports completion only when all authority checks pass.

#### Coverage Scope Completion Standard

This ExecPlan is complete only when deterministic proof shows:

- coverage manifest exists in generated and retrofitted repos;
- repo walk blocks unclassified source;
- changed-file coupling blocks unmeasured changes;
- behavior-class mapping blocks missing dimensions;
- policy mutation gate blocks weakening;
- receipt freshness gate blocks stale/copied proof;
- tool-generated proof gate blocks hand-written/prose receipts;
- fast/full gate split blocks fast proof substitution;
- standards enforcement requires coverage-scope-authority;
- red fixtures pin every bypass;
- green fixtures prove valid states;
- fresh validator receipt proves the current plugin candidate;
- reviewer team signs off after deterministic proof.

#### Coverage Scope Claim Ceiling

Before all mandatory verification passes: coverage scope authority incomplete.

After package/static/fixture verification passes: plugin package/static/fixture
proof only.

This does not prove downstream repos have valid coverage scope authority. Every
downstream repo must generate and validate its own manifest, receipts, scripts,
and claim ceiling.
