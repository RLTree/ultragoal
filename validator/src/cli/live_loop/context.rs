use super::nodes::timing::{self, NodeTiming};
use super::{changed_inputs::ChangedInputs, graph};
use crate::cli::live_loop::LiveLoopCommand;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct AuditContext {
    pub(crate) candidate_digest: String,
    pub(crate) cache_mode: String,
    pub(crate) changed_files_digest: String,
    pub(crate) input_digest: String,
    changed_inputs: ChangedInputs,
    node_timings: BTreeMap<String, NodeTiming>,
    package_digest_baseline_ms: Option<u64>,
    package_truth: PackageTruthSnapshot,
    tier: String,
}

pub(crate) struct PackageTruthSnapshot {
    pub(crate) package_digest: String,
    pub(crate) package_inventory_digest: String,
    pub(crate) package_resource_count: usize,
    pub(crate) builder_contract_resource_count: usize,
    pub(crate) builder_contract_exclusion_status: &'static str,
}

impl AuditContext {
    pub(crate) fn new(root: &Path, candidate_digest: String, command: &LiveLoopCommand) -> Self {
        let inputs =
            ChangedInputs::collect(root, &candidate_digest, &command.tier, &command.cache_mode);
        let changed_files_digest = inputs.changed_files_digest.clone();
        let input_digest = inputs.audit_context_digest.clone();
        let package_digest_baseline_ms = package_digest_baseline_ms(root, &candidate_digest);
        let node_timings = timing::read_current(
            root,
            &candidate_digest,
            &command.tier,
            &command.cache_mode,
            &inputs,
        );
        let package_truth = PackageTruthSnapshot::new(root, &candidate_digest);
        Self {
            candidate_digest,
            tier: command.tier.clone(),
            cache_mode: command.cache_mode.clone(),
            changed_files_digest,
            input_digest,
            changed_inputs: inputs,
            node_timings,
            package_digest_baseline_ms,
            package_truth,
        }
    }

    pub(crate) fn tasks(&self) -> Vec<Box<dyn FnOnce() -> Value + Send + 'static>> {
        graph::tasks(
            &self.candidate_digest,
            &self.changed_files_digest,
            &self.input_digest,
            &self.tier,
            &self.cache_mode,
            self.package_digest_baseline_ms,
            &self.node_timings,
            &self.changed_inputs,
        )
    }

    pub(crate) fn changed_inputs(&self) -> &ChangedInputs {
        &self.changed_inputs
    }

    pub(crate) fn changed_input_summary(&self) -> Value {
        self.changed_inputs.summary()
    }

    pub(crate) fn package_truth_summary(&self) -> Value {
        self.package_truth.summary()
    }

    pub(crate) fn verify_package_truth_current(&self, root: &Path) -> Result<(), String> {
        self.package_truth.verify_current(root)
    }
}

pub(crate) fn verify_cache_hit(expected_key: &str, observed_key: &str) -> &'static str {
    if expected_key == observed_key {
        "pass"
    } else {
        "fail_stale_or_wrong_digest_cache_hit"
    }
}

impl PackageTruthSnapshot {
    pub(super) fn new(root: &Path, package_digest: &str) -> Self {
        let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))
            .unwrap_or_else(|_| json!({}));
        let mut package_resources = Vec::new();
        let mut builder_contract_resource_count = 0usize;
        for rel in crate::package::inventory::inventory_paths(&manifest) {
            if crate::package::inventory::builder_contract_resource_path(&rel) {
                builder_contract_resource_count += 1;
            } else if !crate::package::inventory::package_digest_excluded(&rel) {
                package_resources.push(rel);
            }
        }
        package_resources.sort();
        let package_inventory_digest =
            crate::digest::bytes(package_resources.join("\n").as_bytes());
        Self {
            package_digest: package_digest.to_string(),
            package_inventory_digest,
            package_resource_count: package_resources.len(),
            builder_contract_resource_count,
            builder_contract_exclusion_status: if builder_contract_resource_count == 0 {
                "not_present_in_package_inventory"
            } else {
                "excluded_from_package_truth"
            },
        }
    }

    pub(super) fn summary(&self) -> Value {
        json!({
            "package_digest": self.package_digest,
            "package_inventory_digest": self.package_inventory_digest,
            "package_resource_count": self.package_resource_count,
            "builder_contract_resource_count": self.builder_contract_resource_count,
            "builder_contract_exclusion_status": self.builder_contract_exclusion_status,
            "snapshot_authority": "AuditContext.package_truth_snapshot",
            "claim_limit": "source_package_truth_only_not_install_cache_registry_or_readiness"
        })
    }

    pub(super) fn verify_current(&self, root: &Path) -> Result<(), String> {
        let current = crate::package::inventory::package_digest(root)?;
        if current == self.package_digest {
            Ok(())
        } else {
            Err(format!(
                "AuditContext package truth snapshot mutated after creation: snapshot={} current={} action=fail_closed_start_new_snapshot",
                self.package_digest, current
            ))
        }
    }
}

pub(super) fn package_digest_baseline_ms(root: &Path, candidate_digest: &str) -> Option<u64> {
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/package-digest.json"),
    )
    .ok()?;
    let receipt_candidate = receipt.get("candidate_digest")?.as_str()?;
    if receipt_candidate != candidate_digest {
        return None;
    }
    receipt
        .pointer("/event/duration_ms")
        .and_then(serde_json::Value::as_u64)
}
