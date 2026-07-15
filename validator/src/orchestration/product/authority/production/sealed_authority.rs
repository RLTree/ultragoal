//! Sealed production authority representation and its trusted descendants.
//!
//! Visibility in this subtree is intentional: Rust descendants may use an
//! ancestor's private items. Keeping the concrete capability here prevents a
//! sibling child of `production` from inheriting field, signer, or ledger
//! construction authority.

#[path = "checkpoint.rs"]
mod checkpoint;
#[path = "execution_transaction.rs"]
mod execution_transaction;
#[path = "ledger.rs"]
mod ledger;
#[path = "root_authority.rs"]
mod root_authority;
#[path = "store.rs"]
mod store;

use super::super::{PermitDecisionBinding, ProductError, RootOperation, RootPermit};
use ledger::{Ledger, LedgerState};
use sha2::{Digest, Sha256};
use std::fmt::{Debug, Formatter};
use std::path::Path;

const NONCE_BYTES: usize = 32;
const MAX_PERMIT_LIFETIME: u64 = 300;

include!("permit_issuance.rs");
include!("restart_reconciliation.rs");

pub(crate) use execution_transaction::ProductionExecutionOutcome;
#[cfg(not(test))]
use root_authority::RootAuthority;
use root_authority::RootPermitIssuance;
#[cfg(test)]
pub(crate) use root_authority::{
    root_authority_for_test, RootActionPermitIssuance, RootAuthority, RootReconcilePermitIssuance,
};

#[cfg(test)]
#[path = "interruption_tests.rs"]
mod interruption_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermitReplayState {
    Issued,
    Reserved,
    Committed,
    Refused,
    Ambiguous,
}

pub struct ProductionRootAuthority {
    authority: RootAuthority,
    ledger: Ledger,
}

impl Debug for ProductionRootAuthority {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProductionRootAuthority")
            .field("root_actor", &self.authority.root_actor().as_str())
            .field("store", &"[owner-only]")
            .finish()
    }
}

impl ProductionRootAuthority {
    /// Initializes missing authority files only inside an explicit,
    /// pre-existing owner-only directory. Never call this from a read route.
    pub fn open_or_initialize(
        root: &Path,
        root_actor: crate::orchestration::Actor,
    ) -> Result<Self, ProductError> {
        let (store, key) = store::Store::open_or_initialize(root, root_actor.as_str())?;
        Ok(Self::new(root_actor, key, Ledger::open(store)?))
    }

    /// Reopens only empty existing state. Nonempty replay state remains blocked
    /// until root-owned external monotonic custody is wired.
    pub fn open_existing(
        root: &Path,
        root_actor: crate::orchestration::Actor,
    ) -> Result<Self, ProductError> {
        let (store, key) = store::Store::open_existing(root, root_actor.as_str())?;
        Ok(Self::new(root_actor, key, Ledger::open(store)?))
    }
}

fn permit_id(permit: &RootPermit) -> Result<String, ProductError> {
    let bytes = serde_json::to_vec(permit).map_err(|_| ProductError::AuthorityInvalid)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn causal_slot_id(permit: &RootPermit) -> Result<String, ProductError> {
    #[derive(serde::Serialize)]
    struct CausalSlot<'a> {
        schema_version: &'a str,
        root_actor: &'a str,
        operation: RootOperation,
        binding: &'a crate::orchestration::Binding,
        workspace_identity: &'a str,
        journal_head_identity: &'a str,
        issued_tick: u64,
        target: &'a super::super::PermitTarget,
        decision_binding: &'a PermitDecisionBinding,
    }

    let bytes = serde_json::to_vec(&CausalSlot {
        schema_version: &permit.schema_version,
        root_actor: &permit.root_actor,
        operation: permit.operation,
        binding: &permit.binding,
        workspace_identity: &permit.workspace_identity,
        journal_head_identity: &permit.journal_head_identity,
        issued_tick: permit.issued_tick,
        target: &permit.target,
        decision_binding: &permit.decision_binding,
    })
    .map_err(|_| ProductError::AuthorityInvalid)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
