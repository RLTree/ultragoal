use crate::distribution::{
    DistributionErrorId, ExpectedPrior as Prior, InstallEffects, InstallPlan,
    InstallScope as Scope, PackageEffects, build_package, install, plan_package, rollback_install,
    uninstall,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture, digest};
use std::collections::{BTreeMap, BTreeSet};

include!("install/external_mutation.rs");

include!("install/automatic_and_explicit_rollbacks_preserve_concurrent_mutation.rs");
