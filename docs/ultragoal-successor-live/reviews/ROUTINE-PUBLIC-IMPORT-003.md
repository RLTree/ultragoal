# AC-ROUTINE-PUBLIC-IMPORT-003 review

Verdict: PASS (independent review; no root acceptance claim).

## Identity

- WorkerResult: `docs/ultragoal-successor-live/worker-results/ROUTINE-PUBLIC-IMPORT-CORRECTION-003.json`
- WorkerResult SHA-256: `8d37470c26a0ede0bc5049c544f92c8f53b677ce52458c0c4983fc7d34565914`
- Root decision SHA-256: `e096c3c509b5379cbf3264a3e4e25651526e40ffc5d15b1041ef2875090857ed`
- Lease amendment SHA-256: `8fa91231341c885240412cf7a266be8bce6f0e2c10dc404021f65f6829f16b13`
- Worker: `/root/routine_public_import_corrector` (matches amended dispatch)
- Corrected candidate/full routine set: `sha256:cc4f3589b288f4717697996d7a8f6eb8d09b2488d2f414ae301edf58ba562add`
- Full-set freeze: 46 paths, 194402 bytes, 6061 lines; no missing or extra paths versus the prior 46-row membership.
- WorkerResult artifact rows: exactly the three changed source/test paths; WorkerResult self-row excluded.

Changed paths are exactly `validator/src/routine_work/reuse/artifact.rs`, `validator/src/routine_work/reuse/receipt.rs`, and the authorized wrapper `validator/tests/routine_work_contract/provenance.rs`, plus the owned WorkerResult file.

## Typed result and false-pass controls

Reconstructed N06 dispatch envelope from the root decision/amendment and candidate binding. `WorkerResultV1::parse_json`, `validate_for`, and `ArtifactWorkspace::verify` passed in an isolated rustc harness; the computed result ID is `sha256:b7a2ad85c3e6070a1c5a9d3813b77db77bfdc14597542891d63ed9aa245f1d95`, and the verified WorkerResult artifact count is 3. Negative mutations rejected: forged worker `StaleBinding`, scope escape `UnknownScope`, performed unauthorized effect `EffectDenied`, artifact digest substitution `InvalidWorkerResult`, and altered no-claim `InvalidWorkerResult`.

The four external provenance probes passed their intended boundary: read-only inspection compiles; constructor and partial-subset forgeries fail with E0624/E0451; deserialization fails with E0277; evidence reconstruction fails with E0451. Reverting either production import fails with E0433; omitting the wrapper capture reexport fails with E0432 (`crate::capture`).

## Build/test proof

- `cargo check --manifest-path validator/Cargo.toml --lib --locked --offline -j16`: pass.
- `cargo test ... --test public_api_witness --test routine_work_contract -- --test-threads=16`: public witness 1/1 and routine contract 66/66 pass.
- `cargo test ... --lib cli::capture -- --test-threads=16`: 13 capture-focused library tests pass.
- `rustfmt --edition 2024 --check` on all three changed source/test files: pass.

The public witness compiles all eight target APIs: `CommandSpec`, `CapturedRun`, `ArtifactRef`, `ArtifactResolver`, `ImpactGraph`, `AffectedSet`, `ReuseDecision`, and `CoverageDimensions`.

## Ceiling and open scope

Supported ceiling is the corrected source-level public library/import boundary plus the tested external witness and internal routine/capture contracts. HCT-FIXTURES, inventory activation, broad CLI integration, live dirty/repeat-use journeys, product journeys, readiness, release, node closure, and completion remain unproven. The broad `cli_contract` target remains outside this correction.

This record is independent review evidence only. It does not accept the source candidate, publish public integration, activate inventory, claim product readiness, release, node closure, or goal completion.
