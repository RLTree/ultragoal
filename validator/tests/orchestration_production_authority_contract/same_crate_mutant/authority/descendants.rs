use super::MutantCrate;

const PRODUCTION: &str = "orchestration/product/authority/production/mod.rs";

pub(super) fn expose_types(fixture: &MutantCrate) {
    fixture.replace(
        PRODUCTION,
        "mod sealed_authority;",
        "pub(crate) mod sealed_authority;",
    );
    let sealed = "orchestration/product/authority/production/sealed_authority.rs";
    for module in ["execution_transaction", "ledger", "root_authority", "store"] {
        fixture.replace(
            sealed,
            &format!("mod {module};"),
            &format!("pub(crate) mod {module};"),
        );
    }
    fixture.replace(
        "orchestration/product/authority/root/secret_binding.rs",
        "pub(super) struct RootPermitIssuance<'a>",
        "pub(crate) struct RootPermitIssuance<'a>",
    );
    for name in [
        "RootActionPermitVerification",
        "RootReconcilePermitVerification",
    ] {
        fixture.replace(
            "orchestration/product/authority/root/verification.rs",
            &format!("pub(super) struct {name}<'a>"),
            &format!("pub(crate) struct {name}<'a>"),
        );
    }
    fixture.replace(
        "orchestration/product/authority/production/store.rs",
        "pub(super) struct Store {",
        "pub(crate) struct Store {",
    );
    fixture.replace(
        "orchestration/product/authority/production/ledger.rs",
        "pub(super) struct Ledger {",
        "pub(crate) struct Ledger {",
    );
    let route = "orchestration/product/authority/production/execution_transaction/route.rs";
    for module in ["resume", "recover", "reconcile"] {
        fixture.replace(
            route,
            &format!("mod {module};"),
            &format!("pub(super) mod {module};"),
        );
    }
}

pub(super) fn expose_members(fixture: &MutantCrate) {
    let root = "orchestration/product/authority/production/root_authority.rs";
    fixture.replace(
        root,
        "    root_actor: Actor,",
        "    pub(crate) root_actor: Actor,",
    );
    fixture.replace(root, "    key: [u8; 32],", "    pub(crate) key: [u8; 32],");
    fixture.replace(root, "    pub(super) fn new(", "    pub(crate) fn new(");
    fixture.append(root, "impl RootAuthority { pub(crate) fn clone(&self) {} }");
    let binding = "orchestration/product/authority/root/secret_binding.rs";
    fixture.replace(
        binding,
        "    pub(super) fn issue(",
        "    pub(crate) fn issue(",
    );
    fixture.replace(
        binding,
        "    pub(super) fn verify_action(",
        "    pub(crate) fn verify_action(",
    );
    let sealed = "orchestration/product/authority/production/sealed_authority.rs";
    fixture.replace(
        sealed,
        "    authority: RootAuthority,",
        "    pub(crate) authority: RootAuthority,",
    );
    fixture.replace(
        sealed,
        "    ledger: Ledger,",
        "    pub(crate) ledger: Ledger,",
    );
    fixture.append(
        sealed,
        "impl ProductionRootAuthority { pub(crate) fn clone(&self) {} }",
    );
    fixture.replace(
        "orchestration/product/authority/production/store.rs",
        "    pub(super) fn open_or_initialize(",
        "    pub(crate) fn open_or_initialize(",
    );
    let ledger = "orchestration/product/authority/production/ledger.rs";
    fixture.replace(
        ledger,
        "    pub(super) fn issue(",
        "    pub(crate) fn issue(",
    );
    fixture.replace(
        ledger,
        "    pub(super) fn reserve(",
        "    pub(crate) fn reserve(",
    );
    fixture.replace(
        "orchestration/product/authority/production/ledger_integrity.rs",
        "    pub(super) fn open(",
        "    pub(crate) fn open(",
    );
    for module in ["resume", "recover", "reconcile"] {
        fixture.replace(
            &format!(
                "orchestration/product/authority/production/execution_transaction/{module}.rs"
            ),
            "pub(super) fn execute(",
            "pub(in super::super) fn execute(",
        );
    }
}
