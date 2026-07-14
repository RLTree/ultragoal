use super::digest::sha256_hex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

include!("max_catalog_bytes.rs");

include!("authority_catalog.rs");

include!("activation_failure_code.rs");

#[cfg(test)]
mod catalog_identity_tests {
    use super::*;

    fn catalog(catalog_id: String, generated_surfaces: GeneratedSurfaceIndex) -> AuthorityCatalog {
        AuthorityCatalog::new(AuthorityCatalogDefinition {
            catalog_id,
            context_id: "sha256:context".to_owned(),
            contract_id: "test-contract".to_owned(),
            source_registry_counts: BTreeMap::new(),
            entries: Vec::new(),
            findings: Vec::new(),
            generated_surfaces,
        })
    }

    #[test]
    fn one_canonical_identity_issues_and_revalidates_catalogs() {
        let id = catalog_identity_id(
            "sha256:context",
            "test-contract",
            &BTreeMap::new(),
            &[],
            &[],
        )
        .expect("catalog identity");
        let catalog = AuthorityCatalog::canonical_for_test(
            "sha256:context".to_owned(),
            "test-contract".to_owned(),
            BTreeMap::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("canonical test catalog");
        assert_eq!(catalog.catalog_id(), id);
        catalog.revalidate_identity().expect("canonical catalog");
    }

    #[test]
    fn forged_id_and_noncanonical_generated_projection_fail_closed() {
        assert!(
            catalog(
                format!("sha256:{}", "1".repeat(64)),
                GeneratedSurfaceIndex::new(Vec::new()),
            )
            .revalidate_identity()
            .is_err()
        );

        let id = catalog_identity_id(
            "sha256:context",
            "test-contract",
            &BTreeMap::new(),
            &[],
            &[],
        )
        .expect("catalog identity");
        let forged_projection = InventoryEntry {
            stable_id: "GEN:forged".to_owned(),
            kind: "generated-surface".to_owned(),
            owner_role: "OWN-TEST".to_owned(),
            relative_path: "generated/forged.json".to_owned(),
            digest_sha256: "0".repeat(64),
            unix_mode: None,
            authority_state: AuthorityState::Projection,
            active_status: ActiveStatus::Candidate,
            generator: Some("test".to_owned()),
            input_provenance: Vec::new(),
            references: Vec::new(),
        };
        assert!(
            catalog(id, GeneratedSurfaceIndex::new(vec![forged_projection]),)
                .revalidate_identity()
                .is_err()
        );
    }
}
