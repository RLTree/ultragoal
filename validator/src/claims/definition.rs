use super::evidence::{ClaimObligation, ObligationKind};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ADOPTED_CLAIM_COUNT: usize = 14;
pub const ADOPTED_CLAIM_REGISTRY_SHA256: &str =
    "67e81c4eabe87d16d816a3d7dad352dc21a4a1994dd83b6582e9e4ba5eaa61bc";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDefinition {
    pub claim_id: String,
    pub name: String,
    pub expected_behavior: String,
    pub truth_surface: String,
    pub prerequisite_claim_ids: Vec<String>,
    pub requirement_ids: Vec<String>,
    pub required_surface_ids: Vec<String>,
    pub required_tool_ids: Vec<String>,
    pub required_decision_ids: Vec<String>,
    pub required_evidence: Vec<String>,
    pub current_live_evidence: Vec<String>,
    pub current_live_evidence_status: String,
    pub independent_reconciler: String,
    pub false_pass_controls: Vec<String>,
    pub claim_guard: String,
    pub repair: String,
    pub rerun: String,
    pub allowed_ceiling_on_pass: String,
    pub initial_claim_state: String,
    pub claim_decision_owner: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    contract_id: String,
    separation_rule: String,
    claim_topological_order: Vec<String>,
    claim_count: usize,
    claims: Vec<ClaimDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimDefinitions {
    registry_digest: String,
    contract_id: String,
    order: Vec<String>,
    by_id: BTreeMap<String, ClaimDefinition>,
}

impl ClaimDefinitions {
    /// Adopt frozen registry bytes. This only establishes definition authority.
    pub fn adopt(registry_bytes: &[u8], expected_sha256: &str) -> Result<Self, String> {
        let actual = format!("{:x}", Sha256::digest(registry_bytes));
        if actual != expected_sha256 || actual != ADOPTED_CLAIM_REGISTRY_SHA256 {
            return Err("claims-registry-digest-mismatch".to_owned());
        }
        let registry: Registry = serde_json::from_slice(registry_bytes)
            .map_err(|_| "claims-registry-malformed".to_owned())?;
        if registry.schema_version != "2.0.0" || registry.contract_id.is_empty() {
            return Err("claims-registry-identity-invalid".to_owned());
        }
        if registry.separation_rule.is_empty()
            || registry.claim_count != ADOPTED_CLAIM_COUNT
            || registry.claims.len() != ADOPTED_CLAIM_COUNT
            || registry.claim_topological_order.len() != ADOPTED_CLAIM_COUNT
        {
            return Err("claims-registry-count-invalid".to_owned());
        }
        let mut by_id = BTreeMap::new();
        for definition in registry.claims {
            validate_definition(&definition)?;
            if by_id
                .insert(definition.claim_id.clone(), definition)
                .is_some()
            {
                return Err("claims-registry-duplicate-claim".to_owned());
            }
        }
        let order_set = registry
            .claim_topological_order
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if order_set.len() != ADOPTED_CLAIM_COUNT || order_set != by_id.keys().cloned().collect() {
            return Err("claims-registry-order-membership-invalid".to_owned());
        }
        for (index, claim_id) in registry.claim_topological_order.iter().enumerate() {
            let definition = by_id
                .get(claim_id)
                .ok_or_else(|| "claims-registry-order-unknown-claim".to_owned())?;
            for prerequisite in &definition.prerequisite_claim_ids {
                let prerequisite_index = registry
                    .claim_topological_order
                    .iter()
                    .position(|candidate| candidate == prerequisite)
                    .ok_or_else(|| "claims-registry-prerequisite-unknown".to_owned())?;
                if prerequisite_index >= index {
                    return Err("claims-registry-cycle-or-order-invalid".to_owned());
                }
            }
        }
        Ok(Self {
            registry_digest: actual,
            contract_id: registry.contract_id,
            order: registry.claim_topological_order,
            by_id,
        })
    }

    pub fn definition(&self, claim_id: &str) -> Option<&ClaimDefinition> {
        self.by_id.get(claim_id)
    }
    pub fn order(&self) -> &[String] {
        &self.order
    }
    pub fn registry_digest(&self) -> &str {
        &self.registry_digest
    }
    pub fn contract_id(&self) -> &str {
        &self.contract_id
    }
    pub fn dependent_closure(&self, claim_id: &str) -> Result<BTreeSet<String>, String> {
        if !self.by_id.contains_key(claim_id) {
            return Err("claims-unknown-claim".to_owned());
        }
        let mut invalidated = BTreeSet::from([claim_id.to_owned()]);
        loop {
            let before = invalidated.len();
            for definition in self.by_id.values() {
                if definition
                    .prerequisite_claim_ids
                    .iter()
                    .any(|item| invalidated.contains(item))
                {
                    invalidated.insert(definition.claim_id.clone());
                }
            }
            if invalidated.len() == before {
                return Ok(invalidated);
            }
        }
    }
}

impl ClaimDefinition {
    pub fn required_obligations(&self) -> BTreeSet<ClaimObligation> {
        let mut obligations = BTreeSet::new();
        add_obligations(
            &mut obligations,
            ObligationKind::RequiredEvidence,
            &self.required_evidence,
        );
        add_obligations(
            &mut obligations,
            ObligationKind::RequiredSurface,
            &self.required_surface_ids,
        );
        add_obligations(
            &mut obligations,
            ObligationKind::RequiredTool,
            &self.required_tool_ids,
        );
        add_obligations(
            &mut obligations,
            ObligationKind::RequiredDecision,
            &self.required_decision_ids,
        );
        add_obligations(
            &mut obligations,
            ObligationKind::FalsePassControl,
            &self.false_pass_controls,
        );
        obligations
    }
}

fn add_obligations(
    obligations: &mut BTreeSet<ClaimObligation>,
    kind: ObligationKind,
    values: &[String],
) {
    obligations.extend(values.iter().map(|id| ClaimObligation {
        kind: kind.clone(),
        id: id.clone(),
    }));
}

fn validate_definition(definition: &ClaimDefinition) -> Result<(), String> {
    if definition.claim_id.is_empty()
        || definition.truth_surface.is_empty()
        || definition.independent_reconciler.is_empty()
        || definition.allowed_ceiling_on_pass.is_empty()
        || definition.claim_guard.is_empty()
        || definition.required_evidence.is_empty()
        || definition.required_surface_ids.is_empty()
        || definition.required_tool_ids.is_empty()
        || definition.false_pass_controls.is_empty()
        || definition.current_live_evidence_status != "not_verified"
        || !definition.current_live_evidence.is_empty()
        || !unique_nonempty(&definition.required_evidence)
        || !unique_nonempty(&definition.required_surface_ids)
        || !unique_nonempty(&definition.required_tool_ids)
        || !unique_nonempty(&definition.required_decision_ids)
        || !unique_nonempty(&definition.false_pass_controls)
    {
        return Err("claims-registry-definition-invalid".to_owned());
    }
    Ok(())
}

fn unique_nonempty(values: &[String]) -> bool {
    values.iter().all(|value| !value.is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
