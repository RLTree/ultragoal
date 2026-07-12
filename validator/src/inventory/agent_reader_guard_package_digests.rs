use super::ReaderSpec;

macro_rules! package_reader {
    ($path:literal, $include:literal) => {
        ReaderSpec {
            path: $path,
            bytes: include_bytes!($include),
            legacy_tokens_are_negative_only: false,
        }
    };
}

pub(crate) const PACKAGE_READERS: &[ReaderSpec] = &[
    package_reader!(
        "validator/src/inventory/agent_reader_guard_package_digests.rs",
        "agent_reader_guard_package_digests.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/anchored/json.rs",
        "../package/inventory/anchored/json.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/anchored/mod.rs",
        "../package/inventory/anchored/mod.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/anchored/session.rs",
        "../package/inventory/anchored/session.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/anchored/snapshot.rs",
        "../package/inventory/anchored/snapshot.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/anchored/sys.rs",
        "../package/inventory/anchored/sys.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/closure/mod.rs",
        "../package/inventory/closure/mod.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/closure/package_entries.rs",
        "../package/inventory/closure/package_entries.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/generated_disposition/anchored.rs",
        "../package/inventory/generated_disposition/anchored.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/generated_disposition/mod.rs",
        "../package/inventory/generated_disposition/mod.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/generated_disposition/schema.rs",
        "../package/inventory/generated_disposition/schema.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/generated_disposition/unique_json.rs",
        "../package/inventory/generated_disposition/unique_json.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/payload.rs",
        "../package/inventory/payload.rs"
    ),
    package_reader!(
        "validator/src/package/inventory/snapshot/capture.rs",
        "../package/inventory/snapshot/capture.rs"
    ),
];
