#[cfg(unix)]
mod descriptor;
mod file;
#[cfg(unix)]
mod file_transition;
mod hooks;
mod owned;
mod read_only_tree;
mod remove;
mod root;
mod tree;
mod tree_ops;
mod walk;

pub use file::{ScopedFile, ScopedInstall};
pub(crate) use read_only_tree::ReadOnlyTreeObservation;
pub use root::ConfinedRoot;
pub(crate) use root::ReadOnlyWorkspace;
pub(crate) use root::canonical_temporary_parent;
pub use tree::ScopedTree;

#[cfg(all(test, unix))]
pub(crate) use hooks::{
    EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
};
