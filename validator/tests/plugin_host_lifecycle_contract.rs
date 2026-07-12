pub(crate) mod distribution {
    pub use ultragoal::distribution::*;
}

pub(crate) mod plugin_product {
    pub mod distribution_adapter {
        pub use ultragoal::plugin_product::distribution_adapter::*;
    }
    pub mod lifecycle {
        pub use ultragoal::plugin_product::lifecycle::*;
    }
}

#[path = "../src/plugin_product/host_lifecycle/mod.rs"]
mod host_lifecycle;

#[path = "plugin_host_lifecycle_contract/capability.rs"]
mod capability;
#[path = "plugin_host_lifecycle_contract/false_pass.rs"]
mod false_pass;
#[path = "plugin_host_lifecycle_contract/journeys.rs"]
mod journeys;
#[path = "plugin_host_lifecycle_contract/negative.rs"]
mod negative;
#[path = "plugin_host_lifecycle_contract/security.rs"]
mod security;
#[path = "plugin_host_lifecycle_contract/support.rs"]
mod support;
