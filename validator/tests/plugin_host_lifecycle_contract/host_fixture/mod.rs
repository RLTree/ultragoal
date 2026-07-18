use crate::distribution::{
    CodexPlugin, ConfinedRoot, HostCapabilityDeclaration, HostCommandPlan, JourneyBinding,
    MarketplacePlan, PackagePlan, PackageSnapshot, RuntimeObservation, RuntimeProbePlan,
    ScopedFile, build_package, execute_runtime_probe, plan_codex_marketplace, plan_package,
    registry_document,
};
use crate::host_lifecycle::{
    HostLifecycleBindRequest, HostLifecycleSession, HostObservationTransactionRequest,
    HostScopeAuthority, HostSurfaceReader, HostSurfaceTransaction, HostSurfaceTransactionError,
};
use crate::plugin_product::lifecycle::{
    LifecycleAuthorization, LifecycleIntent, LifecyclePlan, LifecycleRequest, LifecycleState,
    PackageAuthority, Version, plan,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

include!("host_fixture/context.rs");

include!("host_fixture/reader_empty.rs");

include!("host_fixture/ui_document.rs");
