use crate::distribution::{
    AppRegistryVerdict, DiscoveryVerdict, DistributionErrorId as ErrorId, ExpectedPrior,
    ExpectedTree, HostCapabilityDeclaration, InstallPlan, InstallScope, JourneyBinding, ScopedFile,
    ScopedInstall, ScopedTree, install, materialize_package, observe_app_registry,
    observe_discovery, registry_document,
};
use crate::distribution_fixture::digest;
use crate::package_journey_fixture::{JourneyFixture, renamed, write_scoped};
use serde_json::{Value, json};
use std::fs;

include!(
    "journey_adversarial/wrong_scope_identity_duplicates_and_registered_hidden_fail_closed.rs"
);

include!(
    "journey_adversarial/scoped_reads_reject_symlink_hardlink_and_special_file_substitution.rs"
);
