pub(crate) const PRODUCT_VERSION: &str = "0.0.12";
pub(crate) const SOURCE_PATH: &str = "skills/old/SKILL.md";
pub(crate) const TARGET_PATH: &str = "skills/current/SKILL.md";
pub(crate) const REGISTRY_PATH: &str = "migration/authority-routes.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestDisposition {
    Retirement,
    Compatibility,
    Pending,
}

pub(crate) struct TestHost {
    pub(crate) root: PathBuf,
    pub(crate) repository: PathBuf,
    pub(crate) state: PathBuf,
    pub(crate) inventory: MigrationInventory,
    pub(crate) candidate_id: String,
    source_bytes: Vec<u8>,
}

impl TestHost {
    pub(crate) fn new(disposition: TestDisposition) -> Self {
        let root = unique_root();
        let repository = root.join("repository");
        let state = root.join("state");
        create_directory(&root, 0o700);
        create_directory(&repository, 0o700);
        create_directory(&repository.join("skills"), 0o700);
        create_directory(&repository.join("skills/old"), 0o700);
        create_directory(&repository.join("skills/current"), 0o700);
        create_directory(&repository.join("migration"), 0o700);

        let source_bytes = b"synthetic legacy migration source\n".to_vec();
        let target_bytes = b"synthetic canonical migration target\n".to_vec();
        write_new(&repository.join(SOURCE_PATH), &source_bytes, 0o600);
        write_new(&repository.join(TARGET_PATH), &target_bytes, 0o600);

        let source_digest = hash(&source_bytes);
        let target_digest = hash(&target_bytes);
        let registry = registry_bytes(disposition, &source_digest, &target_digest);
        write_new(&repository.join(REGISTRY_PATH), &registry, 0o600);

        let candidate_id = hash(b"synthetic migration candidate");
        let source_status = match disposition {
            TestDisposition::Retirement | TestDisposition::Pending => SurfaceStatus::Candidate,
            TestDisposition::Compatibility => SurfaceStatus::Active,
        };
        let source = InventorySurface::observed(InventorySurfaceObservation {
            stable_id: "LEGACY-SKILL:old".to_owned(),
            kind: "legacy-skill".to_owned(),
            relative_path: SOURCE_PATH.to_owned(),
            digest_sha256: source_digest,
            file_kind: SurfaceFileKind::Regular,
            link_count: 1,
            status: source_status,
            active_readers: vec!["legacy-reader".to_owned()],
            active_writers: vec!["legacy-writer".to_owned()],
            public_routes: vec!["legacy-public".to_owned()],
            generated_outputs: vec!["legacy-generated".to_owned()],
        });
        let target = InventorySurface::observed(InventorySurfaceObservation {
            stable_id: "SKILL:current".to_owned(),
            kind: "skill".to_owned(),
            relative_path: TARGET_PATH.to_owned(),
            digest_sha256: target_digest,
            file_kind: SurfaceFileKind::Regular,
            link_count: 1,
            status: SurfaceStatus::Active,
            active_readers: vec![],
            active_writers: vec![],
            public_routes: vec!["current-public".to_owned()],
            generated_outputs: vec![],
        });
        let inventory = MigrationInventory::new(
            hash(b"synthetic live context"),
            candidate_id.clone(),
            hash(b"synthetic catalog"),
            hash(b"synthetic read session"),
            vec![source, target],
        )
        .unwrap();
        provision_darwin_migration_host_for_test(&repository, &state, &inventory, PRODUCT_VERSION)
            .unwrap();
        Self {
            root,
            repository,
            state,
            inventory,
            candidate_id,
            source_bytes,
        }
    }

    pub(crate) fn open(
        &self,
    ) -> Result<DarwinMigrationAdapters, crate::migration::product::HostError> {
        DarwinMigrationHost::open(
            &self.repository,
            &self.state,
            self.inventory.clone(),
            &self.candidate_id,
            PRODUCT_VERSION,
        )
    }

    pub(crate) fn derive_plan(
        &self,
        adapters: &mut DarwinMigrationAdapters,
    ) -> ProductMigrationPlan {
        use crate::migration::product::MigrationInputSource;
        let input = adapters.source.capture().unwrap();
        let authority = adapters.boundary_authority().unwrap();
        derive_product_plan(&input, Some(&authority)).unwrap()
    }

    pub(crate) fn state_bytes(&self) -> Vec<u8> {
        fs::read(self.state.join("state.json")).unwrap()
    }

    pub(crate) fn source_bytes(&self) -> Vec<u8> {
        fs::read(self.repository.join(SOURCE_PATH)).unwrap()
    }

    pub(crate) fn original_source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        if self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("hul-migration-host-contract-"))
        {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
