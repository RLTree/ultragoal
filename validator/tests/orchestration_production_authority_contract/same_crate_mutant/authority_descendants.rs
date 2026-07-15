pub(super) fn assert_rejected(stderr: &str) {
    for expected in [
        "module `store` is private",
        "module `ledger` is private",
        "module `root_authority` is private",
        "associated function `new` is private",
        "field `authority` of struct",
        "no method named `clone`",
        "module `resume` is private",
        "module `recover` is private",
        "module `reconcile` is private",
    ] {
        assert!(stderr.contains(expected), "missing {expected}: {stderr}");
    }
}

pub(super) const PRODUCTION_MUTANT: &str = r#"
mod descendant_authority_mutant {
    use crate::orchestration::product::{PermitTarget, ProductionRootAuthority, RootOperation};
    use crate::orchestration::{Actor, Binding};
    use std::path::Path;

    fn forge(root: &Path, actor: Actor) {
        let (store, _) = super::sealed_authority::store::Store::open_or_initialize(root, actor.as_str()).unwrap();
        let ledger = super::sealed_authority::ledger::Ledger::open(store).unwrap();
        let _authority = super::sealed_authority::ProductionRootAuthority::new(actor, [0; 32], ledger);
    }

    fn raw_issue(root: &Path, actor: Actor, permit_id: &str, slot_id: &str) {
        let signer = super::sealed_authority::root_authority::RootAuthority { root_actor: actor.clone(), key: [0; 32] };
        let (store, _) = super::sealed_authority::store::Store::open_or_initialize(root, actor.as_str()).unwrap();
        let ledger = super::sealed_authority::ledger::Ledger::open(store).unwrap();
        let _permit = signer.issue(super::sealed_authority::root_authority::RootPermitIssuance {
            operation: RootOperation::Resume,
            binding: Binding::new(permit_id, slot_id).unwrap(),
            workspace_identity: permit_id,
            journal_head_identity: slot_id,
            issued_tick: 1,
            expires_tick: u64::MAX,
            nonce: b"attacker-known-nonce-0123456789",
            target: PermitTarget::default(),
            decision_binding: panic!(),
        }).unwrap();
        ledger.issue(permit_id, slot_id).unwrap();
    }

    fn extract(authority: ProductionRootAuthority) {
        let _ = &authority.authority;
        let _ = authority.clone();
    }
}
"#;

pub(super) const EFFECT_MUTANT: &str = r#"
mod descendant_effect_mutant {
    use crate::orchestration::product::{ProductContext, ProductWorkspace, ReconcileRequest, RecoverRequest, ResumeRequest};

    fn direct_effects(context: &ProductContext, workspace: &ProductWorkspace, resume: &ResumeRequest, recover: &RecoverRequest, reconcile: &ReconcileRequest) {
        let _ = super::route::resume::execute(context, workspace, resume);
        let _ = super::route::recover::execute(context, workspace, recover);
        let _ = super::route::reconcile::execute(context, workspace, reconcile);
    }
}
"#;
