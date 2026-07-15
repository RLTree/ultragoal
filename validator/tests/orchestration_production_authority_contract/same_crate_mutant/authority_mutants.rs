pub(super) const HIDDEN: &str = r#"
mod n10_hidden_authority_probes {
    fn root_type() {
        let _ = std::mem::size_of::<super::sealed_authority::root_authority::RootAuthority>(); // N10_ROOT_AUTHORITY_TYPE
    }
    fn root_construct(actor: crate::orchestration::Actor, key: [u8; 32]) {
        let _ = super::sealed_authority::root_authority::RootAuthority { root_actor: actor, key }; // N10_ROOT_CONSTRUCT
    }
    fn store_type() {
        let _ = std::mem::size_of::<super::sealed_authority::store::Store>(); // N10_STORE_TYPE
    }
    fn ledger_type() {
        let _ = std::mem::size_of::<super::sealed_authority::ledger::Ledger>(); // N10_LEDGER_TYPE
    }
}
"#;

pub(super) const MEMBERS: &str = r#"
mod n10_authority_member_probes {
    type RootAuthority = super::sealed_authority::root_authority::RootAuthority;
    type Ledger = super::sealed_authority::ledger::Ledger;
    fn root_actor_extract(authority: &RootAuthority) {
        let _ = &authority.root_actor; // N10_ROOT_ACTOR_EXTRACT
    }
    fn root_key_extract(authority: &RootAuthority) {
        let _ = &authority.key; // N10_ROOT_KEY_EXTRACT
    }
    fn root_clone(authority: RootAuthority) {
        let _ = authority.clone(); // N10_ROOT_CLONE
    }
    fn root_issue(
        authority: &RootAuthority,
        request: super::sealed_authority::root_authority::RootPermitIssuance<'_>,
    ) {
        let _ = authority.issue(request); // N10_ROOT_ISSUE
    }
    fn root_verify(
        authority: &RootAuthority,
        request: super::sealed_authority::root_authority::RootActionPermitVerification<'_>,
    ) {
        let _ = authority.verify_action(request); // N10_ROOT_VERIFY
    }
    fn production_authority_extract(authority: &super::ProductionRootAuthority) {
        let _ = &authority.authority; // N10_PRODUCTION_AUTHORITY_EXTRACT
    }
    fn production_ledger_extract(authority: &super::ProductionRootAuthority) {
        let _ = &authority.ledger; // N10_PRODUCTION_LEDGER_EXTRACT
    }
    fn production_clone(authority: super::ProductionRootAuthority) {
        let _ = authority.clone(); // N10_PRODUCTION_CLONE
    }
    fn production_new(actor: crate::orchestration::Actor, key: [u8; 32], ledger: Ledger) {
        let _ = super::ProductionRootAuthority::new(actor, key, ledger); // N10_PRODUCTION_NEW
    }
    fn store_open(root: &std::path::Path, actor: &str) {
        let _ = super::sealed_authority::store::Store::open_or_initialize(root, actor); // N10_STORE_OPEN
    }
    fn ledger_open(store: super::sealed_authority::store::Store) {
        let _ = Ledger::open(store); // N10_LEDGER_OPEN
    }
    fn ledger_issue(ledger: &Ledger, permit_id: &str, slot_id: &str) {
        let _ = ledger.issue(permit_id, slot_id); // N10_LEDGER_ISSUE
    }
    fn ledger_reserve(ledger: &Ledger, permit_id: &str) {
        let _ = ledger.reserve(permit_id); // N10_LEDGER_RESERVE
    }
}
"#;

pub(super) const HIDDEN_EFFECTS: &str = r#"
mod n10_hidden_raw_effect_probes {
    fn resume(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ResumeRequest) {
        let _ = super::route::resume::execute(context, workspace, request); // N10_RAW_RESUME_MODULE
    }
    fn recover(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::RecoverRequest) {
        let _ = super::route::recover::execute(context, workspace, request); // N10_RAW_RECOVER_MODULE
    }
    fn reconcile(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ReconcileRequest) {
        let _ = super::route::reconcile::execute(context, workspace, request); // N10_RAW_RECONCILE_MODULE
    }
}
"#;

pub(super) const MEMBER_EFFECTS: &str = r#"
mod n10_raw_effect_member_probes {
    fn resume(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ResumeRequest) {
        let _ = super::route::resume::execute(context, workspace, request); // N10_RAW_RESUME_FUNCTION
    }
    fn recover(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::RecoverRequest) {
        let _ = super::route::recover::execute(context, workspace, request); // N10_RAW_RECOVER_FUNCTION
    }
    fn reconcile(context: &crate::orchestration::product::ProductContext, workspace: &crate::orchestration::product::ProductWorkspace, request: &crate::orchestration::product::ReconcileRequest) {
        let _ = super::route::reconcile::execute(context, workspace, request); // N10_RAW_RECONCILE_FUNCTION
    }
}
"#;
