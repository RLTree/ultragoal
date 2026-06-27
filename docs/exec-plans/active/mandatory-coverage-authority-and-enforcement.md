# ExecPlan: Mandatory Coverage Authority And Enforcement For Harness Ultragoal Repos

Contract status: REQUIRED by `harness-ultragoal-plans-and-orchestrator-automation-hardening.md`. This file is not advisory. It is incorporated by reference into the active goal contract and every initialized or retrofitted Harness Ultragoal repo that inherits the plugin.

## Mandatory Coverage Authority And Enforcement

This ExecPlan defines mandatory coverage authority and enforcement for Harness
Ultragoal repos.

Coverage is a hard authority gate. Agents do not decide how strict coverage
needs to be. The repo contract decides. The validator enforces. Unsupported
claims fail.

#### Non-Negotiable Result

Every repo initialized or retrofitted by Harness Ultragoal must enforce
coverage through deterministic gates.

A repo cannot claim:

- complete
- done
- ready
- passing as completion
- production-ready
- release-ready
- phase advancement
- material sign-off

unless one of these hard states is true:

1. The repo has exact 100% coverage proof for the declared repo-owned surface.
2. The repo explicitly withholds the claim with a blocker, owner, reason, debt
   id, and claim ceiling.

There is no agent discretion. There is no "good enough." There is no "tests
passed, so ready." There is no reviewer override. There is no warning-only path.

#### Mandatory Definition Of 100% Coverage

100% coverage requires every item below:

1. Exact target paths.
2. Exact measured dimensions.
3. Configured coverage command.
4. Fresh command execution.
5. Typed coverage receipt.
6. Receipt bound to exact claim id.
7. Receipt bound to exact command.
8. Receipt bound to exact tool.
9. Receipt bound to exact target paths.
10. Receipt bound to generated timestamp.
11. `coverage.policy = 100_percent_required`.
12. `coverage.percent = 100`.
13. `uncovered_records = []`.
14. Every exclusion is mechanically listed.
15. Every exclusion has rationale.
16. Every exclusion has `reviewed = true`.
17. Every exclusion has `counts_as_covered = false`.
18. Receipt claim ceiling is `supports_complete_claim`.

If any item is missing, the claim fails.

#### Mandatory Coverage Dimensions

Every repo must declare its coverage dimensions.

The only valid dimensions are:

- `line`
- `branch`
- `function`
- `region`
- `ui_state`
- `artifact`

Dimension requirements are not optional:

- Repos with source code must enforce `line`.
- Repos with branches, conditionals, match arms, or error paths must enforce
  `branch`.
- Repos with public functions, exported APIs, commands, routes, handlers,
  workflow steps, or entrypoints must enforce `function`.
- Repos with region-capable tooling must enforce `region`.
- Repos with user-facing UI, workflow screens, dashboard states, control
  surfaces, forms, replay views, run consoles, install flows, or product routes
  must enforce `ui_state`.
- Repos with generated receipts, manifests, schemas, indexes, archives,
  packages, cache records, validator outputs, or derived authority files must
  enforce `artifact`.

If tooling cannot measure a required dimension, the repo must record a blocker.
The claim ceiling becomes `withheld_or_blocked`.

The agent must not silently drop a dimension.

#### Mandatory Source Surface

Coverage applies to every repo-owned source surface that can affect a claim.

Repo-owned means the repo owns the behavior, maintenance, and proof burden.

Coverage scope must include:

- application source
- library source
- validator source
- scripts
- shell entrypoints
- command entrypoints
- workflow/orchestration code
- schema-processing code
- UI routes
- UI states
- API routes
- proof generation code
- receipt generation code
- artifact generation code
- archive/package generation code
- install/cache verification code
- plugin/custom-agent setup code
- configuration that changes runtime behavior
- any source path that can affect the claimed behavior

The agent must not narrow this scope unless a mechanical exclusion row exists.

#### Mandatory Exclusion Contract

Only these exclusion kinds exist:

- `generated`
- `vendor`
- `external`

Every exclusion must include:

- path
- kind
- rationale
- reviewed = true
- counts_as_covered = false

Exclusions never increase coverage.

Exclusions never count as covered.

Repo-owned code cannot be excluded as generated, vendor, or external.

If a file's ownership is ambiguous, it is repo-owned until proven otherwise.

If an exclusion is missing, vague, unreviewed, or counted as covered, the
validator fails.

#### Mandatory Ratchet Contract

A repo below 100% is not complete.

A lower floor is not a completion state. It is debt.

A ratchet floor must include:

- current measured percent
- required floor percent
- exact target paths
- exact measured dimensions
- owner
- reason
- blocker or debt id
- fresh coverage artifact
- uncovered records
- required next increase
- deadline or next enforcement event
- claim ceiling `ratchet_floor_only` or `withheld_or_blocked`

Ratchet floors can support progress claims only.

Ratchet floors cannot support:

- complete
- done
- ready
- passing as completion
- production-ready
- release-ready
- phase advancement
- material sign-off

If a lower-than-100 receipt supports any completion/readiness/release claim, the
validator fails.

#### Mandatory Setup Contract

Every initialized or retrofitted repo must contain:

- coverage receipt template
- coverage receipt schema or installed schema reference
- configured coverage command path
- `scripts/check`
- coverage validator wiring
- standards row `coverage-proof-accountability`
- red fixtures for coverage bypasses
- green fixtures for valid coverage states
- backlog/claim-ceiling path for blocked coverage

The default coverage command path is:

```text
.harness/coverage-command
```

`scripts/check` must execute the configured coverage command. If
`.harness/coverage-command` is missing, empty, unreadable, or invalid,
`scripts/check` fails. It must not warn and continue. It must not skip. It must
not delegate the decision to the agent.

#### Mandatory Claim Gate

The validator must fail claims that use any of these as coverage proof:

- tests passed
- smoke tests passed
- fixture tests passed
- mocks passed
- examples ran
- generated examples passed
- reviewer approved
- CI passed
- all checks passed
- screenshots exist
- runtime started
- install worked
- target fixture passed

None of these prove coverage. They prove only their own surface. Coverage proof
requires a valid coverage receipt.

#### Mandatory Error Codes

The validator must emit stable errors:

- `coverage_receipt_missing`
- `coverage_receipt_malformed`
- `coverage_receipt_stale`
- `coverage_receipt_wrong_claim_id`
- `coverage_command_missing`
- `coverage_command_empty`
- `coverage_command_failed`
- `coverage_claim_missing_target_paths`
- `coverage_claim_missing_dimensions`
- `coverage_claim_uncovered_code`
- `coverage_required_dimension_missing`
- `coverage_exclusion_missing`
- `coverage_exclusion_missing_rationale`
- `coverage_exclusion_unreviewed`
- `coverage_exclusion_counted_as_covered`
- `coverage_repo_owned_code_excluded`
- `coverage_ratchet_missing_owner`
- `coverage_ratchet_missing_debt`
- `coverage_ratchet_presented_as_complete`
- `coverage_test_pass_substitution`
- `coverage_reviewer_signoff_substitution`
- `coverage_check_pass_substitution`

#### Mandatory Red Fixtures

The plugin must include red fixtures proving these fail:

- completion claim with tests passed and no coverage receipt;
- completion claim with CI/check pass and no coverage receipt;
- completion claim with reviewer signoff and no coverage receipt;
- 100% claim with `coverage.percent < 100`;
- 100% claim with nonempty `uncovered_records`;
- 100% claim missing target paths;
- 100% claim missing measured dimensions;
- required dimension omitted;
- receipt attached to wrong claim id;
- stale receipt;
- malformed receipt;
- missing coverage command;
- empty coverage command;
- failing coverage command;
- ratchet floor presented as complete;
- ratchet floor missing owner;
- ratchet floor missing blocker/debt id;
- exclusion without rationale;
- unreviewed exclusion;
- exclusion counted as covered;
- repo-owned code excluded as generated;
- repo-owned code excluded as vendor;
- repo-owned code excluded as external;
- smoke test substituted for coverage;
- fixture test substituted for coverage;
- mock proof substituted for coverage;
- generated example substituted for coverage.

Every red fixture must fail for the intended error. Wrong-error failure does not
count.

#### Mandatory Green Fixtures

The plugin must include green fixtures proving these pass:

- exact 100% line coverage receipt;
- exact 100% branch coverage receipt;
- exact 100% function coverage receipt;
- exact 100% artifact coverage receipt;
- exact 100% UI-state coverage receipt for a product surface;
- valid generated exclusion with `counts_as_covered = false`;
- valid vendor exclusion with `counts_as_covered = false`;
- valid external exclusion with `counts_as_covered = false`;
- valid ratchet floor with withheld completion claim;
- repo with no completion/readiness/release claim and no coverage claim;
- repo with configured coverage command and valid coverage receipt.

#### Mandatory Standards Enforcement

The standards surface must include `coverage-proof-accountability`.

The row must be mechanized. The row must name:

- source law
- required behavior
- deterministic gate
- owner
- affected claims
- repair action

If the row is missing, non-mechanized, stale, unclassified, or reviewer-memory
based, the standards gate fails. Reviewer approval cannot satisfy this row.

#### Mandatory Verification

Run and record:

```bash
bash -n templates/scripts/check templates/scripts/check-agent-standards
cargo fmt --check
cargo test --offline
cargo run --offline -- --root . audit   --receipt validation_artifacts/ultragoal-audit/validator-receipt.json   --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json
```

Also prove:

- every coverage red fixture fails for its intended error;
- every coverage green fixture passes;
- fresh target repo fixtures contain coverage enforcement;
- retrofit target repo fixtures contain coverage enforcement;
- `scripts/check` fails when coverage command is missing;
- `scripts/check` fails when coverage command is empty;
- `scripts/check` fails when completion is claimed without coverage proof;
- valid coverage receipt supports only the exact claim id it binds to.

#### Completion Standard

This ExecPlan is complete only when deterministic proof shows:

- templates install coverage enforcement;
- schemas type coverage authority;
- validators block vague coverage;
- validators block substitution;
- validators block lower-than-100 completion;
- validators block invalid exclusions;
- standards enforce coverage accountability;
- red fixtures pin every bypass;
- green fixtures prove valid states;
- target fixtures inherit the enforcement surface;
- fresh validator receipt proves the candidate;
- reviewer team signs off after deterministic proof.

#### Coverage Claim Ceiling

Before all mandatory verification passes: coverage enforcement incomplete.

After package/static/fixture verification passes: plugin package/static/fixture
proof only.

Downstream repos still need their own fresh coverage receipts. Plugin proof does
not transfer coverage proof to target repos.
