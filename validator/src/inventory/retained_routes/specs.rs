use super::model::{
    HCT_CLAIMS, PS_CLI, PS_ORCHESTRATION, PS_PLUGIN_MANIFEST, RouteSpec, TargetSpec,
};

pub(crate) const ROUTE_COUNT: usize = 33;
pub(crate) const TARGET_COUNT: usize = 4;

pub(crate) fn routes() -> impl Iterator<Item = &'static RouteSpec> {
    super::commands::ROUTES
        .iter()
        .chain(super::lanes::ROUTES.iter())
        .chain(super::claims::ROUTES.iter())
        .chain(super::manifest::ROUTES.iter())
}

pub(crate) fn by_route_id(route_id: &str) -> Option<&'static RouteSpec> {
    routes().find(|route| route.route_id == route_id)
}

pub(crate) fn by_stable_id(stable_id: &str) -> Option<&'static RouteSpec> {
    routes().find(|route| route.stable_id == stable_id)
}

pub(crate) fn is_source_kind(kind: &str) -> bool {
    matches!(
        kind,
        "legacy-command-authority"
            | "legacy-lane-authority"
            | "legacy-finalizer-authority"
            | "legacy-manifest-projection-authority"
    )
}

pub(crate) fn is_target_id(stable_id: &str) -> bool {
    targets().any(|target| target.stable_id == stable_id)
}

pub(crate) fn targets() -> impl Iterator<Item = &'static TargetSpec> {
    [&PS_CLI, &PS_ORCHESTRATION, &HCT_CLAIMS, &PS_PLUGIN_MANIFEST].into_iter()
}
