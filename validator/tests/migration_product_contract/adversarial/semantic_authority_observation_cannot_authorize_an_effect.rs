#[test]
fn semantic_authority_observation_cannot_authorize_an_effect() {
    let semantic = |id: &str, kind: &str, path: &str, digest_byte: char| {
        InventorySurface::observed(InventorySurfaceObservation {
            stable_id: id.to_owned(),
            kind: kind.to_owned(),
            relative_path: path.to_owned(),
            digest_sha256: sha(digest_byte),
            file_kind: SurfaceFileKind::Semantic,
            link_count: 0,
            status: SurfaceStatus::Active,
            active_readers: Vec::new(),
            active_writers: Vec::new(),
            public_routes: Vec::new(),
            generated_outputs: Vec::new(),
        })
    };
    let input = input_with_routes(
        vec![
            semantic(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
            ),
            semantic("SKILL:current", "skill", "skills/current/SKILL.md", 'b'),
        ],
        vec![compatibility_route()],
        '0',
    );

    assert_eq!(
        derive_plan(&input).unwrap_err().code(),
        "migration-product-adopted-transition-invalid",
    );
}
