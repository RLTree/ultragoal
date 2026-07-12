use super::capability_gate::{HostEffectCapabilityGate, same_gate};
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::issuance::{SessionIssuance, same_issuance};
use super::scope::{BoundHostScope, same_scope};
use crate::distribution::{HostCommandPlan, JourneyBinding, MarketplaceScope, PackageIdentity};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecyclePlan};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostScopeAuthority {
    Personal {
        marketplace: String,
    },
    Repository {
        repository_root: String,
        marketplace: String,
    },
}

impl HostScopeAuthority {
    pub const fn marketplace_scope(&self) -> MarketplaceScope {
        match self {
            Self::Personal { .. } => MarketplaceScope::Personal,
            Self::Repository { .. } => MarketplaceScope::Repository,
        }
    }

    pub fn marketplace(&self) -> &str {
        match self {
            Self::Personal { marketplace } | Self::Repository { marketplace, .. } => marketplace,
        }
    }

    pub const fn accepts_marketplace_scope(&self, scope: MarketplaceScope) -> bool {
        matches!(
            (self, scope),
            (Self::Personal { .. }, MarketplaceScope::Personal)
                | (Self::Repository { .. }, MarketplaceScope::Repository)
        )
    }
}

#[derive(Serialize)]
pub struct ExternalHostEffectRequest {
    intent: LifecycleIntent,
    binding_sha256: String,
    session_issuance_sha256: String,
    host_scope_sha256: String,
    capability_gate_sha256: String,
    lifecycle_plan_id: String,
    plan_sha256: String,
    request_sha256: String,
    #[serde(skip)]
    plan: HostCommandPlan,
    #[serde(skip)]
    issuance: Arc<SessionIssuance>,
    #[serde(skip)]
    capability_gate: Arc<HostEffectCapabilityGate>,
    #[serde(skip)]
    host_scope: Arc<BoundHostScope>,
}

pub struct PreparedExternalHostEffect {
    intent: LifecycleIntent,
    binding_sha256: String,
    session_issuance_sha256: String,
    host_scope_sha256: String,
    capability_gate_sha256: String,
    request_sha256: String,
    plan: HostCommandPlan,
    issuance: Arc<SessionIssuance>,
    capability_gate: Arc<HostEffectCapabilityGate>,
    host_scope: Arc<BoundHostScope>,
}

impl ExternalHostEffectRequest {
    pub(super) fn for_intent(
        package: &PackageIdentity,
        binding: &JourneyBinding,
        lifecycle: &LifecyclePlan,
        issuance: Arc<SessionIssuance>,
        capability_gate: Arc<HostEffectCapabilityGate>,
        host_scope: Arc<BoundHostScope>,
        intent: LifecycleIntent,
    ) -> Result<Option<Self>, HostLifecycleError> {
        if matches!(
            intent,
            LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
        ) {
            return Ok(None);
        }
        if issuance.lifecycle() != lifecycle
            || issuance.binding_sha256() != binding.binding_sha256()
            || capability_gate.intent() != intent
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::InvalidBinding,
            ));
        }
        let scope = host_scope.authority();
        let remove = intent == LifecycleIntent::UninstallTeardown;
        let plan = match (scope, remove) {
            (HostScopeAuthority::Personal { marketplace }, false) => {
                HostCommandPlan::personal_install(package, marketplace)
            }
            (HostScopeAuthority::Personal { marketplace }, true) => {
                HostCommandPlan::personal_remove(package, marketplace)
            }
            (
                HostScopeAuthority::Repository {
                    repository_root,
                    marketplace,
                },
                false,
            ) => HostCommandPlan::repository_install(package, repository_root, marketplace),
            (HostScopeAuthority::Repository { marketplace, .. }, true) => {
                HostCommandPlan::repository_remove(package, marketplace)
            }
        }
        .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding))?;
        let request_sha256 = digest_request(
            intent,
            binding,
            lifecycle,
            &issuance,
            &capability_gate,
            &host_scope,
            &plan,
        );
        Ok(Some(Self {
            intent,
            binding_sha256: binding.binding_sha256().to_owned(),
            session_issuance_sha256: issuance.issuance_sha256().to_owned(),
            host_scope_sha256: host_scope.scope_sha256().to_owned(),
            capability_gate_sha256: capability_gate.gate_sha256().to_owned(),
            lifecycle_plan_id: lifecycle.plan_id.clone(),
            plan_sha256: plan.plan_sha256().to_owned(),
            request_sha256,
            plan,
            issuance,
            capability_gate,
            host_scope,
        }))
    }

    pub const fn intent(&self) -> LifecycleIntent {
        self.intent
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn lifecycle_plan_id(&self) -> &str {
        &self.lifecycle_plan_id
    }

    pub fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub fn capability_gate_sha256(&self) -> &str {
        &self.capability_gate_sha256
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub(super) fn consume_for(
        self,
        expected: &Arc<SessionIssuance>,
        expected_gate: &Arc<HostEffectCapabilityGate>,
        expected_scope: &Arc<BoundHostScope>,
    ) -> Result<PreparedExternalHostEffect, HostLifecycleError> {
        if !same_issuance(&self.issuance, expected)
            || !same_gate(&self.capability_gate, expected_gate)
            || !same_scope(&self.host_scope, expected_scope)
            || self.binding_sha256 != expected.binding_sha256()
            || self.session_issuance_sha256 != expected.issuance_sha256()
            || self.lifecycle_plan_id != expected.lifecycle().plan_id
            || self.capability_gate_sha256 != expected_gate.gate_sha256()
            || self.host_scope_sha256 != expected_scope.scope_sha256()
        {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::ExternalEffectSessionMismatch,
            ));
        }
        if let Err(error) = expected_scope.revalidate() {
            expected.reject();
            return Err(error);
        }
        if let Err(error) = expected_gate.require_supported() {
            expected.reject();
            return Err(error);
        }
        expected.consume_request()?;
        Ok(PreparedExternalHostEffect {
            intent: self.intent,
            binding_sha256: self.binding_sha256,
            session_issuance_sha256: self.session_issuance_sha256,
            host_scope_sha256: self.host_scope_sha256,
            capability_gate_sha256: self.capability_gate_sha256,
            request_sha256: self.request_sha256,
            plan: self.plan,
            issuance: self.issuance,
            capability_gate: self.capability_gate,
            host_scope: self.host_scope,
        })
    }
}

impl std::fmt::Debug for ExternalHostEffectRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExternalHostEffectRequest")
            .field("intent", &self.intent)
            .field("binding_sha256", &self.binding_sha256)
            .field("session_issuance_sha256", &self.session_issuance_sha256)
            .field("host_scope_sha256", &self.host_scope_sha256)
            .field("capability_gate_sha256", &self.capability_gate_sha256)
            .field("lifecycle_plan_id", &self.lifecycle_plan_id)
            .field("plan_sha256", &self.plan_sha256)
            .field("request_sha256", &self.request_sha256)
            .finish_non_exhaustive()
    }
}

impl PreparedExternalHostEffect {
    pub const fn intent(&self) -> LifecycleIntent {
        self.intent
    }

    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    pub fn request_sha256(&self) -> &str {
        &self.request_sha256
    }

    pub fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    pub fn capability_gate_sha256(&self) -> &str {
        &self.capability_gate_sha256
    }

    pub fn into_plan(self) -> Result<HostCommandPlan, HostLifecycleError> {
        if let Err(error) = self.host_scope.revalidate() {
            self.issuance.reject();
            return Err(error);
        }
        if let Err(error) = self.capability_gate.require_supported() {
            self.issuance.reject();
            return Err(error);
        }
        self.issuance.release_plan()?;
        Ok(self.plan)
    }
}

impl std::fmt::Debug for PreparedExternalHostEffect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedExternalHostEffect")
            .field("intent", &self.intent)
            .field("binding_sha256", &self.binding_sha256)
            .field("session_issuance_sha256", &self.session_issuance_sha256)
            .field("host_scope_sha256", &self.host_scope_sha256)
            .field("capability_gate_sha256", &self.capability_gate_sha256)
            .field("request_sha256", &self.request_sha256)
            .field("plan_sha256", &self.plan.plan_sha256())
            .finish_non_exhaustive()
    }
}

fn digest_request(
    intent: LifecycleIntent,
    binding: &JourneyBinding,
    lifecycle: &LifecyclePlan,
    issuance: &SessionIssuance,
    capability_gate: &HostEffectCapabilityGate,
    host_scope: &BoundHostScope,
    plan: &HostCommandPlan,
) -> String {
    #[derive(Serialize)]
    struct Request<'a> {
        schema: &'static str,
        intent: LifecycleIntent,
        binding_sha256: &'a str,
        session_issuance_sha256: &'a str,
        host_scope_sha256: &'a str,
        capability_gate_sha256: &'a str,
        lifecycle_plan_id: &'a str,
        lifecycle_authorization_sha256: &'a str,
        plan_sha256: &'a str,
    }
    let bytes = serde_json::to_vec(&Request {
        schema: "harness-ultragoal.external-host-effect-request.v1",
        intent,
        binding_sha256: binding.binding_sha256(),
        session_issuance_sha256: issuance.issuance_sha256(),
        host_scope_sha256: host_scope.scope_sha256(),
        capability_gate_sha256: capability_gate.gate_sha256(),
        lifecycle_plan_id: &lifecycle.plan_id,
        lifecycle_authorization_sha256: &lifecycle.authorization_sha256,
        plan_sha256: plan.plan_sha256(),
    })
    .expect("external host effect request is serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}
