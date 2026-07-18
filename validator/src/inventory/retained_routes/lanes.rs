use super::model::{PS_ORCHESTRATION, RouteSpec, spec};

const KIND: &str = "legacy-lane-authority";

pub(super) const ROUTES: [RouteSpec; 2] = [
    spec(
        "lane-root-registry-to-ps-orchestration",
        "LEGACY-LANE:LANE_REGISTRY.json",
        KIND,
        "LANE_REGISTRY.json",
        "bdd68d238b33a36497b02a5ab0b5d74845f3a24d033a9751f913793ec6be812b",
        &PS_ORCHESTRATION,
    ),
    spec(
        "lane-template-registry-to-ps-orchestration",
        "LEGACY-LANE:templates/LANE_REGISTRY.json",
        KIND,
        "templates/LANE_REGISTRY.json",
        "130c6c0c237468706ee8d7e7b492f71e5dfc4906d5b67bc661d16d9b8732e324",
        &PS_ORCHESTRATION,
    ),
];
