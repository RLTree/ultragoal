impl ConsumedReplacementEvidence {
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
