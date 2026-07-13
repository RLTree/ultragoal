#![allow(dead_code, unused_imports)]

#[path = "../src/digest.rs"]
mod digest;

#[path = "../src/context/mod.rs"]
mod context;

#[path = "supported_package_product_contract/inventory.rs"]
mod inventory;

#[path = "../src/package/inventory/mod.rs"]
mod package_inventory;

mod package {
    pub(crate) mod inventory {
        pub(crate) use crate::package_inventory::*;
    }
}

#[path = "../src/plugin_manifest/mod.rs"]
mod plugin_manifest;

mod self_tests {
    pub(crate) mod boundaries {
        pub(crate) mod workspace_fixtures {
            use std::path::PathBuf;
            use std::sync::atomic::{AtomicU64, Ordering};

            static NEXT: AtomicU64 = AtomicU64::new(0);

            pub(crate) fn temp_root(label: &str) -> PathBuf {
                std::env::temp_dir().join(format!(
                    "hul-supported-package-production-087-{label}-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ))
            }
        }
    }
}

macro_rules! include_production_package_module {
    () => {
        mod product;
        pub(crate) use product::{
            ProductionPackageArtifact, ProductionPackageError, ProductionPackageErrorId,
            ProductionPackageSession, capture_product_package, verify_product_package,
        };
        #[path = "../../../tests/supported_package_product_contract/cases.rs"]
        mod supported_package_product_contract_cases;
    };
}

#[path = "../src/distribution/mod.rs"]
mod distribution;
