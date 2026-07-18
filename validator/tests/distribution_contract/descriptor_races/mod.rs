#![cfg(unix)]

use crate::distribution::{
    DistributionErrorId as ErrorId, EffectPoint, ExpectedTree, ScopedFile, ScopedTree,
    assert_test_effect_hook_consumed, materialize_package, set_test_effect_hook_matching,
};
use crate::distribution_fixture::{digest, tree};
use crate::package_journey_fixture::{JourneyFixture, renamed, write_scoped};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

include!("next_outside.rs");

include!("directory_ancestor_substitution_then_relocation_preserves_outside_tree.rs");

include!("tree_materialization_and_recovery_rename_races_preserve_outside_tree.rs");
