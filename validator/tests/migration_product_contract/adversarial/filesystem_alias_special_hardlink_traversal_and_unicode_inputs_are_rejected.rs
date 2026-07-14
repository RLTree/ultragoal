#[test]
fn filesystem_alias_special_hardlink_traversal_and_unicode_inputs_are_rejected() {
    for kind in [
        SurfaceFileKind::Directory,
        SurfaceFileKind::Symlink,
        SurfaceFileKind::Special,
    ] {
        let bad = InventorySurface::observed(InventorySurfaceObservation {
            stable_id: "LEGACY-SKILL:bad".to_owned(),
            kind: "legacy-skill".to_owned(),
            relative_path: "skills/bad/SKILL.md".to_owned(),
            digest_sha256: sha('a'),
            file_kind: kind,
            link_count: 1,
            status: SurfaceStatus::Active,
            active_readers: vec![],
            active_writers: vec![],
            public_routes: vec![],
            generated_outputs: vec![],
        });
        assert!(
            MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![bad]).is_err()
        );
    }
    let hardlink = InventorySurface::observed(InventorySurfaceObservation {
        stable_id: "LEGACY-SKILL:bad".to_owned(),
        kind: "legacy-skill".to_owned(),
        relative_path: "skills/bad/SKILL.md".to_owned(),
        digest_sha256: sha('a'),
        file_kind: SurfaceFileKind::Regular,
        link_count: 2,
        status: SurfaceStatus::Active,
        active_readers: vec![],
        active_writers: vec![],
        public_routes: vec![],
        generated_outputs: vec![],
    });
    assert!(
        MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![hardlink]).is_err()
    );
    let traversal = InventorySurface::observed(InventorySurfaceObservation {
        stable_id: "LEGACY-SKILL:bad".to_owned(),
        kind: "legacy-skill".to_owned(),
        relative_path: "../escape".to_owned(),
        digest_sha256: sha('a'),
        file_kind: SurfaceFileKind::Regular,
        link_count: 1,
        status: SurfaceStatus::Active,
        active_readers: vec![],
        active_writers: vec![],
        public_routes: vec![],
        generated_outputs: vec![],
    });
    assert!(
        MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![traversal]).is_err()
    );
    for stable_id in [
        "/LEGACY-SKILL:absolute",
        "LEGACY-SKILL:../escape",
        "LEGACY-SKILL:double//component",
        "LEGACY-SKILL:dot/./component",
        "LEGACY-SKILL:backslash\\component",
        "LEGACY-SKILL:unicodé",
    ] {
        let bad_identity = InventorySurface::observed(InventorySurfaceObservation {
            stable_id: stable_id.to_owned(),
            kind: "legacy-skill".to_owned(),
            relative_path: "skills/bad/SKILL.md".to_owned(),
            digest_sha256: sha('a'),
            file_kind: SurfaceFileKind::Regular,
            link_count: 1,
            status: SurfaceStatus::Active,
            active_readers: vec![],
            active_writers: vec![],
            public_routes: vec![],
            generated_outputs: vec![],
        });
        assert!(
            MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha('0'), vec![bad_identity],)
                .is_err()
        );
    }

    let registry_payload = registry_bytes(vec![route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        None,
    )]);
    for kind in [
        SurfaceFileKind::Directory,
        SurfaceFileKind::Symlink,
        SurfaceFileKind::Special,
    ] {
        assert!(
            AdoptedRegistrySnapshot::observed(
                "migration/authority-routes.json",
                kind,
                1,
                sha('f'),
                registry_payload.clone(),
            )
            .is_err()
        );
    }
    assert!(
        AdoptedRegistrySnapshot::observed(
            "migration/authority-routes.json",
            SurfaceFileKind::Regular,
            2,
            sha('f'),
            registry_payload,
        )
        .is_err()
    );

    let unicode_inventory = inventory(
        vec![surface(
            "LEGACY-SKILL:unicode",
            "legacy-skill",
            "skills/café/SKILL.md",
            'a',
            SurfaceStatus::Active,
            &[],
            &[],
            &[],
            &[],
        )],
        '0',
    );
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(vec![route(
            "route-unicode",
            "LEGACY-SKILL:unicode",
            "skills/cafe/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            None,
        )]),
    )
    .unwrap();
    assert!(ProductInputSnapshot::observed(unicode_inventory, registry).is_err());

    let collision_inventory = inventory(
        vec![
            surface(
                "LEGACY-SKILL:a",
                "legacy-skill",
                "skills/Case/SKILL.md",
                'a',
                SurfaceStatus::Active,
                &[],
                &[],
                &[],
                &[],
            ),
            surface(
                "LEGACY-SKILL:b",
                "legacy-skill",
                "skills/case/SKILL.md",
                'b',
                SurfaceStatus::Candidate,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        '0',
    );
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(vec![route(
            "route-case",
            "LEGACY-SKILL:a",
            "skills/Case/SKILL.md",
            "LEGACY-SKILL:b",
            'a',
            'b',
            None,
        )]),
    )
    .unwrap();
    assert_eq!(
        ProductInputSnapshot::observed(collision_inventory, registry)
            .unwrap_err()
            .code(),
        "migration-product-inventory-path-collision"
    );
}
