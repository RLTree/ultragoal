use crate::migration::product::{
    DarwinMigrationAdapters, DarwinMigrationHost, ProductMigrationPlan, derive_product_plan,
    provision_darwin_migration_host_for_test,
};
use crate::migration::{
    InventorySurface, InventorySurfaceObservation, MigrationInventory, SurfaceFileKind,
    SurfaceStatus,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

include!("host_fixture/product_version.rs");

include!("host_fixture/registry_bytes.rs");
