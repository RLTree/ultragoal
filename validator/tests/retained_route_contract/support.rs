use crate::retained_routes::{
    ACTIVE_READER_WRITER_STATE, COMPATIBILITY_BEHAVIOR, COMPATIBILITY_BOUNDARY, CatalogEvidence,
    DigestEvidence, EQUIVALENCE_PROOF, EntryEvidence, INTENDED_DISPOSITION, MatcherEvidence,
    OBSERVED_AUTHORITY_STATE, PHYSICAL_CLEANUP_STATE, REPLACEMENT_STATE, RegistryRouteEvidence,
    RouteSpec, TransitionEvidence, VerificationError, VerifiedCatalog, routes, targets,
    verify_catalog,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(crate) fn source(spec: &'static RouteSpec) -> EntryEvidence<'static> {
    EntryEvidence {
        stable_id: spec.stable_id,
        kind: spec.kind,
        path: spec.path,
        sha256: spec.source_sha256,
        authority_state: "legacy",
        active_status: "active",
    }
}

pub(crate) fn target(spec: &'static RouteSpec) -> EntryEvidence<'static> {
    EntryEvidence {
        stable_id: spec.target.stable_id,
        kind: spec.target.kind,
        path: spec.target.path,
        sha256: spec.target.sha256,
        authority_state: "canonical",
        active_status: "definition",
    }
}

pub(crate) fn transition(spec: &'static RouteSpec) -> TransitionEvidence<'static> {
    TransitionEvidence {
        compatibility_behavior: COMPATIBILITY_BEHAVIOR,
        compatibility_boundary: COMPATIBILITY_BOUNDARY,
        replacement_state: REPLACEMENT_STATE,
        active_reader_writer_state: ACTIVE_READER_WRITER_STATE,
        observed_authority_state: OBSERVED_AUTHORITY_STATE,
        equivalence_proof: EQUIVALENCE_PROOF,
        physical_cleanup_state: PHYSICAL_CLEANUP_STATE,
        proof_refs: &spec.proof_refs,
    }
}

pub(crate) fn registry(spec: &'static RouteSpec) -> RegistryRouteEvidence<'static> {
    RegistryRouteEvidence {
        route_id: spec.route_id,
        matcher: MatcherEvidence {
            stable_id: Some(spec.stable_id),
            kind: Some(spec.kind),
            relative_path: Some(spec.path),
        },
        canonical_target: spec.target.stable_id,
        intended_disposition: INTENDED_DISPOSITION,
        transition: transition(spec),
    }
}

pub(crate) struct Fixture {
    pub sources: Vec<EntryEvidence<'static>>,
    pub targets: Vec<EntryEvidence<'static>>,
    pub registry: Vec<RegistryRouteEvidence<'static>>,
    pub digests: Vec<DigestEvidence<'static>>,
}

impl Fixture {
    pub(crate) fn baseline() -> Self {
        let sources = routes().map(source).collect();
        let targets = targets()
            .map(|target_spec| {
                let route = routes()
                    .find(|route| route.target.stable_id == target_spec.stable_id)
                    .unwrap();
                target(route)
            })
            .collect();
        let registry = routes().map(registry).collect();
        let digests = routes()
            .map(|spec| DigestEvidence {
                path: spec.path,
                sha256: spec.source_sha256,
            })
            .collect();
        Self {
            sources,
            targets,
            registry,
            digests,
        }
    }

    pub(crate) fn verify(&self) -> Result<VerifiedCatalog, VerificationError> {
        verify_catalog(CatalogEvidence {
            raw_sources: &self.sources,
            raw_targets: &self.targets,
            raw_registry_routes: &self.registry,
            current_sources: &self.digests,
        })
    }
}

pub(crate) fn repo_root() -> PathBuf {
    option_env!("CARGO_MANIFEST_DIR").map_or_else(
        || std::env::current_dir().unwrap(),
        |manifest_dir| PathBuf::from(manifest_dir).parent().unwrap().to_path_buf(),
    )
}

pub(crate) fn sha256(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    hex(&Sha256::digest(bytes))
}

pub(crate) fn canonical_definition_sha256(stable_id: &str) -> String {
    let root = repo_root();
    let (path, array, id_field) = if stable_id.starts_with("PS-") {
        (
            root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json"),
            "surfaces",
            "surface_id",
        )
    } else {
        (
            root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CUSTOM_TOOL_INVENTORY.json"),
            "tools",
            "tool_id",
        )
    };
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let definition = document[array]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row[id_field] == stable_id)
        .unwrap();
    hex(&Sha256::digest(serde_json::to_vec(definition).unwrap()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
