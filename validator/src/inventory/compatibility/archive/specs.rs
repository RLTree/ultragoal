pub(crate) const DECISION_PATH: &str =
    "docs/ultragoal-successor-live/root-decisions/OD-008-INTERNAL-LANE-READY-RETIREMENT.json";

const PS_ORCHESTRATION_PATH: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/PRODUCT_SURFACE_INVENTORY.json#/product-surface-definition/PS-ORCHESTRATION";
const HCT_CLAIMS_PATH: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CUSTOM_TOOL_INVENTORY.json#/custom-tool-definition/HCT-CLAIMS";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveRouteSpec {
    pub route_id: &'static str,
    pub stable_id: &'static str,
    pub kind: &'static str,
    pub source_path: &'static str,
    pub source_sha256: &'static str,
    pub canonical_target: &'static str,
    pub target_kind: &'static str,
    pub target_path: &'static str,
    pub target_sha256: &'static str,
}

impl ArchiveRouteSpec {
    pub fn proof_refs_match(&self, proof_refs: &[String]) -> bool {
        proof_refs.iter().map(String::as_str).eq([
            self.source_path,
            self.target_path,
            DECISION_PATH,
        ])
    }
}

const LANE_KIND: &str = "legacy-lane-authority";
const READY_KIND: &str = "legacy-finalizer-authority";
const PS_ORCHESTRATION: (&str, &str) = (
    PS_ORCHESTRATION_PATH,
    "05bf0adb779048ce18d60faae10de9e2453afb2fc188cb3d66e6e255e63d611e",
);
const HCT_CLAIMS: (&str, &str) = (
    HCT_CLAIMS_PATH,
    "26b4bb4683ebd01b04bb081e9f4213d4053aee53ce62f7484294c4bd1da01641",
);

const fn lane(
    route_id: &'static str,
    path: &'static str,
    sha256: &'static str,
) -> ArchiveRouteSpec {
    ArchiveRouteSpec {
        route_id,
        stable_id: "",
        kind: LANE_KIND,
        source_path: path,
        source_sha256: sha256,
        canonical_target: "PS-ORCHESTRATION",
        target_kind: "product-surface-definition",
        target_path: PS_ORCHESTRATION.0,
        target_sha256: PS_ORCHESTRATION.1,
    }
}

const fn ready(
    route_id: &'static str,
    path: &'static str,
    sha256: &'static str,
) -> ArchiveRouteSpec {
    ArchiveRouteSpec {
        route_id,
        stable_id: "",
        kind: READY_KIND,
        source_path: path,
        source_sha256: sha256,
        canonical_target: "HCT-CLAIMS",
        target_kind: "custom-tool-definition",
        target_path: HCT_CLAIMS.0,
        target_sha256: HCT_CLAIMS.1,
    }
}

pub(crate) const ROUTES: [ArchiveRouteSpec; 14] = [
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/actor/freshness.rs",
        ..lane(
            "lane-claim-actor-freshness-to-ps-orchestration",
            "validator/src/claim_semantics/lane/actor/freshness.rs",
            "bc60c7002409b2369f2eb56138bed9e06f18f2ade7724e0a1155e58c2b1ce406",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/actor/mod.rs",
        ..lane(
            "lane-claim-actor-root-to-ps-orchestration",
            "validator/src/claim_semantics/lane/actor/mod.rs",
            "c7a635362af7aaff94f6f74c7fbb4a289c65f59f43608f7277c33ab3fa2d6a63",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/dependency.rs",
        ..lane(
            "lane-claim-dependency-to-ps-orchestration",
            "validator/src/claim_semantics/lane/dependency.rs",
            "d98f6fbfa44bdfa264ae28003dcb2c4a2cdb205259728bfdf4754577459c54c3",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/isolation.rs",
        ..lane(
            "lane-claim-isolation-to-ps-orchestration",
            "validator/src/claim_semantics/lane/isolation.rs",
            "4122698decf54ef2dcbc662b11ae67fb82bea9052a6a8abf0f3515510679b69f",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/mod.rs",
        ..lane(
            "lane-claim-root-to-ps-orchestration",
            "validator/src/claim_semantics/lane/mod.rs",
            "265b59fa7c0c761cb521a9df870449140e3a62ba2b6c187d360f03575cfb021b",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/paths.rs",
        ..lane(
            "lane-claim-paths-to-ps-orchestration",
            "validator/src/claim_semantics/lane/paths.rs",
            "cd014198c1097cdd0dba43152a7cc3dc0a7e12d443fd0d5c686b6af585eebb03",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/policy.rs",
        ..lane(
            "lane-claim-policy-to-ps-orchestration",
            "validator/src/claim_semantics/lane/policy.rs",
            "5db0c04508950bc72cbf6443c8b43d96bdd3cc90b095342c30c0fd1a998e2377",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/root/mod.rs",
        ..lane(
            "lane-claim-root-module-to-ps-orchestration",
            "validator/src/claim_semantics/lane/root/mod.rs",
            "885589f6d7f1e1eebc9f773a13c82261790a964ed278030e6ddb3a6cd8f0ea12",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/root/scope.rs",
        ..lane(
            "lane-claim-root-scope-to-ps-orchestration",
            "validator/src/claim_semantics/lane/root/scope.rs",
            "5caf68f5e39ce579b22ea9c9f6585906366b98045475754918a3392e5dd3e28a",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/runtime/mod.rs",
        ..lane(
            "lane-claim-runtime-root-to-ps-orchestration",
            "validator/src/claim_semantics/lane/runtime/mod.rs",
            "419264b1da032f5edd148ac86ccc2b109c0d564d50508467c463bc11cd73a88f",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/runtime/state.rs",
        ..lane(
            "lane-claim-runtime-state-to-ps-orchestration",
            "validator/src/claim_semantics/lane/runtime/state.rs",
            "355d20951115bb4cff110abf5e5b64e52d112fc8cb70ff7fdba983932e9dad45",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-LANE:validator/src/claim_semantics/lane/status.rs",
        ..lane(
            "lane-claim-status-to-ps-orchestration",
            "validator/src/claim_semantics/lane/status.rs",
            "17b0fc92ff03f569306dbe00cf182f33fb8b25ea12be1368b80837aea03bcf69",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-FINALIZER:validator/src/claim_semantics/ready/mod.rs",
        ..ready(
            "finalizer-ready-module-to-hct-claims",
            "validator/src/claim_semantics/ready/mod.rs",
            "4768bedcdfbe4b23c948df184ea4ce099c1feb2101c83738b5eaac67850e27d2",
        )
    },
    ArchiveRouteSpec {
        stable_id: "LEGACY-FINALIZER:validator/src/claim_semantics/ready/receipt.rs",
        ..ready(
            "finalizer-ready-receipt-to-hct-claims",
            "validator/src/claim_semantics/ready/receipt.rs",
            "eee39ba3d3469f81279a29a08fd960341b6b246b83ad216da69e14f136dd2b17",
        )
    },
];

pub(crate) fn by_archive_route_id(route_id: &str) -> Option<&'static ArchiveRouteSpec> {
    ROUTES.iter().find(|route| route.route_id == route_id)
}
