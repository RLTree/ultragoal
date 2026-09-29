use super::super::{GeneratedAuthorityParseRequest, GeneratedSurface, parse};

#[test]
fn current_registry_is_exact_canonical_v3() {
    let bytes = include_bytes!("../../../../migration/generated-surface-authority.json");
    let response =
        parse(GeneratedAuthorityParseRequest { bytes }).expect("current registry parses");

    assert_eq!(response.canonical_bytes, bytes);
    assert!(!response.registry.surfaces.is_empty());
    let product_success = response
        .registry
        .surfaces
        .values()
        .find(|surface| {
            surface.output().as_str() == "examples/generated/PRODUCT_SUCCESS_CONTRACT.json"
        })
        .expect("retained predecessor product contract remains registered");
    assert!(matches!(
        product_success,
        GeneratedSurface::RetainedContext { .. }
    ));
    assert!(
        !response
            .registry
            .surfaces
            .values()
            .any(|surface| matches!(surface, GeneratedSurface::AdoptedSchemaContract { .. }))
    );
    assert!(
        response
            .registry
            .surfaces
            .values()
            .any(|surface| matches!(surface, GeneratedSurface::SourceProjection { .. }))
    );
    assert!(
        response
            .registry
            .surfaces
            .values()
            .any(|surface| matches!(surface, GeneratedSurface::ToolProjection { .. }))
    );
}
