pub(crate) mod distribution {
    pub use ultragoal::distribution::*;
}

#[path = "../src/plugin_product/host_lifecycle/darwin/mod.rs"]
mod host_lifecycle;

#[path = "supported_host_plugin_transaction_contract/product.rs"]
mod product;
#[path = "supported_host_plugin_transaction_contract/recovery.rs"]
mod recovery;
#[path = "supported_host_plugin_transaction_contract/security.rs"]
mod security;
#[path = "supported_host_plugin_transaction_contract/transaction_fixture.rs"]
mod transaction_fixture;
