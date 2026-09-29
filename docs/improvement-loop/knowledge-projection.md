# Improvement Loop Knowledge Projection

Generated from `docs/improvement-loop-registry.json`; this file is a read-only projection.

- Projection schema: `harness-ultragoal.improvement-loop-knowledge-projection.v1`
- Registry digest: `sha256:c9359145ab01bd2f42d8a58e43765204b61ac750287dc75455481753fa5827e4`
- Generator: `validator/src/audit/improvement_loop/mod.rs`

## knowledge-projection-stale-mismatch

- Status: `adopted`
- Decision: `adopt`
- Owner: improvement-loop-audit
- Version: 1.0.0
- Review date: 2026-07-23
- Operating envelope: source-local improvement-loop registry and generated Markdown projection on a clean candidate
- Responsible layer: `harness`

**Observation.** A human-readable learning card can drift from the canonical improvement-loop registry unless its bytes are checked against the source registry.

**Mechanism.** The existing improvement-loop audit can own a pure byte-and-digest comparison for the projection.

**Intervention.** Generate the Markdown projection from registry bytes and reject any digest or byte mismatch.

**Evidence.** The current candidate has one adopted learning and one held learning in the canonical registry.; The validator recomputes the projection and refuses stale or hand-edited bytes.

**Counterevidence.** The projection check does not prove that an adopted learning improves installed runtime or human use.

### Evaluation

- baseline: Current registry loop closes without a learning adoption or projection check.
- visible: Adopted record renders the use condition, mechanism, evidence, owner, and rollback.
- held out: A second record with a different disposition renders without changing the adopted card.
- negative controls: A missing projection, stale registry digest, or altered line fails closed.
- overlap: The registry remains the only learning authority; no second store or lifecycle is introduced.
- already specified: The existing improvement-loop receipt and WorkerResult-v1 remain unchanged.
- no change: No projection or authority change is made when registry bytes and projection bytes already match.
- semantic mutation: Renaming a record field or changing its status changes the expected projection and is detected.
- specification evolution: A future registry schema change must update the renderer and schema together before adoption.
- measures: projection byte equality; registry digest equality; claim ceiling remains source-local

**Repair budget.** 2 attempts for `registry projection validation` in source-local improvement-loop registry and generated Markdown projection on a clean candidate. One renderer correction and one exact confirmation are sufficient for a deterministic projection mismatch.

**Rollback.** Remove the learning-adoption block and projection together, then retain the pre-existing loop receipt and validator behavior.

**Expiry or invalidation.** The registry schema or projection path changes without a coordinated renderer update.; The projection ceases to be generated solely from registry bytes.

**Authority handoff.** The existing improvement-loop audit remains the sole executable checker; root retains adoption, claim, release, and completion authority.

## universal-repair-budget

- Status: `held`
- Decision: `hold`
- Owner: improvement-loop-audit
- Version: 1.0.0
- Review date: 2026-07-23
- Operating envelope: behavior-level skill/control evaluation for source-local repair loops
- Responsible layer: `loop`

**Observation.** A single repair-attempt count is tempting to reuse across all behavior and operating envelopes.

**Mechanism.** The existing semantic repair circuit breaker should remain the mechanism, with each candidate record naming its own budget.

**Intervention.** Adopt one universal three-attempt repair cap for every learning and skill evaluation.

**Evidence.** The proposed universal cap is held and no new global counter is introduced.; The adopted projection control uses a two-attempt budget only for its named deterministic task family.

**Counterevidence.** Some small deterministic repairs may share a practical budget, but that does not make the budget universal.

### Evaluation

- baseline: Existing repair controls are task-specific and do not claim a universal cap.
- visible: The held record names the proposed cap and the reason it is not adopted.
- held out: A different task family requires a separately calibrated budget.
- negative controls: A record declaring universal_count true is invalid.
- overlap: No second repair tracker or lifecycle is introduced.
- already specified: The semantic repair circuit breaker remains current authority.
- no change: No universal cap is added while evidence is unresolved.
- semantic mutation: Changing the task family or envelope requires recalibration, not silent reuse.
- specification evolution: A future repair contract must preserve named calibration and the existing circuit breaker.
- measures: task-family calibration present; universal count absent; repair authority unchanged

**Repair budget.** 3 attempts for `behavior-level skill/control evaluation` in behavior-level skill/control evaluation for source-local repair loops. Provisional comparison only; not eligible for universal reuse while this learning is held.

**Rollback.** Delete this held proposal from the registry if the root retires the universal-budget question without adopting it.

**Expiry or invalidation.** A representative behavior-evaluation corpus demonstrates a calibrated budget for a named task family.; The semantic repair circuit breaker changes authority.

**Authority handoff.** The existing semantic repair circuit breaker and WorkerResult-v1 remain authoritative; this held record cannot authorize a repair or claim.
