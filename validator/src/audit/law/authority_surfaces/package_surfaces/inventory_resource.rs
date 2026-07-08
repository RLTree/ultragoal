pub(super) fn live_proof_package_resource(rel: &str) -> bool {
    rel.starts_with("validation_artifacts/")
}

pub(super) fn live_proof_resource_reason() -> &'static str {
    "package inventory lists a top-level runtime proof artifact as a package resource"
}

pub(super) fn live_proof_resource_repair() -> &'static str {
    "remove the live proof artifact from package inventory; fixture-contained validation artifacts may remain fixture resources"
}
