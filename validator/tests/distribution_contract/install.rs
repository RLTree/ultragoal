use crate::distribution::{
    DistributionErrorId, EffectPoint, ExpectedPrior as Prior, InstallEffects, InstallPlan,
    InstallScope as Scope, PackageEffects, RollbackInstallError, ScopedInstall,
    assert_test_effect_hook_consumed, build_package, install, plan_package, rollback_install,
    set_test_effect_hook_matching, uninstall,
};
use crate::distribution_fixture::{CANDIDATE_ID, CONTEXT_ID, Fixture, digest};
use std::collections::{BTreeMap, BTreeSet};

include!("install/external_mutation.rs");

include!("install/automatic_and_explicit_rollbacks_preserve_concurrent_mutation.rs");

include!("install/rollback_authority.rs");

include!("install/rollback_authority_helpers.rs");
