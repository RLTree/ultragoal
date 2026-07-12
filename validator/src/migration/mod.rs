//! Candidate-bound migration planning and retirement reconciliation.
//!
//! This kernel cannot mutate a registry, route, generated surface, repository,
//! or product state. It can require a root-owned authority to atomically record
//! and consume evidence-ledger state; root must separately adopt a plan and
//! broker every product effect.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_SURFACES: usize = 16_384;
const MAX_ROUTES: usize = 4_096;
const MAX_REFS_PER_SURFACE: usize = 4_096;
const MAX_IDENTIFIER_BYTES: usize = 160;
const MAX_PATH_BYTES: usize = 768;
const MAX_AUTHORIZATION_TTL_MS: u64 = 10 * 60 * 1_000;
const REQUIRED_FALSE_PASS_CONTROLS: [&str; 5] = [
    "proof-artifact",
    "receipt-production",
    "score-only",
    "test-manipulation",
    "verbosity",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationError {
    code: &'static str,
}

impl MigrationError {
    pub(crate) fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for MigrationError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceFileKind {
    Regular,
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceStatus {
    Active,
    Candidate,
    Definition,
    ContextOnly,
    Retired,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InventorySurface {
    stable_id: String,
    kind: String,
    relative_path: String,
    digest_sha256: String,
    file_kind: SurfaceFileKind,
    link_count: u64,
    status: SurfaceStatus,
    active_readers: Vec<String>,
    active_writers: Vec<String>,
    public_routes: Vec<String>,
    generated_outputs: Vec<String>,
}

impl InventorySurface {
    #[allow(clippy::too_many_arguments)]
    pub fn observed(
        stable_id: impl Into<String>,
        kind: impl Into<String>,
        relative_path: impl Into<String>,
        digest_sha256: impl Into<String>,
        file_kind: SurfaceFileKind,
        link_count: u64,
        status: SurfaceStatus,
        active_readers: Vec<String>,
        active_writers: Vec<String>,
        public_routes: Vec<String>,
        generated_outputs: Vec<String>,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            kind: kind.into(),
            relative_path: relative_path.into(),
            digest_sha256: digest_sha256.into(),
            file_kind,
            link_count,
            status,
            active_readers: normalized(active_readers),
            active_writers: normalized(active_writers),
            public_routes: normalized(public_routes),
            generated_outputs: normalized(generated_outputs),
        }
    }

    pub fn stable_id(&self) -> &str {
        &self.stable_id
    }

    pub fn status(&self) -> SurfaceStatus {
        self.status
    }

    fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if !valid_identifier(&self.stable_id) || !valid_identifier(&self.kind) {
            findings.push("migration-surface-identity-invalid".to_owned());
        }
        if !safe_relative_path(&self.relative_path) {
            findings.push("migration-surface-path-unsafe".to_owned());
        }
        if !valid_sha256(&self.digest_sha256) {
            findings.push("migration-surface-digest-invalid".to_owned());
        }
        if self.file_kind != SurfaceFileKind::Regular {
            findings.push("migration-surface-non-regular".to_owned());
        }
        if self.link_count != 1 {
            findings.push("migration-surface-hardlink-rejected".to_owned());
        }
        for (label, values) in [
            ("reader", &self.active_readers),
            ("writer", &self.active_writers),
            ("route", &self.public_routes),
            ("generated", &self.generated_outputs),
        ] {
            if values.len() > MAX_REFS_PER_SURFACE
                || values.iter().any(|value| !safe_reference(value))
                || values.windows(2).any(|pair| pair[0] == pair[1])
            {
                findings.push(format!("migration-surface-{label}-set-invalid"));
            }
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}|{}|{:?}|{}|{}|{}|{}",
            self.stable_id,
            self.kind,
            self.relative_path,
            self.digest_sha256,
            self.file_kind,
            self.link_count,
            self.status,
            self.active_readers.join(","),
            self.active_writers.join(","),
            self.public_routes.join(","),
            self.generated_outputs.join(",")
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationInventory {
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    surfaces: Vec<InventorySurface>,
    inventory_sha256: String,
}

impl MigrationInventory {
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        catalog_id: impl Into<String>,
        read_session_id: impl Into<String>,
        mut surfaces: Vec<InventorySurface>,
    ) -> Result<Self, MigrationError> {
        if surfaces.is_empty() || surfaces.len() > MAX_SURFACES {
            return Err(MigrationError::new("migration-surface-count-out-of-bounds"));
        }
        surfaces.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
        let live_context_id = live_context_id.into();
        let candidate_id = candidate_id.into();
        let catalog_id = catalog_id.into();
        let read_session_id = read_session_id.into();
        let inventory_sha256 = inventory_digest(
            &live_context_id,
            &candidate_id,
            &catalog_id,
            &read_session_id,
            &surfaces,
        );
        let inventory = Self {
            live_context_id,
            candidate_id,
            catalog_id,
            read_session_id,
            surfaces,
            inventory_sha256,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn inventory_sha256(&self) -> &str {
        &self.inventory_sha256
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub(crate) fn live_context_id(&self) -> &str {
        &self.live_context_id
    }

    pub(crate) fn catalog_id(&self) -> &str {
        &self.catalog_id
    }

    pub(crate) fn read_session_id(&self) -> &str {
        &self.read_session_id
    }

    pub fn surfaces(&self) -> &[InventorySurface] {
        &self.surfaces
    }

    fn validate(&self) -> Result<(), MigrationError> {
        if !valid_sha256(&self.live_context_id)
            || !valid_sha256(&self.candidate_id)
            || !valid_sha256(&self.catalog_id)
            || !valid_sha256(&self.read_session_id)
        {
            return Err(MigrationError::new("migration-inventory-binding-invalid"));
        }
        let mut ids = BTreeSet::new();
        for surface in &self.surfaces {
            if !ids.insert(surface.stable_id.as_str()) {
                return Err(MigrationError::new("migration-duplicate-surface"));
            }
            if !surface.findings().is_empty() {
                return Err(MigrationError::new("migration-surface-input-refused"));
            }
        }
        if self.inventory_sha256
            != inventory_digest(
                &self.live_context_id,
                &self.candidate_id,
                &self.catalog_id,
                &self.read_session_id,
                &self.surfaces,
            )
        {
            return Err(MigrationError::new("migration-inventory-mutated"));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn rotate_session_for_test(&mut self, session: impl Into<String>) {
        self.read_session_id = session.into();
        self.inventory_sha256 = inventory_digest(
            &self.live_context_id,
            &self.candidate_id,
            &self.catalog_id,
            &self.read_session_id,
            &self.surfaces,
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompatibilityRoute {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    owner_id: String,
    warning: String,
    usage_measurement_sha256: String,
    compatibility_boundary: String,
    removal_condition: String,
    equivalence_sha256: String,
    observed_invocations: u64,
    compatibility_window_complete: bool,
}

impl CompatibilityRoute {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        route_id: impl Into<String>,
        source_id: impl Into<String>,
        canonical_target_id: impl Into<String>,
        owner_id: impl Into<String>,
        warning: impl Into<String>,
        usage_measurement_sha256: impl Into<String>,
        compatibility_boundary: impl Into<String>,
        removal_condition: impl Into<String>,
        equivalence_sha256: impl Into<String>,
        observed_invocations: u64,
        compatibility_window_complete: bool,
    ) -> Self {
        Self {
            route_id: route_id.into(),
            source_id: source_id.into(),
            canonical_target_id: canonical_target_id.into(),
            owner_id: owner_id.into(),
            warning: warning.into(),
            usage_measurement_sha256: usage_measurement_sha256.into(),
            compatibility_boundary: compatibility_boundary.into(),
            removal_condition: removal_condition.into(),
            equivalence_sha256: equivalence_sha256.into(),
            observed_invocations,
            compatibility_window_complete,
        }
    }

    pub fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }

    pub(crate) fn canonical_target_id(&self) -> &str {
        &self.canonical_target_id
    }

    pub fn validate(&self) -> Result<(), MigrationError> {
        for value in [
            self.route_id.as_str(),
            self.source_id.as_str(),
            self.canonical_target_id.as_str(),
            self.owner_id.as_str(),
        ] {
            if !valid_identifier(value) {
                return Err(MigrationError::new("migration-route-identity-invalid"));
            }
        }
        if self.source_id == self.canonical_target_id {
            return Err(MigrationError::new("migration-route-self-target"));
        }
        if self.warning.is_empty()
            || self.warning.len() > 512
            || self.warning.chars().any(char::is_control)
            || !self.warning.to_ascii_lowercase().contains("compatib")
        {
            return Err(MigrationError::new("migration-route-warning-invalid"));
        }
        if !valid_sha256(&self.usage_measurement_sha256) || !valid_sha256(&self.equivalence_sha256)
        {
            return Err(MigrationError::new("migration-route-proof-invalid"));
        }
        if !safe_reference(&self.compatibility_boundary) || !safe_reference(&self.removal_condition)
        {
            return Err(MigrationError::new("migration-route-boundary-invalid"));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.route_id,
            self.source_id,
            self.canonical_target_id,
            self.owner_id,
            self.warning,
            self.usage_measurement_sha256,
            self.compatibility_boundary,
            self.removal_condition,
            self.equivalence_sha256,
            self.observed_invocations,
            self.compatibility_window_complete
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceVerdict {
    Passed,
    CausalFailure,
}

/// Kernel-issued key for one persistent replacement-evidence ledger row. The
/// constructor is private so adapters can verify or store a key but cannot
/// reconstruct one from caller-supplied embedded plan fields.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ReplacementLedgerBinding {
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    reviewer_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    evidence_binding_sha256: String,
    evidence_id: String,
    attestation_sha256: String,
    consumption_sha256: String,
    consumption_binding_sha256: String,
    ledger_binding_sha256: String,
}

impl ReplacementLedgerBinding {
    pub(crate) fn route_id(&self) -> &str {
        &self.route_id
    }

    pub(crate) fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    pub(crate) fn ledger_binding_sha256(&self) -> &str {
        &self.ledger_binding_sha256
    }

    pub(crate) fn authority_id(&self) -> &str {
        &self.authority_id
    }

    pub(crate) fn authority_session_id(&self) -> &str {
        &self.authority_session_id
    }

    pub(crate) fn consumption_sha256(&self) -> &str {
        &self.consumption_sha256
    }
}

impl fmt::Debug for ReplacementLedgerBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplacementLedgerBinding")
            .field("contents", &"<redacted>")
            .finish()
    }
}

pub(crate) trait ReplacementEvidenceAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self, route_id: &str) -> Option<&str>;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self, route_id: &str) -> Option<&str>;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str, &str);
    fn old_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn new_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn representative_journey(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn false_pass_control_results(
        &self,
        route_id: &str,
    ) -> Option<&BTreeMap<String, (EvidenceVerdict, String)>>;
    fn rollback_execution(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)>;
    fn issue_attestation(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
    ) -> Result<String, MigrationError>;
    fn verify_attestation(
        &self,
        route_id: &str,
        binding_sha256: &str,
        attestation_sha256: &str,
    ) -> bool;
    fn consume_once(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
    ) -> Result<String, MigrationError>;
    fn verify_consumed(
        &self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
        consumption_sha256: &str,
    ) -> bool;
    /// Atomically binds an already-consumed evidence row to its exact plan and
    /// target. Implementations must reject missing, revoked, or rolled-back
    /// consumption and conflicting bindings.
    fn bind_plan_target(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<(), MigrationError>;
    /// One atomic read of persistent current/unrevoked consumption and the
    /// exact plan/target binding.
    fn verify_plan_target_consumed(&mut self, binding: &ReplacementLedgerBinding) -> bool;
    /// Atomically elects one final-reconciliation winner for this ledger row.
    fn claim_final_reconciliation(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<String, MigrationError>;
    /// Revalidates the elected winner and the underlying persistent row.
    fn verify_final_claim(
        &mut self,
        binding: &ReplacementLedgerBinding,
        claim_sha256: &str,
    ) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct ReplacementObservation {
    old_behavior_id: String,
    old_verdict: EvidenceVerdict,
    old_result_sha256: String,
    new_behavior_id: String,
    new_verdict: EvidenceVerdict,
    new_result_sha256: String,
    journey_execution_id: String,
    journey_verdict: EvidenceVerdict,
    journey_result_sha256: String,
    false_pass_control_results: BTreeMap<String, (EvidenceVerdict, String)>,
    rollback_execution_id: String,
    rollback_verdict: EvidenceVerdict,
    rollback_result_sha256: String,
}

impl ReplacementObservation {
    fn capture<A: ReplacementEvidenceAuthority + ?Sized>(
        route: &CompatibilityRoute,
        authority: &A,
    ) -> Result<Self, MigrationError> {
        let (old_behavior_id, old_verdict, old_result_sha256) = authority
            .old_behavior(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let (new_behavior_id, new_verdict, new_result_sha256) = authority
            .new_behavior(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let (journey_execution_id, journey_verdict, journey_result_sha256) = authority
            .representative_journey(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let false_pass_control_results = authority
            .false_pass_control_results(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?
            .clone();
        let (rollback_execution_id, rollback_verdict, rollback_result_sha256) = authority
            .rollback_execution(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-observation-missing"))?;
        let observation = Self {
            old_behavior_id: old_behavior_id.to_owned(),
            old_verdict,
            old_result_sha256: old_result_sha256.to_owned(),
            new_behavior_id: new_behavior_id.to_owned(),
            new_verdict,
            new_result_sha256: new_result_sha256.to_owned(),
            journey_execution_id: journey_execution_id.to_owned(),
            journey_verdict,
            journey_result_sha256: journey_result_sha256.to_owned(),
            false_pass_control_results,
            rollback_execution_id: rollback_execution_id.to_owned(),
            rollback_verdict,
            rollback_result_sha256: rollback_result_sha256.to_owned(),
        };
        observation.validate(route)?;
        Ok(observation)
    }

    fn validate(&self, route: &CompatibilityRoute) -> Result<(), MigrationError> {
        if self.old_behavior_id != route.source_id
            || self.new_behavior_id != route.canonical_target_id
            || !valid_identifier(&self.journey_execution_id)
            || !valid_identifier(&self.rollback_execution_id)
            || self.old_verdict != EvidenceVerdict::Passed
            || self.new_verdict != EvidenceVerdict::Passed
            || self.journey_verdict != EvidenceVerdict::Passed
            || self.rollback_verdict != EvidenceVerdict::Passed
        {
            return Err(MigrationError::new(
                "migration-replacement-observation-invalid",
            ));
        }
        let expected_controls = REQUIRED_FALSE_PASS_CONTROLS
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if self
            .false_pass_control_results
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>()
            != expected_controls
            || self
                .false_pass_control_results
                .values()
                .any(|(verdict, digest)| {
                    *verdict != EvidenceVerdict::CausalFailure || !valid_sha256(digest)
                })
            || [
                self.old_result_sha256.as_str(),
                self.new_result_sha256.as_str(),
                self.journey_result_sha256.as_str(),
                self.rollback_result_sha256.as_str(),
            ]
            .iter()
            .any(|digest| !valid_sha256(digest))
        {
            return Err(MigrationError::new(
                "migration-replacement-controls-incomplete",
            ));
        }
        let all_digests = [
            self.old_result_sha256.as_str(),
            self.new_result_sha256.as_str(),
            self.journey_result_sha256.as_str(),
            self.rollback_result_sha256.as_str(),
        ]
        .into_iter()
        .chain(
            self.false_pass_control_results
                .values()
                .map(|(_, digest)| digest.as_str()),
        )
        .collect::<Vec<_>>();
        if all_digests.iter().copied().collect::<BTreeSet<_>>().len() == 1 {
            return Err(MigrationError::new(
                "migration-replacement-repeated-digest-refused",
            ));
        }
        if self
            .false_pass_control_results
            .values()
            .map(|(_, digest)| digest)
            .collect::<BTreeSet<_>>()
            .len()
            != REQUIRED_FALSE_PASS_CONTROLS.len()
        {
            return Err(MigrationError::new(
                "migration-replacement-controls-incomplete",
            ));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        let controls = self
            .false_pass_control_results
            .iter()
            .map(|(control, (verdict, digest))| format!("{control}:{verdict:?}:{digest}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{:?}|{}|{}|{:?}|{}|{}|{:?}|{}|{}|{}|{:?}|{}",
            self.old_behavior_id,
            self.old_verdict,
            self.old_result_sha256,
            self.new_behavior_id,
            self.new_verdict,
            self.new_result_sha256,
            self.journey_execution_id,
            self.journey_verdict,
            self.journey_result_sha256,
            controls,
            self.rollback_execution_id,
            self.rollback_verdict,
            self.rollback_result_sha256,
        )
    }
}

/// Opaque, root-observed replacement evidence. There is no public constructor,
/// clone, or deserialize path.
#[derive(Eq, PartialEq)]
pub struct ReplacementEvidence {
    source_id: String,
    canonical_target_id: String,
    route_id: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    observation: ReplacementObservation,
    semantic_sha256: String,
    reviewer_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    binding_sha256: String,
    evidence_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for ReplacementEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReplacementEvidence")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl ReplacementEvidence {
    pub(crate) fn issue<A: ReplacementEvidenceAuthority>(
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        inventory.validate()?;
        route.validate()?;
        let observation = ReplacementObservation::capture(route, authority)?;
        let reviewer_id = authority
            .reviewer_id(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-reviewer-missing"))?
            .to_owned();
        let authority_id = authority.authority_id().to_owned();
        let authority_session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority
            .nonce_sha256(&route.route_id)
            .ok_or_else(|| MigrationError::new("migration-replacement-nonce-missing"))?
            .to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let (context, candidate, catalog, read_session, inventory_sha256) =
            authority.current_binding();
        if !valid_identifier(&reviewer_id)
            || !valid_identifier(&authority_id)
            || reviewer_id == authority_id
            || reviewer_id == route.owner_id
            || authority_id == route.owner_id
            || !valid_sha256(&authority_session_id)
            || authority_session_id == inventory.read_session_id
            || !valid_sha256(&nonce_sha256)
            || !valid_authority_window(
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority.now_unix_ms(),
            )
            || (context, candidate, catalog, read_session, inventory_sha256)
                != (
                    inventory.live_context_id.as_str(),
                    inventory.candidate_id.as_str(),
                    inventory.catalog_id.as_str(),
                    inventory.read_session_id.as_str(),
                    inventory.inventory_sha256.as_str(),
                )
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-issuance-refused",
            ));
        }
        let semantic_sha256 = digest(observation.digest_fragment().as_bytes());
        let binding_sha256 = replacement_evidence_binding(
            inventory,
            route,
            &observation,
            &semantic_sha256,
            &reviewer_id,
            &authority_id,
            &authority_session_id,
            &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
        );
        let attestation_sha256 = authority.issue_attestation(&route.route_id, &binding_sha256)?;
        let evidence_id = digest(
            format!("replacement-evidence|{binding_sha256}|{attestation_sha256}").as_bytes(),
        );
        if !valid_sha256(&attestation_sha256)
            || !authority.verify_attestation(&route.route_id, &binding_sha256, &attestation_sha256)
            || !replacement_authority_matches(
                inventory,
                route,
                &observation,
                &reviewer_id,
                &authority_id,
                &authority_session_id,
                &nonce_sha256,
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority,
            )
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-issuance-refused",
            ));
        }
        Ok(Self {
            source_id: route.source_id.clone(),
            canonical_target_id: route.canonical_target_id.clone(),
            route_id: route.route_id.clone(),
            live_context_id: inventory.live_context_id.clone(),
            candidate_id: inventory.candidate_id.clone(),
            catalog_id: inventory.catalog_id.clone(),
            read_session_id: inventory.read_session_id.clone(),
            inventory_sha256: inventory.inventory_sha256.clone(),
            observation,
            semantic_sha256,
            reviewer_id,
            authority_id,
            authority_session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            binding_sha256,
            evidence_id,
            attestation_sha256,
        })
    }

    fn validate_current<A: ReplacementEvidenceAuthority>(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &A,
    ) -> Result<(), MigrationError> {
        self.observation.validate(route)?;
        let semantic_sha256 = digest(self.observation.digest_fragment().as_bytes());
        let binding_sha256 = replacement_evidence_binding(
            inventory,
            route,
            &self.observation,
            &semantic_sha256,
            &self.reviewer_id,
            &self.authority_id,
            &self.authority_session_id,
            &self.nonce_sha256,
            self.issued_at_unix_ms,
            self.expires_at_unix_ms,
        );
        let evidence_id = digest(
            format!(
                "replacement-evidence|{}|{}",
                binding_sha256, self.attestation_sha256
            )
            .as_bytes(),
        );
        if self.source_id != route.source_id
            || self.canonical_target_id != route.canonical_target_id
            || self.route_id != route.route_id
            || self.live_context_id != inventory.live_context_id
            || self.candidate_id != inventory.candidate_id
            || self.catalog_id != inventory.catalog_id
            || self.read_session_id != inventory.read_session_id
            || self.inventory_sha256 != inventory.inventory_sha256
            || self.semantic_sha256 != semantic_sha256
            || self.binding_sha256 != binding_sha256
            || self.evidence_id != evidence_id
            || !valid_sha256(&self.attestation_sha256)
            || !authority.verify_attestation(
                &route.route_id,
                &binding_sha256,
                &self.attestation_sha256,
            )
            || !replacement_authority_matches(
                inventory,
                route,
                &self.observation,
                &self.reviewer_id,
                &self.authority_id,
                &self.authority_session_id,
                &self.nonce_sha256,
                self.issued_at_unix_ms,
                self.expires_at_unix_ms,
                authority,
            )
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-stale-or-substituted",
            ));
        }
        Ok(())
    }

    fn consume<A: ReplacementEvidenceAuthority>(
        self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &mut A,
    ) -> Result<ConsumedReplacementEvidence, MigrationError> {
        self.validate_current(inventory, route, authority)?;
        let consumption_sha256 = authority.consume_once(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
        )?;
        if !valid_sha256(&consumption_sha256)
            || !authority.verify_consumed(
                &self.route_id,
                &self.binding_sha256,
                &self.evidence_id,
                &self.attestation_sha256,
                &consumption_sha256,
            )
            || self.validate_current(inventory, route, authority).is_err()
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-consumption-refused",
            ));
        }
        let consumption_binding_sha256 = replacement_consumption_binding(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
            &consumption_sha256,
        );
        Ok(ConsumedReplacementEvidence {
            source_id: self.source_id,
            canonical_target_id: self.canonical_target_id,
            route_id: self.route_id,
            live_context_id: self.live_context_id,
            candidate_id: self.candidate_id,
            catalog_id: self.catalog_id,
            read_session_id: self.read_session_id,
            inventory_sha256: self.inventory_sha256,
            observation: self.observation,
            semantic_sha256: self.semantic_sha256,
            reviewer_id: self.reviewer_id,
            authority_id: self.authority_id,
            authority_session_id: self.authority_session_id,
            nonce_sha256: self.nonce_sha256,
            issued_at_unix_ms: self.issued_at_unix_ms,
            expires_at_unix_ms: self.expires_at_unix_ms,
            binding_sha256: self.binding_sha256,
            evidence_id: self.evidence_id,
            attestation_sha256: self.attestation_sha256,
            consumption_sha256,
            consumption_binding_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_semantic_for_test(&mut self, digest: impl Into<String>) {
        self.semantic_sha256 = digest.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_attestation_for_test(&mut self, digest: impl Into<String>) {
        self.attestation_sha256 = digest.into();
    }
}

#[allow(clippy::too_many_arguments)]
fn replacement_evidence_binding(
    inventory: &MigrationInventory,
    route: &CompatibilityRoute,
    observation: &ReplacementObservation,
    semantic_sha256: &str,
    reviewer_id: &str,
    authority_id: &str,
    authority_session_id: &str,
    nonce_sha256: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
) -> String {
    digest(
        format!(
            "replacement-evidence-v2|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            inventory.live_context_id,
            inventory.candidate_id,
            inventory.catalog_id,
            inventory.read_session_id,
            inventory.inventory_sha256,
            route.route_id,
            route.source_id,
            route.canonical_target_id,
            route.digest_fragment(),
            observation.digest_fragment(),
            semantic_sha256,
            reviewer_id,
            authority_id,
            authority_session_id,
            nonce_sha256,
            format_args!("{issued_at_unix_ms}:{expires_at_unix_ms}"),
        )
        .as_bytes(),
    )
}

#[allow(clippy::too_many_arguments)]
fn replacement_authority_matches<A: ReplacementEvidenceAuthority + ?Sized>(
    inventory: &MigrationInventory,
    route: &CompatibilityRoute,
    observation: &ReplacementObservation,
    reviewer_id: &str,
    authority_id: &str,
    authority_session_id: &str,
    nonce_sha256: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    authority: &A,
) -> bool {
    let current_binding = authority.current_binding();
    authority.authority_id() == authority_id
        && authority.reviewer_id(&route.route_id) == Some(reviewer_id)
        && authority.session_id() == authority_session_id
        && authority.nonce_sha256(&route.route_id) == Some(nonce_sha256)
        && authority.issued_at_unix_ms() == issued_at_unix_ms
        && authority.expires_at_unix_ms() == expires_at_unix_ms
        && valid_identifier(reviewer_id)
        && valid_identifier(authority_id)
        && reviewer_id != authority_id
        && reviewer_id != route.owner_id
        && authority_id != route.owner_id
        && valid_sha256(authority_session_id)
        && authority_session_id != inventory.read_session_id
        && valid_sha256(nonce_sha256)
        && valid_authority_window(
            issued_at_unix_ms,
            expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && current_binding
            == (
                inventory.live_context_id.as_str(),
                inventory.candidate_id.as_str(),
                inventory.catalog_id.as_str(),
                inventory.read_session_id.as_str(),
                inventory.inventory_sha256.as_str(),
            )
        && ReplacementObservation::capture(route, authority).as_ref() == Ok(observation)
}

#[derive(Eq, PartialEq)]
struct ConsumedReplacementEvidence {
    source_id: String,
    canonical_target_id: String,
    route_id: String,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    observation: ReplacementObservation,
    semantic_sha256: String,
    reviewer_id: String,
    authority_id: String,
    authority_session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    binding_sha256: String,
    evidence_id: String,
    attestation_sha256: String,
    consumption_sha256: String,
    consumption_binding_sha256: String,
}

impl fmt::Debug for ConsumedReplacementEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ConsumedReplacementEvidence")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl ConsumedReplacementEvidence {
    fn validate(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        now_unix_ms: u64,
    ) -> Result<(), MigrationError> {
        self.observation.validate(route)?;
        let semantic_sha256 = digest(self.observation.digest_fragment().as_bytes());
        let binding_sha256 = replacement_evidence_binding(
            inventory,
            route,
            &self.observation,
            &semantic_sha256,
            &self.reviewer_id,
            &self.authority_id,
            &self.authority_session_id,
            &self.nonce_sha256,
            self.issued_at_unix_ms,
            self.expires_at_unix_ms,
        );
        let evidence_id = digest(
            format!(
                "replacement-evidence|{}|{}",
                binding_sha256, self.attestation_sha256
            )
            .as_bytes(),
        );
        let consumption_binding_sha256 = replacement_consumption_binding(
            &self.route_id,
            &binding_sha256,
            &evidence_id,
            &self.attestation_sha256,
            &self.consumption_sha256,
        );
        if self.source_id != route.source_id
            || self.canonical_target_id != route.canonical_target_id
            || self.route_id != route.route_id
            || self.live_context_id != inventory.live_context_id
            || self.candidate_id != inventory.candidate_id
            || self.catalog_id != inventory.catalog_id
            || self.read_session_id != inventory.read_session_id
            || self.inventory_sha256 != inventory.inventory_sha256
            || self.semantic_sha256 != semantic_sha256
            || self.binding_sha256 != binding_sha256
            || self.evidence_id != evidence_id
            || !valid_identifier(&self.reviewer_id)
            || !valid_identifier(&self.authority_id)
            || self.reviewer_id == self.authority_id
            || self.reviewer_id == route.owner_id
            || self.authority_id == route.owner_id
            || !valid_sha256(&self.authority_session_id)
            || self.authority_session_id == inventory.read_session_id
            || !valid_sha256(&self.nonce_sha256)
            || !valid_authority_window(self.issued_at_unix_ms, self.expires_at_unix_ms, now_unix_ms)
            || !valid_sha256(&self.attestation_sha256)
            || !valid_sha256(&self.consumption_sha256)
            || self.consumption_binding_sha256 != consumption_binding_sha256
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-invalid",
            ));
        }
        Ok(())
    }

    fn validate_with_authority<A: ReplacementEvidenceAuthority + ?Sized>(
        &self,
        inventory: &MigrationInventory,
        route: &CompatibilityRoute,
        authority: &A,
    ) -> Result<(), MigrationError> {
        self.validate(inventory, route, authority.now_unix_ms())?;
        if !replacement_authority_matches(
            inventory,
            route,
            &self.observation,
            &self.reviewer_id,
            &self.authority_id,
            &self.authority_session_id,
            &self.nonce_sha256,
            self.issued_at_unix_ms,
            self.expires_at_unix_ms,
            authority,
        ) || !authority.verify_attestation(
            &self.route_id,
            &self.binding_sha256,
            &self.attestation_sha256,
        ) || !authority.verify_consumed(
            &self.route_id,
            &self.binding_sha256,
            &self.evidence_id,
            &self.attestation_sha256,
            &self.consumption_sha256,
        ) {
            return Err(MigrationError::new(
                "migration-replacement-ledger-not-current",
            ));
        }
        Ok(())
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.source_id,
            self.canonical_target_id,
            self.route_id,
            self.live_context_id,
            self.candidate_id,
            self.catalog_id,
            self.read_session_id,
            self.inventory_sha256,
            self.observation.digest_fragment(),
            self.semantic_sha256,
            self.reviewer_id,
            self.authority_id,
            self.authority_session_id,
            self.nonce_sha256,
            self.issued_at_unix_ms,
            self.expires_at_unix_ms,
            self.binding_sha256,
            self.evidence_id,
            self.attestation_sha256,
            self.consumption_sha256,
            self.consumption_binding_sha256,
        )
    }

    #[cfg(test)]
    fn inject_serialization_sentinels_for_test(&mut self) -> Vec<String> {
        let mut sentinels = Vec::new();
        let mut marker = |label: &str| {
            let value = format!("opaque-consumed-replacement-{label}-marker");
            sentinels.push(value.clone());
            value
        };

        self.source_id = marker("source-id");
        self.canonical_target_id = marker("canonical-target-id");
        self.route_id = marker("route-id");
        self.live_context_id = marker("live-context-id");
        self.candidate_id = marker("candidate-id");
        self.catalog_id = marker("catalog-id");
        self.read_session_id = marker("read-session-id");
        self.inventory_sha256 = marker("inventory-sha256");
        self.observation.old_behavior_id = marker("old-behavior-id");
        self.observation.old_verdict = EvidenceVerdict::CausalFailure;
        self.observation.old_result_sha256 = marker("old-result-sha256");
        self.observation.new_behavior_id = marker("new-behavior-id");
        self.observation.new_verdict = EvidenceVerdict::Passed;
        self.observation.new_result_sha256 = marker("new-result-sha256");
        self.observation.journey_execution_id = marker("journey-execution-id");
        self.observation.journey_verdict = EvidenceVerdict::CausalFailure;
        self.observation.journey_result_sha256 = marker("journey-result-sha256");
        let mut controls = BTreeMap::new();
        for index in 0..REQUIRED_FALSE_PASS_CONTROLS.len() {
            controls.insert(
                marker(&format!("false-pass-control-{index}-id")),
                (
                    EvidenceVerdict::CausalFailure,
                    marker(&format!("false-pass-control-{index}-digest")),
                ),
            );
        }
        self.observation.false_pass_control_results = controls;
        self.observation.rollback_execution_id = marker("rollback-execution-id");
        self.observation.rollback_verdict = EvidenceVerdict::Passed;
        self.observation.rollback_result_sha256 = marker("rollback-result-sha256");
        self.semantic_sha256 = marker("semantic-sha256");
        self.reviewer_id = marker("reviewer-id");
        self.authority_id = marker("authority-id");
        self.authority_session_id = marker("authority-session-id");
        self.nonce_sha256 = marker("nonce-sha256");
        self.issued_at_unix_ms = 88_101;
        self.expires_at_unix_ms = 88_102;
        self.binding_sha256 = marker("binding-sha256");
        self.evidence_id = marker("evidence-id");
        self.attestation_sha256 = marker("attestation-sha256");
        self.consumption_sha256 = marker("consumption-sha256");
        self.consumption_binding_sha256 = marker("consumption-binding-sha256");
        drop(marker);

        sentinels.extend(
            ["88101", "88102", "Passed", "CausalFailure"]
                .into_iter()
                .map(str::to_owned),
        );
        sentinels
    }
}

fn replacement_consumption_binding(
    route_id: &str,
    binding_sha256: &str,
    evidence_id: &str,
    attestation_sha256: &str,
    consumption_sha256: &str,
) -> String {
    digest(
        format!(
            "replacement-evidence-consumed-v1|{route_id}|{binding_sha256}|{evidence_id}|{attestation_sha256}|{consumption_sha256}"
        )
        .as_bytes(),
    )
}

#[derive(Eq, PartialEq)]
pub struct RetirementTarget {
    target_id: String,
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    source_status: SurfaceStatus,
    active_readers: Vec<String>,
    active_writers: Vec<String>,
    public_routes: Vec<String>,
    generated_outputs: Vec<String>,
    observed_invocations: u64,
    compatibility_window_complete: bool,
    owner_id: String,
    replacement_summary_sha256: String,
}

impl fmt::Debug for RetirementTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("RetirementTarget")
            .field(&self.projection())
            .finish()
    }
}

impl RetirementTarget {
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn active_reader_count(&self) -> usize {
        self.active_readers.len()
    }

    pub fn active_writer_count(&self) -> usize {
        self.active_writers.len()
    }

    /// Returns the deliberately safe, inspection-only representation. It
    /// contains counts and domain-separated commitments, never replacement
    /// authority, evidence, attestation, session, nonce, or consumption data.
    pub fn projection(&self) -> RetirementTargetProjection {
        let inspection_commitment_sha256 = digest(
            format!("retirement-target-inspection-v1|{}", self.digest_fragment()).as_bytes(),
        );
        RetirementTargetProjection {
            schema_version: "RetirementTargetProjection-v1".to_owned(),
            target_id: self.target_id.clone(),
            route_id: self.route_id.clone(),
            source_id: self.source_id.clone(),
            canonical_target_id: self.canonical_target_id.clone(),
            source_status: format!("{:?}", self.source_status),
            active_reader_count: self.active_readers.len(),
            active_writer_count: self.active_writers.len(),
            public_route_count: self.public_routes.len(),
            generated_output_count: self.generated_outputs.len(),
            observed_invocations: self.observed_invocations,
            compatibility_window_complete: self.compatibility_window_complete,
            inspection_commitment_sha256,
            replacement_summary_sha256: self.replacement_summary_sha256.clone(),
        }
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.target_id,
            self.route_id,
            self.source_id,
            self.canonical_target_id,
            self.source_status,
            self.active_readers.join(","),
            self.active_writers.join(","),
            self.public_routes.join(","),
            self.generated_outputs.join(","),
            self.observed_invocations,
            self.compatibility_window_complete,
            self.owner_id,
            self.replacement_summary_sha256,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetirementTargetProjection {
    schema_version: String,
    target_id: String,
    route_id: String,
    source_id: String,
    canonical_target_id: String,
    source_status: String,
    active_reader_count: usize,
    active_writer_count: usize,
    public_route_count: usize,
    generated_output_count: usize,
    observed_invocations: u64,
    compatibility_window_complete: bool,
    inspection_commitment_sha256: String,
    replacement_summary_sha256: String,
}

impl RetirementTargetProjection {
    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.schema_version,
            self.target_id,
            self.route_id,
            self.source_id,
            self.canonical_target_id,
            self.source_status,
            self.active_reader_count,
            self.active_writer_count,
            self.public_route_count,
            self.generated_output_count,
            self.observed_invocations,
            self.compatibility_window_complete,
            self.inspection_commitment_sha256,
            self.replacement_summary_sha256,
        )
    }
}

#[derive(Eq, PartialEq)]
struct PlanReplacementLedger {
    by_target_id: BTreeMap<String, ConsumedReplacementEvidence>,
}

impl fmt::Debug for PlanReplacementLedger {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PlanReplacementLedger")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl PlanReplacementLedger {
    fn new() -> Self {
        Self {
            by_target_id: BTreeMap::new(),
        }
    }

    fn insert(
        &mut self,
        target_id: String,
        evidence: ConsumedReplacementEvidence,
    ) -> Result<(), MigrationError> {
        if self.by_target_id.insert(target_id, evidence).is_some() {
            return Err(MigrationError::new(
                "migration-duplicate-private-replacement-evidence",
            ));
        }
        Ok(())
    }

    fn get(&self, target_id: &str) -> Option<&ConsumedReplacementEvidence> {
        self.by_target_id.get(target_id)
    }
}

/// Inspection-only migration plan. Round-tripping this DTO never reconstructs
/// a `MigrationPlan` and cannot carry private replacement authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlanProjection {
    schema_version: String,
    plan_sha256: String,
    context_commitment_sha256: String,
    route_count: usize,
    target_count: usize,
    targets: Vec<RetirementTargetProjection>,
    projection_sha256: String,
}

#[derive(Eq, PartialEq)]
pub struct MigrationPlan {
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    routes: Vec<CompatibilityRoute>,
    targets: Vec<RetirementTarget>,
    replacement_ledger: PlanReplacementLedger,
}

impl fmt::Debug for MigrationPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MigrationPlan")
            .field("plan_sha256", &self.plan_sha256)
            .field("target_count", &self.targets.len())
            .field("private_replacement_ledger", &"<redacted>")
            .finish()
    }
}

impl ReplacementLedgerBinding {
    fn issue(
        plan: &MigrationPlan,
        target: &RetirementTarget,
        current: &MigrationInventory,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let evidence = plan
            .replacement_evidence(target)
            .ok_or_else(|| MigrationError::new("migration-private-replacement-evidence-missing"))?;
        if target.route_id != evidence.route_id
            || target.source_id != evidence.source_id
            || target.canonical_target_id != evidence.canonical_target_id
        {
            return Err(MigrationError::new(
                "migration-replacement-ledger-binding-invalid",
            ));
        }
        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let ledger_binding_sha256 = digest(
            format!(
                "replacement-ledger-plan-target-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                current.live_context_id,
                current.candidate_id,
                current.catalog_id,
                current.read_session_id,
                current.inventory_sha256,
                plan.plan_sha256,
                target.target_id,
                target_sha256,
                target.route_id,
                target.source_id,
                target.canonical_target_id,
                evidence.digest_fragment(),
            )
            .as_bytes(),
        );
        Ok(Self {
            route_id: target.route_id.clone(),
            source_id: target.source_id.clone(),
            canonical_target_id: target.canonical_target_id.clone(),
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            read_session_id: current.read_session_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            reviewer_id: evidence.reviewer_id.clone(),
            authority_id: evidence.authority_id.clone(),
            authority_session_id: evidence.authority_session_id.clone(),
            nonce_sha256: evidence.nonce_sha256.clone(),
            issued_at_unix_ms: evidence.issued_at_unix_ms,
            expires_at_unix_ms: evidence.expires_at_unix_ms,
            evidence_binding_sha256: evidence.binding_sha256.clone(),
            evidence_id: evidence.evidence_id.clone(),
            attestation_sha256: evidence.attestation_sha256.clone(),
            consumption_sha256: evidence.consumption_sha256.clone(),
            consumption_binding_sha256: evidence.consumption_binding_sha256.clone(),
            ledger_binding_sha256,
        })
    }
}

impl MigrationPlan {
    pub(crate) fn build<A: ReplacementEvidenceAuthority>(
        inventory: &MigrationInventory,
        mut routes: Vec<CompatibilityRoute>,
        evidence: Vec<ReplacementEvidence>,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        inventory.validate()?;
        if routes.is_empty() || routes.len() > MAX_ROUTES || routes.len() != evidence.len() {
            return Err(MigrationError::new(
                "migration-route-evidence-count-invalid",
            ));
        }
        routes.sort_by(|left, right| left.route_id.cmp(&right.route_id));
        let surfaces = inventory
            .surfaces
            .iter()
            .map(|surface| (surface.stable_id.as_str(), surface))
            .collect::<BTreeMap<_, _>>();
        let mut evidence = evidence
            .into_iter()
            .map(|row| (row.source_id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        if evidence.len() != routes.len() {
            return Err(MigrationError::new(
                "migration-duplicate-replacement-evidence",
            ));
        }
        let mut route_ids = BTreeSet::new();
        let mut source_ids = BTreeSet::new();
        let mut targets = Vec::with_capacity(routes.len());
        let mut replacement_ledger = PlanReplacementLedger::new();
        for route in &routes {
            route.validate()?;
            if !route_ids.insert(route.route_id.as_str()) {
                return Err(MigrationError::new("migration-duplicate-route"));
            }
            if !source_ids.insert(route.source_id.as_str()) {
                return Err(MigrationError::new("migration-ambiguous-source-route"));
            }
            let source = surfaces
                .get(route.source_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-route-source-unknown"))?;
            let canonical = surfaces
                .get(route.canonical_target_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-route-target-unknown"))?;
            if canonical.status == SurfaceStatus::Retired
                || canonical.status == SurfaceStatus::ContextOnly
            {
                return Err(MigrationError::new("migration-canonical-target-inactive"));
            }
            let replacement = evidence
                .remove(route.source_id.as_str())
                .ok_or_else(|| MigrationError::new("migration-replacement-evidence-missing"))?;
            if replacement.canonical_target_id != route.canonical_target_id {
                return Err(MigrationError::new(
                    "migration-replacement-evidence-conflict",
                ));
            }
            let replacement = replacement.consume(inventory, route, authority)?;
            let target_id = format!("retire-{}", route.source_id);
            let replacement_summary_sha256 = replacement_summary_commitment(&replacement);
            let target = RetirementTarget {
                target_id: target_id.clone(),
                route_id: route.route_id.clone(),
                source_id: route.source_id.clone(),
                canonical_target_id: route.canonical_target_id.clone(),
                source_status: source.status,
                active_readers: source.active_readers.clone(),
                active_writers: source.active_writers.clone(),
                public_routes: source.public_routes.clone(),
                generated_outputs: source.generated_outputs.clone(),
                observed_invocations: route.observed_invocations,
                compatibility_window_complete: route.compatibility_window_complete,
                owner_id: route.owner_id.clone(),
                replacement_summary_sha256,
            };
            replacement_ledger.insert(target_id, replacement)?;
            targets.push(target);
        }
        if !evidence.is_empty() {
            return Err(MigrationError::new(
                "migration-replacement-evidence-unmatched",
            ));
        }
        targets.sort_by(|left, right| left.target_id.cmp(&right.target_id));
        let plan_sha256 = plan_digest(inventory, &routes, &targets);
        let plan = Self {
            live_context_id: inventory.live_context_id.clone(),
            candidate_id: inventory.candidate_id.clone(),
            catalog_id: inventory.catalog_id.clone(),
            read_session_id: inventory.read_session_id.clone(),
            inventory_sha256: inventory.inventory_sha256.clone(),
            plan_sha256,
            routes,
            targets,
            replacement_ledger,
        };
        for target in &plan.targets {
            let route = plan
                .routes
                .iter()
                .find(|route| route.route_id == target.route_id)
                .ok_or_else(|| MigrationError::new("migration-replacement-route-missing"))?;
            plan.replacement_evidence(target)
                .ok_or_else(|| {
                    MigrationError::new("migration-private-replacement-evidence-missing")
                })?
                .validate_with_authority(inventory, route, authority)?;
            let binding = ReplacementLedgerBinding::issue(&plan, target, inventory)?;
            authority.bind_plan_target(&binding)?;
            if !authority.verify_plan_target_consumed(&binding)
                || plan.replacement_evidence(target).is_none_or(|evidence| {
                    evidence
                        .validate_with_authority(inventory, route, authority)
                        .is_err()
                })
                || !authority.verify_plan_target_consumed(&binding)
            {
                return Err(MigrationError::new(
                    "migration-replacement-ledger-binding-refused",
                ));
            }
        }
        Ok(plan)
    }

    pub fn verify_current(&self, current: &MigrationInventory) -> Result<(), MigrationError> {
        current.validate()?;
        if self.live_context_id != current.live_context_id
            || self.candidate_id != current.candidate_id
            || self.catalog_id != current.catalog_id
            || self.read_session_id != current.read_session_id
            || self.inventory_sha256 != current.inventory_sha256
            || self.plan_sha256 != plan_digest(current, &self.routes, &self.targets)
            || !self.private_replacement_ledger_is_current()
        {
            return Err(MigrationError::new("migration-plan-stale"));
        }
        Ok(())
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn targets(&self) -> &[RetirementTarget] {
        &self.targets
    }

    /// Safe inspection DTO. It deliberately omits the private consumed
    /// replacement ledger and cannot be converted back into migration
    /// authority.
    pub fn projection(&self) -> MigrationPlanProjection {
        let targets = self
            .targets
            .iter()
            .map(RetirementTarget::projection)
            .collect::<Vec<_>>();
        let context_commitment_sha256 = digest(
            format!(
                "migration-plan-context-v1|{}|{}|{}|{}|{}",
                self.live_context_id,
                self.candidate_id,
                self.catalog_id,
                self.read_session_id,
                self.inventory_sha256,
            )
            .as_bytes(),
        );
        let projection_sha256 = migration_plan_projection_digest(
            &self.plan_sha256,
            &context_commitment_sha256,
            self.routes.len(),
            &targets,
        );
        MigrationPlanProjection {
            schema_version: "MigrationPlanProjection-v1".to_owned(),
            plan_sha256: self.plan_sha256.clone(),
            context_commitment_sha256,
            route_count: self.routes.len(),
            target_count: targets.len(),
            targets,
            projection_sha256,
        }
    }

    /// Confirms only that an inspection DTO is the current exact projection.
    /// It never imports or reconstructs authority from serialized bytes.
    pub fn verify_projection(
        &self,
        projection: &MigrationPlanProjection,
    ) -> Result<(), MigrationError> {
        if &self.projection() != projection {
            return Err(MigrationError::new("migration-plan-projection-substituted"));
        }
        Ok(())
    }

    fn replacement_evidence(
        &self,
        target: &RetirementTarget,
    ) -> Option<&ConsumedReplacementEvidence> {
        self.replacement_ledger
            .get(&target.target_id)
            .filter(|evidence| {
                evidence.route_id == target.route_id
                    && evidence.source_id == target.source_id
                    && evidence.canonical_target_id == target.canonical_target_id
                    && replacement_summary_commitment(evidence) == target.replacement_summary_sha256
            })
    }

    fn private_replacement_ledger_is_current(&self) -> bool {
        self.replacement_ledger.by_target_id.len() == self.targets.len()
            && self
                .targets
                .iter()
                .all(|target| self.replacement_evidence(target).is_some())
    }

    #[cfg(test)]
    pub(crate) fn substitute_replacement_consumption_for_test(
        &mut self,
        consumption_sha256: impl Into<String>,
    ) {
        if let Some(evidence) = self.replacement_ledger.by_target_id.values_mut().next() {
            evidence.consumption_sha256 = consumption_sha256.into();
        }
    }

    #[cfg(test)]
    pub(crate) fn inject_private_serialization_sentinels_for_test(&mut self) -> Vec<String> {
        self.replacement_ledger
            .by_target_id
            .values_mut()
            .next()
            .map(ConsumedReplacementEvidence::inject_serialization_sentinels_for_test)
            .unwrap_or_default()
    }
}

fn replacement_summary_commitment(evidence: &ConsumedReplacementEvidence) -> String {
    digest(
        format!(
            "replacement-plan-inspection-summary-v1|{}|{}|{}|{}",
            evidence.route_id,
            evidence.source_id,
            evidence.canonical_target_id,
            evidence.digest_fragment(),
        )
        .as_bytes(),
    )
}

fn migration_plan_projection_digest(
    plan_sha256: &str,
    context_commitment_sha256: &str,
    route_count: usize,
    targets: &[RetirementTargetProjection],
) -> String {
    let target_count = targets.len();
    let target_rows = targets
        .iter()
        .map(RetirementTargetProjection::digest_fragment)
        .collect::<Vec<_>>()
        .join("|");
    digest(
        format!(
            "migration-plan-projection-v1|{plan_sha256}|{context_commitment_sha256}|{route_count}|{target_count}|{target_rows}",
        )
        .as_bytes(),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Od009Decision {
    PreservePhysicalArtifact,
    RequestPhysicalDeletion,
}

pub(crate) trait RetirementReviewAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self) -> &str;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str);
    fn od009_decision(&self) -> Od009Decision;
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}

pub(crate) trait DestructiveEffectAuthority {
    fn authority_id(&self) -> &str;
    fn principal_id(&self) -> &str;
    fn session_id(&self) -> &str;
    fn nonce_sha256(&self) -> &str;
    fn issued_at_unix_ms(&self) -> u64;
    fn expires_at_unix_ms(&self) -> u64;
    fn now_unix_ms(&self) -> u64;
    fn current_binding(&self) -> (&str, &str, &str, &str);
    fn effect_scope_sha256(&self) -> &str;
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        authorization_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}

/// Opaque root-issued review of one exact plan and retirement target. It is
/// intentionally neither clonable nor deserializable and has no public
/// constructor.
#[derive(Eq, PartialEq)]
pub struct RetirementReview {
    reviewer_id: String,
    authority_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    read_session_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    effect_scope_sha256: String,
    od009_decision: Od009Decision,
    binding_sha256: String,
    review_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for RetirementReview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RetirementReview")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl RetirementReview {
    pub(crate) fn issue<A: RetirementReviewAuthority>(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id)
            .ok_or_else(|| MigrationError::new("migration-retirement-target-unknown"))?;
        let authority_id = authority.authority_id().to_owned();
        let reviewer_id = authority.reviewer_id().to_owned();
        let session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority.nonce_sha256().to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let now_unix_ms = authority.now_unix_ms();
        let od009_decision = authority.od009_decision();
        let (context, candidate, catalog, inventory) = authority.current_binding();
        let context = context.to_owned();
        let candidate = candidate.to_owned();
        let catalog = catalog.to_owned();
        let inventory = inventory.to_owned();
        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let effect_scope_sha256 = retirement_effect_scope(plan, target, current)?;

        if !valid_identifier(&authority_id)
            || !valid_identifier(&reviewer_id)
            || authority_id == reviewer_id
            || retirement_principal_conflicts(plan, target, &authority_id)
            || retirement_principal_conflicts(plan, target, &reviewer_id)
            || !valid_sha256(&session_id)
            || session_id == current.read_session_id
            || !valid_sha256(&nonce_sha256)
            || !valid_authority_window(issued_at_unix_ms, expires_at_unix_ms, now_unix_ms)
            || (context, candidate, catalog, inventory)
                != (
                    current.live_context_id.clone(),
                    current.candidate_id.clone(),
                    current.catalog_id.clone(),
                    current.inventory_sha256.clone(),
                )
        {
            return Err(MigrationError::new(
                "migration-retirement-review-issuance-refused",
            ));
        }

        let binding_sha256 = retirement_review_binding(
            plan,
            target,
            current,
            &reviewer_id,
            &authority_id,
            &session_id,
            &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            od009_decision,
            &effect_scope_sha256,
        );
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let review_id =
            digest(format!("retirement-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.reviewer_id() != reviewer_id
            || authority.session_id() != session_id
            || authority.nonce_sha256() != nonce_sha256
            || authority.issued_at_unix_ms() != issued_at_unix_ms
            || authority.expires_at_unix_ms() != expires_at_unix_ms
            || authority.od009_decision() != od009_decision
            || authority.current_binding()
                != (
                    current.live_context_id.as_str(),
                    current.candidate_id.as_str(),
                    current.catalog_id.as_str(),
                    current.inventory_sha256.as_str(),
                )
            || !valid_authority_window(
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority.now_unix_ms(),
            )
        {
            return Err(MigrationError::new(
                "migration-retirement-review-issuance-refused",
            ));
        }

        Ok(Self {
            reviewer_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            read_session_id: current.read_session_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            effect_scope_sha256,
            od009_decision,
            binding_sha256,
            review_id,
            attestation_sha256,
        })
    }

    pub(crate) fn effect_scope_sha256(&self) -> &str {
        &self.effect_scope_sha256
    }

    #[cfg(test)]
    pub(crate) fn substitute_plan_for_test(&mut self, plan_sha256: impl Into<String>) {
        self.plan_sha256 = plan_sha256.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_target_for_test(&mut self, target_id: impl Into<String>) {
        self.target_id = target_id.into();
    }
}

/// Opaque authorization for the one exact physical effect scope selected by
/// an authenticated OD-009 deletion review. This token cannot perform effects.
#[derive(Eq, PartialEq)]
pub struct DestructiveAuthorization {
    principal_id: String,
    authority_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    live_context_id: String,
    candidate_id: String,
    catalog_id: String,
    inventory_sha256: String,
    plan_sha256: String,
    target_id: String,
    target_sha256: String,
    review_id: String,
    effect_scope_sha256: String,
    binding_sha256: String,
    authorization_id: String,
    attestation_sha256: String,
}

impl fmt::Debug for DestructiveAuthorization {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DestructiveAuthorization")
            .field("contents", &"<redacted>")
            .finish()
    }
}

impl DestructiveAuthorization {
    pub(crate) fn issue<R: RetirementReviewAuthority, A: DestructiveEffectAuthority>(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        review_authority: &R,
        authority: &mut A,
    ) -> Result<Self, MigrationError> {
        plan.verify_current(current)?;
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id)
            .ok_or_else(|| MigrationError::new("migration-retirement-target-unknown"))?;
        if review.od009_decision != Od009Decision::RequestPhysicalDeletion
            || !retirement_review_is_current(plan, target, current, review, review_authority)
        {
            return Err(MigrationError::new(
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        let authority_id = authority.authority_id().to_owned();
        let principal_id = authority.principal_id().to_owned();
        let session_id = authority.session_id().to_owned();
        let nonce_sha256 = authority.nonce_sha256().to_owned();
        let issued_at_unix_ms = authority.issued_at_unix_ms();
        let expires_at_unix_ms = authority.expires_at_unix_ms();
        let now_unix_ms = authority.now_unix_ms();
        let effect_scope_sha256 = authority.effect_scope_sha256().to_owned();
        let (context, candidate, catalog, inventory) = authority.current_binding();
        let context = context.to_owned();
        let candidate = candidate.to_owned();
        let catalog = catalog.to_owned();
        let inventory = inventory.to_owned();

        if !valid_identifier(&authority_id)
            || !valid_identifier(&principal_id)
            || authority_id == principal_id
            || authority_id == review.authority_id
            || authority_id == review.reviewer_id
            || principal_id == review.authority_id
            || principal_id == review.reviewer_id
            || retirement_principal_conflicts(plan, target, &authority_id)
            || retirement_principal_conflicts(plan, target, &principal_id)
            || !valid_sha256(&session_id)
            || session_id == current.read_session_id
            || session_id == review.session_id
            || !valid_sha256(&nonce_sha256)
            || nonce_sha256 == review.nonce_sha256
            || !valid_authority_window(issued_at_unix_ms, expires_at_unix_ms, now_unix_ms)
            || effect_scope_sha256 != review.effect_scope_sha256
            || (context, candidate, catalog, inventory)
                != (
                    current.live_context_id.clone(),
                    current.candidate_id.clone(),
                    current.catalog_id.clone(),
                    current.inventory_sha256.clone(),
                )
        {
            return Err(MigrationError::new(
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        let target_sha256 = digest(target.digest_fragment().as_bytes());
        let binding_sha256 = destructive_authorization_binding(
            plan,
            target,
            current,
            review,
            &principal_id,
            &authority_id,
            &session_id,
            &nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            &effect_scope_sha256,
        );
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let authorization_id = digest(
            format!("destructive-authorization|{binding_sha256}|{attestation_sha256}").as_bytes(),
        );
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.principal_id() != principal_id
            || authority.session_id() != session_id
            || authority.nonce_sha256() != nonce_sha256
            || authority.issued_at_unix_ms() != issued_at_unix_ms
            || authority.expires_at_unix_ms() != expires_at_unix_ms
            || authority.effect_scope_sha256() != effect_scope_sha256
            || authority.current_binding()
                != (
                    current.live_context_id.as_str(),
                    current.candidate_id.as_str(),
                    current.catalog_id.as_str(),
                    current.inventory_sha256.as_str(),
                )
            || !valid_authority_window(
                issued_at_unix_ms,
                expires_at_unix_ms,
                authority.now_unix_ms(),
            )
        {
            return Err(MigrationError::new(
                "migration-destructive-authorization-issuance-refused",
            ));
        }

        Ok(Self {
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            catalog_id: current.catalog_id.clone(),
            inventory_sha256: current.inventory_sha256.clone(),
            plan_sha256: plan.plan_sha256.clone(),
            target_id: target.target_id.clone(),
            target_sha256,
            review_id: review.review_id.clone(),
            effect_scope_sha256,
            binding_sha256,
            authorization_id,
            attestation_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_effect_scope_for_test(&mut self, scope: impl Into<String>) {
        self.effect_scope_sha256 = scope.into();
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetirementStatus {
    Blocked,
    NonAuthoritativePreservationCandidate,
    DestructiveRetirementCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RetirementDecision {
    pub target_id: String,
    pub live_context_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub review_id: String,
    pub destructive_authorization_id: Option<String>,
    pub status: RetirementStatus,
    pub reasons: Vec<String>,
    pub plan_sha256: String,
    pub claim_ceiling: String,
}

impl RetirementDecision {
    pub(crate) fn reconcile_preservation<
        P: ReplacementEvidenceAuthority,
        R: RetirementReviewAuthority,
    >(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        replacement_authority: &mut P,
        review_authority: &mut R,
    ) -> Self {
        Self::reconcile_internal(
            plan,
            target_id,
            current,
            review,
            replacement_authority,
            review_authority,
            None,
        )
    }

    pub(crate) fn reconcile_destructive<
        P: ReplacementEvidenceAuthority,
        R: RetirementReviewAuthority,
        A: DestructiveEffectAuthority,
    >(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        authorization: &DestructiveAuthorization,
        replacement_authority: &mut P,
        review_authority: &mut R,
        effect_authority: &mut A,
    ) -> Self {
        Self::reconcile_internal(
            plan,
            target_id,
            current,
            review,
            replacement_authority,
            review_authority,
            Some((authorization, effect_authority)),
        )
    }

    fn reconcile_internal(
        plan: &MigrationPlan,
        target_id: &str,
        current: &MigrationInventory,
        review: &RetirementReview,
        replacement_authority: &mut dyn ReplacementEvidenceAuthority,
        review_authority: &mut dyn RetirementReviewAuthority,
        destructive: Option<(
            &DestructiveAuthorization,
            &mut dyn DestructiveEffectAuthority,
        )>,
    ) -> Self {
        let mut reasons = Vec::new();
        if plan.verify_current(current).is_err() {
            reasons.push("migration-plan-not-current".to_owned());
        }
        if !replacement_plan_ledger_is_current(plan, current, replacement_authority) {
            reasons.push("migration-replacement-ledger-not-current".to_owned());
        }
        let target = plan
            .targets
            .iter()
            .find(|target| target.target_id == target_id);
        if target.is_none() {
            reasons.push("migration-retirement-target-unknown".to_owned());
        }
        let review_is_current = target
            .map(|target| {
                retirement_review_is_current(plan, target, current, review, review_authority)
            })
            .unwrap_or(false);
        if !review_is_current {
            reasons.push("migration-retirement-review-invalid".to_owned());
        }
        if let Some(target) = target {
            let route = plan
                .routes
                .iter()
                .find(|route| route.route_id == target.route_id);
            let replacement = plan.replacement_evidence(target);
            let replacement_is_current = route.zip(replacement).is_some_and(|(route, evidence)| {
                evidence
                    .validate_with_authority(current, route, replacement_authority)
                    .is_ok()
            });
            if !replacement_is_current {
                reasons.push("migration-replacement-evidence-invalid".to_owned());
            }
            if !target.active_readers.is_empty() {
                reasons.push("migration-active-readers-remain".to_owned());
            }
            if !target.active_writers.is_empty() {
                reasons.push("migration-active-writers-remain".to_owned());
            }
            if !target.public_routes.is_empty() {
                reasons.push("migration-public-routes-remain".to_owned());
            }
            if !target.generated_outputs.is_empty() {
                reasons.push("migration-generated-authority-remains".to_owned());
            }
            if target.observed_invocations != 0 || !target.compatibility_window_complete {
                reasons.push("migration-compatibility-window-open".to_owned());
            }
            if target.source_status == SurfaceStatus::Retired {
                reasons.push("migration-target-already-retired".to_owned());
            }
            if route.zip(replacement).is_none_or(|(route, evidence)| {
                evidence.observation.validate(route).is_err()
                    || !valid_sha256(&target.replacement_summary_sha256)
                    || replacement_summary_commitment(evidence) != target.replacement_summary_sha256
            }) {
                reasons.push("migration-replacement-behavior-unverified".to_owned());
            }
        }

        let mut destructive_authorization_id = None;
        let mut destructive_to_consume = None;
        match (review.od009_decision, destructive) {
            (Od009Decision::PreservePhysicalArtifact, Some(_)) => {
                reasons.push("migration-conflicting-destructive-authorization".to_owned());
            }
            (Od009Decision::RequestPhysicalDeletion, None) => {
                reasons.push("migration-destructive-authority-required".to_owned());
            }
            (Od009Decision::RequestPhysicalDeletion, Some((authorization, authority))) => {
                let authorization_is_current = target
                    .map(|target| {
                        destructive_authorization_is_current(
                            plan,
                            target,
                            current,
                            review,
                            authorization,
                            review_authority,
                            authority,
                        )
                    })
                    .unwrap_or(false);
                if !authorization_is_current {
                    reasons.push("migration-destructive-authorization-invalid".to_owned());
                } else {
                    destructive_authorization_id = Some(authorization.authorization_id.clone());
                    destructive_to_consume = Some((authorization, authority));
                }
            }
            (Od009Decision::PreservePhysicalArtifact, None) => {}
        }

        if reasons.is_empty() {
            let ledger_binding = target
                .and_then(|target| ReplacementLedgerBinding::issue(plan, target, current).ok());
            let final_claim = ledger_binding.as_ref().and_then(|binding| {
                replacement_authority
                    .claim_final_reconciliation(binding)
                    .ok()
            });
            let ledger_is_current_before_review = match (&ledger_binding, &final_claim) {
                (Some(binding), Some(claim)) => {
                    valid_sha256(claim)
                        && replacement_plan_ledger_is_current(plan, current, replacement_authority)
                        && replacement_authority.verify_final_claim(binding, claim)
                }
                _ => false,
            };
            if !ledger_is_current_before_review {
                reasons.push("migration-replacement-ledger-invalid-or-replayed".to_owned());
            } else {
                let review_consumed = review_authority.verify_and_consume(
                    &review.binding_sha256,
                    &review.review_id,
                    &review.attestation_sha256,
                );
                let review_remained_current = target
                    .map(|target| {
                        retirement_review_is_current(
                            plan,
                            target,
                            current,
                            review,
                            review_authority,
                        )
                    })
                    .unwrap_or(false);
                let replacement_remained_current = match (&ledger_binding, &final_claim) {
                    (Some(binding), Some(claim)) => {
                        replacement_plan_ledger_is_current(plan, current, replacement_authority)
                            && replacement_authority.verify_final_claim(binding, claim)
                    }
                    _ => false,
                };
                if !replacement_remained_current {
                    reasons.push("migration-replacement-ledger-drifted".to_owned());
                }
                if !review_consumed || !review_remained_current {
                    reasons.push("migration-retirement-review-invalid-or-replayed".to_owned());
                } else if replacement_remained_current {
                    if let Some((authorization, authority)) = destructive_to_consume {
                        let replacement_is_current_before_effect =
                            match (&ledger_binding, &final_claim) {
                                (Some(binding), Some(claim)) => {
                                    replacement_plan_ledger_is_current(
                                        plan,
                                        current,
                                        replacement_authority,
                                    ) && replacement_authority.verify_final_claim(binding, claim)
                                }
                                _ => false,
                            };
                        if !replacement_is_current_before_effect {
                            reasons.push("migration-replacement-ledger-drifted".to_owned());
                        } else {
                            let authorization_consumed = authority.verify_and_consume(
                                &authorization.binding_sha256,
                                &authorization.authorization_id,
                                &authorization.attestation_sha256,
                            );
                            let authorization_remained_current = target
                                .map(|target| {
                                    destructive_authorization_is_current(
                                        plan,
                                        target,
                                        current,
                                        review,
                                        authorization,
                                        review_authority,
                                        authority,
                                    )
                                })
                                .unwrap_or(false);
                            let replacement_remained_current = match (&ledger_binding, &final_claim)
                            {
                                (Some(binding), Some(claim)) => {
                                    replacement_plan_ledger_is_current(
                                        plan,
                                        current,
                                        replacement_authority,
                                    ) && replacement_authority.verify_final_claim(binding, claim)
                                }
                                _ => false,
                            };
                            if !replacement_remained_current {
                                reasons.push("migration-replacement-ledger-drifted".to_owned());
                            }
                            if !authorization_consumed || !authorization_remained_current {
                                reasons.push(
                                    "migration-destructive-authorization-invalid-or-replayed"
                                        .to_owned(),
                                );
                            }
                        }
                    }
                }
            }
        }
        reasons.sort();
        reasons.dedup();
        let status = if !reasons.is_empty() {
            RetirementStatus::Blocked
        } else if review.od009_decision == Od009Decision::RequestPhysicalDeletion {
            RetirementStatus::DestructiveRetirementCandidate
        } else {
            RetirementStatus::NonAuthoritativePreservationCandidate
        };
        Self {
            target_id: target_id.to_owned(),
            live_context_id: current.live_context_id.clone(),
            candidate_id: current.candidate_id.clone(),
            reviewer_id: review.reviewer_id.clone(),
            review_id: review.review_id.clone(),
            destructive_authorization_id,
            status,
            reasons,
            plan_sha256: plan.plan_sha256.clone(),
            claim_ceiling: "migration_candidate_not_adoption_or_retirement".to_owned(),
        }
    }
}

fn replacement_plan_ledger_is_current(
    plan: &MigrationPlan,
    current: &MigrationInventory,
    authority: &mut dyn ReplacementEvidenceAuthority,
) -> bool {
    if plan.verify_current(current).is_err() {
        return false;
    }
    for target in &plan.targets {
        let Some(route) = plan
            .routes
            .iter()
            .find(|route| route.route_id == target.route_id)
        else {
            return false;
        };
        let Some(evidence) = plan.replacement_evidence(target) else {
            return false;
        };
        if evidence
            .validate_with_authority(current, route, authority)
            .is_err()
        {
            return false;
        }
        let Ok(binding) = ReplacementLedgerBinding::issue(plan, target, current) else {
            return false;
        };
        if !authority.verify_plan_target_consumed(&binding)
            || evidence
                .validate_with_authority(current, route, authority)
                .is_err()
        {
            return false;
        }
    }
    true
}

fn valid_authority_window(issued_at: u64, expires_at: u64, now: u64) -> bool {
    issued_at <= now
        && now <= expires_at
        && issued_at < expires_at
        && expires_at.saturating_sub(issued_at) <= MAX_AUTHORIZATION_TTL_MS
}

fn retirement_principal_conflicts(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    principal_id: &str,
) -> bool {
    principal_id == target.owner_id
        || plan
            .replacement_evidence(target)
            .is_some_and(|evidence| principal_id == evidence.reviewer_id)
        || target
            .active_readers
            .iter()
            .chain(&target.active_writers)
            .any(|principal| principal == principal_id)
}

fn retirement_effect_scope(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
) -> Result<String, MigrationError> {
    let source = current
        .surfaces
        .iter()
        .find(|surface| surface.stable_id == target.source_id)
        .ok_or_else(|| MigrationError::new("migration-retirement-source-unknown"))?;
    Ok(digest(
        format!(
            "od-009-retirement-effect-v1|{}|{}|{}|{}|{}|{}|{}",
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.digest_fragment(),
            source.digest_fragment(),
        )
        .as_bytes(),
    ))
}

#[allow(clippy::too_many_arguments)]
fn retirement_review_binding(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
    reviewer_id: &str,
    authority_id: &str,
    session_id: &str,
    nonce_sha256: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    decision: Od009Decision,
    effect_scope_sha256: &str,
) -> String {
    digest(
        format!(
            "retirement-review-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{}",
            reviewer_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.read_session_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.target_id,
            target.digest_fragment(),
            decision,
            effect_scope_sha256,
        )
        .as_bytes(),
    )
}

fn retirement_review_is_current(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
    review: &RetirementReview,
    authority: &dyn RetirementReviewAuthority,
) -> bool {
    let Ok(effect_scope_sha256) = retirement_effect_scope(plan, target, current) else {
        return false;
    };
    let expected_binding = retirement_review_binding(
        plan,
        target,
        current,
        &review.reviewer_id,
        &review.authority_id,
        &review.session_id,
        &review.nonce_sha256,
        review.issued_at_unix_ms,
        review.expires_at_unix_ms,
        review.od009_decision,
        &effect_scope_sha256,
    );
    let expected_review_id = digest(
        format!(
            "retirement-review|{}|{}",
            expected_binding, review.attestation_sha256
        )
        .as_bytes(),
    );
    valid_identifier(&review.reviewer_id)
        && valid_identifier(&review.authority_id)
        && review.reviewer_id != review.authority_id
        && !retirement_principal_conflicts(plan, target, &review.reviewer_id)
        && !retirement_principal_conflicts(plan, target, &review.authority_id)
        && valid_sha256(&review.session_id)
        && review.session_id != current.read_session_id
        && valid_sha256(&review.nonce_sha256)
        && valid_authority_window(
            review.issued_at_unix_ms,
            review.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && plan.verify_current(current).is_ok()
        && review.live_context_id == current.live_context_id
        && review.candidate_id == current.candidate_id
        && review.catalog_id == current.catalog_id
        && review.read_session_id == current.read_session_id
        && review.inventory_sha256 == current.inventory_sha256
        && review.plan_sha256 == plan.plan_sha256
        && review.target_id == target.target_id
        && review.target_sha256 == digest(target.digest_fragment().as_bytes())
        && review.effect_scope_sha256 == effect_scope_sha256
        && valid_sha256(&review.attestation_sha256)
        && review.binding_sha256 == expected_binding
        && review.review_id == expected_review_id
        && authority.authority_id() == review.authority_id
        && authority.reviewer_id() == review.reviewer_id
        && authority.session_id() == review.session_id
        && authority.nonce_sha256() == review.nonce_sha256
        && authority.issued_at_unix_ms() == review.issued_at_unix_ms
        && authority.expires_at_unix_ms() == review.expires_at_unix_ms
        && authority.od009_decision() == review.od009_decision
        && authority.current_binding()
            == (
                current.live_context_id.as_str(),
                current.candidate_id.as_str(),
                current.catalog_id.as_str(),
                current.inventory_sha256.as_str(),
            )
}

#[allow(clippy::too_many_arguments)]
fn destructive_authorization_binding(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
    review: &RetirementReview,
    principal_id: &str,
    authority_id: &str,
    session_id: &str,
    nonce_sha256: &str,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
    effect_scope_sha256: &str,
) -> String {
    digest(
        format!(
            "destructive-authorization-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            principal_id,
            authority_id,
            session_id,
            nonce_sha256,
            issued_at_unix_ms,
            expires_at_unix_ms,
            current.live_context_id,
            current.candidate_id,
            current.catalog_id,
            current.inventory_sha256,
            plan.plan_sha256,
            target.target_id,
            target.digest_fragment(),
            review.review_id,
            effect_scope_sha256,
        )
        .as_bytes(),
    )
}

#[allow(clippy::too_many_arguments)]
fn destructive_authorization_is_current(
    plan: &MigrationPlan,
    target: &RetirementTarget,
    current: &MigrationInventory,
    review: &RetirementReview,
    authorization: &DestructiveAuthorization,
    review_authority: &dyn RetirementReviewAuthority,
    authority: &dyn DestructiveEffectAuthority,
) -> bool {
    if !retirement_review_is_current(plan, target, current, review, review_authority)
        || review.od009_decision != Od009Decision::RequestPhysicalDeletion
    {
        return false;
    }
    let Ok(effect_scope_sha256) = retirement_effect_scope(plan, target, current) else {
        return false;
    };
    let expected_binding = destructive_authorization_binding(
        plan,
        target,
        current,
        review,
        &authorization.principal_id,
        &authorization.authority_id,
        &authorization.session_id,
        &authorization.nonce_sha256,
        authorization.issued_at_unix_ms,
        authorization.expires_at_unix_ms,
        &effect_scope_sha256,
    );
    let expected_authorization_id = digest(
        format!(
            "destructive-authorization|{}|{}",
            expected_binding, authorization.attestation_sha256
        )
        .as_bytes(),
    );
    valid_identifier(&authorization.principal_id)
        && valid_identifier(&authorization.authority_id)
        && authorization.principal_id != authorization.authority_id
        && authorization.principal_id != review.reviewer_id
        && authorization.principal_id != review.authority_id
        && authorization.authority_id != review.reviewer_id
        && authorization.authority_id != review.authority_id
        && !retirement_principal_conflicts(plan, target, &authorization.principal_id)
        && !retirement_principal_conflicts(plan, target, &authorization.authority_id)
        && valid_sha256(&authorization.session_id)
        && authorization.session_id != current.read_session_id
        && authorization.session_id != review.session_id
        && valid_sha256(&authorization.nonce_sha256)
        && authorization.nonce_sha256 != review.nonce_sha256
        && valid_authority_window(
            authorization.issued_at_unix_ms,
            authorization.expires_at_unix_ms,
            authority.now_unix_ms(),
        )
        && authorization.live_context_id == current.live_context_id
        && authorization.candidate_id == current.candidate_id
        && authorization.catalog_id == current.catalog_id
        && authorization.inventory_sha256 == current.inventory_sha256
        && authorization.plan_sha256 == plan.plan_sha256
        && authorization.target_id == target.target_id
        && authorization.target_sha256 == digest(target.digest_fragment().as_bytes())
        && authorization.review_id == review.review_id
        && authorization.effect_scope_sha256 == effect_scope_sha256
        && valid_sha256(&authorization.attestation_sha256)
        && authorization.binding_sha256 == expected_binding
        && authorization.authorization_id == expected_authorization_id
        && authority.authority_id() == authorization.authority_id
        && authority.principal_id() == authorization.principal_id
        && authority.session_id() == authorization.session_id
        && authority.nonce_sha256() == authorization.nonce_sha256
        && authority.issued_at_unix_ms() == authorization.issued_at_unix_ms
        && authority.expires_at_unix_ms() == authorization.expires_at_unix_ms
        && authority.effect_scope_sha256() == authorization.effect_scope_sha256
        && authority.current_binding()
            == (
                current.live_context_id.as_str(),
                current.candidate_id.as_str(),
                current.catalog_id.as_str(),
                current.inventory_sha256.as_str(),
            )
}

fn inventory_digest(
    context: &str,
    candidate: &str,
    catalog: &str,
    session: &str,
    surfaces: &[InventorySurface],
) -> String {
    let rows = surfaces
        .iter()
        .map(InventorySurface::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(format!("{context}|{candidate}|{catalog}|{session}|{rows}").as_bytes())
}

fn plan_digest(
    inventory: &MigrationInventory,
    routes: &[CompatibilityRoute],
    targets: &[RetirementTarget],
) -> String {
    let route_rows = routes
        .iter()
        .map(CompatibilityRoute::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    let target_rows = targets
        .iter()
        .map(RetirementTarget::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!(
            "{}|{}|{}",
            inventory.inventory_sha256, route_rows, target_rows
        )
        .as_bytes(),
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn safe_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains("../")
        && !value.contains("/..")
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn normalized(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}
