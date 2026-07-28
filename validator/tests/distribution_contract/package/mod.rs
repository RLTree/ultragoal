use crate::distribution::{
    DistributionErrorId as ErrorId, PackageEffects, PackagePlan, build_package, plan_package,
    verify_package,
};
use crate::distribution_fixture::{Fixture, digest, tree};
use crate::package_manifest::{manifest, spec, write_sources};
use serde_json::{Value, json};
use std::fs;

include!("mutation.rs");

include!("package_inputs_reject_symlink_hardlink_and_special_files.rs");
