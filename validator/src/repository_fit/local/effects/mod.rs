//! Descriptor-anchored local compare-exchange effects.
//!
//! The supported implementation is intentionally Darwin-only because it
//! relies on `renameatx_np(RENAME_SWAP|RENAME_EXCL)` for one finite atomic
//! namespace linearization point. Other hosts fail closed.
#[cfg(target_vendor = "apple")]
#[path = "apple/mod.rs"]
mod supported;
#[cfg(not(target_vendor = "apple"))]
#[path = "unsupported.rs"]
mod supported;

pub(crate) use supported::LocalEffects;
