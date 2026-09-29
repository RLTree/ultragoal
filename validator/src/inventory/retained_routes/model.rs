pub(crate) const COMPATIBILITY_BEHAVIOR: &str = "unverified";
pub(crate) const COMPATIBILITY_BOUNDARY: &str = "blocked-by-OD-008";
pub(crate) const REPLACEMENT_STATE: &str = "unverified";
pub(crate) const ACTIVE_READER_WRITER_STATE: &str = "active";
pub(crate) const OBSERVED_AUTHORITY_STATE: &str = "active";
pub(crate) const EQUIVALENCE_PROOF: &str = "missing";
pub(crate) const PHYSICAL_CLEANUP_STATE: &str = "blocked-by-OD-009";
pub(crate) const INTENDED_DISPOSITION: &str = "non-authoritative";
pub(crate) const RESULT_PATH: &str =
    "docs/ultragoal-successor-live/root-decisions/N02-SOLE-CURRENT-PENDING-MIGRATION.json";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TargetSpec {
    pub stable_id: &'static str,
    pub kind: &'static str,
    pub path: &'static str,
    pub sha256: &'static str,
}

pub(crate) const PS_CLI: TargetSpec = TargetSpec {
    stable_id: "PS-CLI",
    kind: "product-surface-definition",
    path: "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-CLI",
    sha256: "dce0b2e93886ea1a4edfcd757fe45dee3997c2314f77f85c03d317f83b864148",
};

pub(crate) const PS_ORCHESTRATION: TargetSpec = TargetSpec {
    stable_id: "PS-ORCHESTRATION",
    kind: "product-surface-definition",
    path: "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-ORCHESTRATION",
    sha256: "05bf0adb779048ce18d60faae10de9e2453afb2fc188cb3d66e6e255e63d611e",
};

pub(crate) const HCT_CLAIMS: TargetSpec = TargetSpec {
    stable_id: "HCT-CLAIMS",
    kind: "custom-tool-definition",
    path: "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CUSTOM_TOOL_INVENTORY.json#/custom-tool-definition/HCT-CLAIMS",
    sha256: "26b4bb4683ebd01b04bb081e9f4213d4053aee53ce62f7484294c4bd1da01641",
};

pub(crate) const PS_PLUGIN_MANIFEST: TargetSpec = TargetSpec {
    stable_id: "PS-PLUGIN-MANIFEST",
    kind: "product-surface-definition",
    path: "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-PLUGIN-MANIFEST",
    sha256: "e8d3e1e51955752f05f55416ebc67ebb8437b07fc47b9eacc1f91171692dac6e",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RouteSpec {
    pub route_id: &'static str,
    pub stable_id: &'static str,
    pub kind: &'static str,
    pub path: &'static str,
    pub source_sha256: &'static str,
    pub target: &'static TargetSpec,
    pub proof_refs: [&'static str; 3],
}

pub(crate) const fn spec(
    route_id: &'static str,
    stable_id: &'static str,
    kind: &'static str,
    path: &'static str,
    source_sha256: &'static str,
    target: &'static TargetSpec,
) -> RouteSpec {
    RouteSpec {
        route_id,
        stable_id,
        kind,
        path,
        source_sha256,
        target,
        proof_refs: [path, target.path, RESULT_PATH],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EntryEvidence<'a> {
    pub stable_id: &'a str,
    pub kind: &'a str,
    pub path: &'a str,
    pub sha256: &'a str,
    pub authority_state: &'a str,
    pub active_status: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DigestEvidence<'a> {
    pub path: &'a str,
    pub sha256: &'a str,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MatcherEvidence<'a> {
    pub stable_id: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub relative_path: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TransitionEvidence<'a> {
    pub compatibility_behavior: &'a str,
    pub compatibility_boundary: &'a str,
    pub replacement_state: &'a str,
    pub active_reader_writer_state: &'a str,
    pub observed_authority_state: &'a str,
    pub equivalence_proof: &'a str,
    pub physical_cleanup_state: &'a str,
    pub proof_refs: &'a [&'a str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RegistryRouteEvidence<'a> {
    pub route_id: &'a str,
    pub matcher: MatcherEvidence<'a>,
    pub canonical_target: &'a str,
    pub intended_disposition: &'a str,
    pub transition: TransitionEvidence<'a>,
}

pub(crate) struct CatalogEvidence<'a> {
    /// Unmerged rows from the exact four pending-migration source classes.
    pub raw_sources: &'a [EntryEvidence<'a>],
    /// Unmerged definition rows for the four canonical target identities.
    pub raw_targets: &'a [EntryEvidence<'a>],
    /// Unmerged exact route rows; broad or duplicate rows are not normalized.
    pub raw_registry_routes: &'a [RegistryRouteEvidence<'a>],
    /// Digests read from current source bytes in the same candidate session.
    pub current_sources: &'a [DigestEvidence<'a>],
}
