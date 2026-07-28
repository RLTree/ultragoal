#[derive(Clone, Copy)]
pub(crate) struct ReaderSpec {
    pub(crate) path: &'static str,
    pub(crate) bytes: &'static [u8],
    pub(crate) legacy_tokens_are_negative_only: bool,
}

macro_rules! reader {
    ($path:literal, $include:literal, $negative:literal) => {
        ReaderSpec {
            path: $path,
            // Bind the compiled witness to the canonical repository-relative
            // path. Macro invocations live in included semantic fragments, so
            // resolving `$include` from the invocation file would make the
            // witness depend on the fragment's directory depth.
            bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../", $path)),
            legacy_tokens_are_negative_only: $negative,
        }
    };
}

// This compiled allowlist is deliberately outside inventory/compatibility. The
// witness scans that subtree too, and compares this source file with its own
// compile-time bytes before trusting any row below.
include!("foundational_authority_readers.rs");
include!("plugin_product_review_readers.rs");
include!("runtime_routing_readers.rs");

#[path = "../agent/manifest_digests.rs"]
mod manifests;
pub(crate) use manifests::MANIFESTS;
#[path = "../agent/package_digests.rs"]
mod package;
pub(crate) use package::PACKAGE_READERS;

pub(crate) fn all_readers() -> impl Iterator<Item = &'static ReaderSpec> {
    FOUNDATIONAL_AUTHORITY_READERS
        .iter()
        .chain(PLUGIN_PRODUCT_REVIEW_READERS)
        .chain(RUNTIME_ROUTING_READERS)
        .chain(PACKAGE_READERS)
}
