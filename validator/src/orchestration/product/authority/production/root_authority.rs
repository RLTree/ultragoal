use super::super::super::{
    AUTHORITY_DOMAIN, AUTHORITY_SCHEMA, PermitDecisionBinding, PermitTarget, RootOperation,
    RootPermit,
};
use super::{Ledger, ProductError, ProductionRootAuthority};
use crate::orchestration::{Actor, Binding, EffectResolution};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};

pub(crate) struct RootAuthority {
    root_actor: Actor,
    key: [u8; 32],
}

impl Debug for RootAuthority {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RootAuthority")
            .field("root_actor", &self.root_actor.as_str())
            .field("key", &"[redacted]")
            .finish()
    }
}

impl RootAuthority {
    pub(super) fn root_actor(&self) -> &Actor {
        &self.root_actor
    }
}

impl ProductionRootAuthority {
    pub(super) fn new(root_actor: Actor, key: [u8; 32], ledger: Ledger) -> Self {
        Self {
            authority: RootAuthority { root_actor, key },
            ledger,
        }
    }
}

include!("../root/secret_binding.rs");
include!("../root/verification.rs");

#[cfg(test)]
include!("root_authority_lifetime_tests.rs");
