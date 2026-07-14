pub(crate) fn input_with_routes(
    surfaces: Vec<InventorySurface>,
    routes: Vec<Value>,
    session: char,
) -> ProductInputSnapshot {
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(routes),
    )
    .unwrap();
    ProductInputSnapshot::observed(inventory(surfaces, session), registry).unwrap()
}

pub(crate) fn compatibility_route() -> Value {
    route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        Some("compatibility"),
    )
}

pub(crate) fn compatibility_input_with_route(
    compatibility_route: Value,
    session: char,
) -> ProductInputSnapshot {
    input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Active,
                &["legacy-reader"],
                &["legacy-writer"],
                &["legacy-public"],
                &["legacy-generated"],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        vec![compatibility_route],
        session,
    )
}

pub(crate) fn compatibility_input() -> ProductInputSnapshot {
    compatibility_input_with_route(compatibility_route(), '0')
}

pub(crate) fn retirement_input() -> ProductInputSnapshot {
    input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Candidate,
                &["legacy-reader"],
                &["legacy-writer"],
                &["legacy-public"],
                &["legacy-generated"],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &["current-public"],
                &[],
            ),
        ],
        vec![route(
            "route-old-to-current",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            Some("retirement"),
        )],
        '0',
    )
}

#[derive(Clone)]
pub(crate) struct FakeSource {
    pub(crate) input: ProductInputSnapshot,
    pub(crate) stale: bool,
}

impl FakeSource {
    pub(crate) fn new(input: ProductInputSnapshot) -> Self {
        Self {
            input,
            stale: false,
        }
    }
}

impl MigrationInputSource for FakeSource {
    fn capture(&mut self) -> Result<ProductInputSnapshot, ProductMigrationError> {
        Ok(self.input.clone())
    }

    fn revalidate(
        &mut self,
        binding: &MigrationInputBinding,
        _applied_effect_ids: &[String],
    ) -> Result<(), ProductMigrationError> {
        if self.stale || &self.input.binding() != binding {
            Err(ProductMigrationError::new("test-migration-source-stale"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub(crate) struct FakeAuthority {
    pub(crate) principal: String,
    pub(crate) authority: String,
    pub(crate) session: String,
    pub(crate) nonce: String,
    pub(crate) issued: u64,
    pub(crate) expires: u64,
    pub(crate) now: u64,
    pub(crate) input_binding: String,
    pub(crate) plan_sha256: String,
    pub(crate) key: String,
    pub(crate) boundary_authority: String,
    pub(crate) boundary_source_identity: String,
    pub(crate) boundary_sequence: u64,
    pub(crate) boundary_now: u64,
    pub(crate) current_product_version: String,
    pub(crate) boundary_key: String,
    pub(crate) boundary_capture_count: Arc<AtomicUsize>,
    pub(crate) boundary_cross_after_captures: Option<usize>,
    pub(crate) boundary_crossed_now: Option<u64>,
    pub(crate) boundary_crossed_version: Option<String>,
}
