macro_rules! include_production_package_module {
    () => {};
}

#[path = "../src/plugin_manifest/mod.rs"]
mod plugin_manifest;

#[path = "../src/distribution/mod.rs"]
mod distribution;

#[path = "distribution_contract/adversarial_fs.rs"]
mod adversarial_fs;
#[path = "distribution_contract/adversarial_semantics.rs"]
mod adversarial_semantics;
#[path = "distribution_contract/cache_host_kernel.rs"]
mod cache_host_kernel;
#[path = "distribution_contract/descriptor_cas.rs"]
mod descriptor_cas;
#[path = "distribution_contract/descriptor_races.rs"]
mod descriptor_races;
#[path = "distribution_contract/descriptor_recovery.rs"]
mod descriptor_recovery;
#[path = "distribution_contract/descriptor_unlink.rs"]
mod descriptor_unlink;
#[path = "distribution_contract/distribution_fixture.rs"]
mod distribution_fixture;
#[path = "distribution_contract/identity_ladder.rs"]
mod identity_ladder;
#[path = "distribution_contract/install.rs"]
mod install;
#[path = "distribution_contract/isolated_journey.rs"]
mod isolated_journey;
#[path = "distribution_contract/journey_adversarial.rs"]
mod journey_adversarial;
#[path = "distribution_contract/manifest_semantics.rs"]
mod manifest_semantics;
#[path = "distribution_contract/marketplace_install_kernel.rs"]
mod marketplace_install_kernel;
#[path = "distribution_contract/marketplace_supply.rs"]
mod marketplace_supply;
#[path = "distribution_contract/observation_races.rs"]
mod observation_races;
#[path = "distribution_contract/package.rs"]
mod package;
#[path = "distribution_contract/package_identity.rs"]
mod package_identity;
#[path = "distribution_contract/package_journey_fixture.rs"]
mod package_journey_fixture;
#[path = "distribution_contract/package_manifest.rs"]
mod package_manifest;
#[path = "distribution_contract/package_output.rs"]
mod package_output;
#[path = "distribution_contract/positive.rs"]
mod positive;
#[path = "distribution_contract/prefix_collisions.rs"]
mod prefix_collisions;
#[path = "distribution_contract/runtime_session.rs"]
mod runtime_session;
#[path = "distribution_contract/states.rs"]
mod states;
#[path = "distribution_contract/version_identity.rs"]
mod version_identity;
