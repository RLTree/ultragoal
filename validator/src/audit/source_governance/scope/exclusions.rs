#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExclusionKind {
    BehaviorFixturePayload,
    BuildCache,
    FrozenEvidence,
    HistoricalContractSnapshot,
    ThirdPartySource,
}

pub(crate) struct ScopeExclusion {
    pub(crate) relative: &'static str,
    pub(crate) kind: ExclusionKind,
    pub(crate) rationale: &'static str,
}

pub(crate) const EXCLUSIONS: &[ScopeExclusion] = &[
    exclusion(
        "fixtures/",
        ExclusionKind::BehaviorFixturePayload,
        "behavior payloads are governed by fixture schemas and tests, not authored-source LOC",
    ),
    exclusion(
        "artifacts/",
        ExclusionKind::FrozenEvidence,
        "captured evidence is immutable input rather than plugin or CLI source",
    ),
    exclusion(
        "validation_artifacts/",
        ExclusionKind::FrozenEvidence,
        "receipts support claims but are not source-law authority",
    ),
    exclusion(
        "state/",
        ExclusionKind::FrozenEvidence,
        "ephemeral review state is not authored product authority",
    ),
    exclusion(
        "docs/ultragoal-successor-live/worker-results/",
        ExclusionKind::FrozenEvidence,
        "frozen WorkerResults cannot become product authority",
    ),
    exclusion(
        "docs/ultragoal-successor-live/reviews/",
        ExclusionKind::FrozenEvidence,
        "frozen reviews are evidence rather than source authority",
    ),
    exclusion(
        "docs/ultragoal-successor-live/acceptance/",
        ExclusionKind::FrozenEvidence,
        "acceptance evidence is not authored plugin or CLI source",
    ),
    exclusion(
        "docs/ultragoal-contract-2026-07/",
        ExclusionKind::HistoricalContractSnapshot,
        "superseded contract snapshot is routing context",
    ),
    exclusion(
        "docs/ultragoal-contract-2026-07-successor-candidate-v1/",
        ExclusionKind::HistoricalContractSnapshot,
        "superseded candidate is routing context",
    ),
    exclusion(
        "docs/ultragoal-contract-2026-07-successor-v2/",
        ExclusionKind::HistoricalContractSnapshot,
        "adopted frozen contract is immutable scope authority, not implementation source",
    ),
    exclusion(
        "target/",
        ExclusionKind::BuildCache,
        "reproducible build output is not authored source",
    ),
    exclusion(
        "validator/target/",
        ExclusionKind::BuildCache,
        "reproducible validator output is not authored source",
    ),
    exclusion(
        ".git/",
        ExclusionKind::BuildCache,
        "version-control object storage is outside product source",
    ),
    exclusion(
        "vendor/",
        ExclusionKind::ThirdPartySource,
        "vendored third-party code is governed by supply-chain policy",
    ),
];

const fn exclusion(
    relative: &'static str,
    kind: ExclusionKind,
    rationale: &'static str,
) -> ScopeExclusion {
    ScopeExclusion {
        relative,
        kind,
        rationale,
    }
}
