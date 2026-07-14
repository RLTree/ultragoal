#[cfg(unix)]
mod descriptor;
mod file;
#[cfg(unix)]
mod file_transition;
mod hooks;
mod owned;
mod remove;
mod root;
mod tree;
mod tree_ops;
mod walk;

pub use file::{ScopedFile, ScopedInstall};
pub use root::ConfinedRoot;
pub use tree::ScopedTree;

#[cfg(all(test, unix))]
pub(crate) use hooks::{
    EffectPoint, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
};
