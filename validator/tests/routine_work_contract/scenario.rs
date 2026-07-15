pub use super::routine_fixture_repository::{TempRepo, sha};
use super::routine_work::{
    CheckClass, CheckNode, ClaimBoundary, ImpactGraph, PathMatcher, PathRoute, RepoPath, RunnerSpec,
};

pub fn graph() -> ImpactGraph {
    graph_with_order(false)
}

pub fn graph_with_order(reverse: bool) -> ImpactGraph {
    let mut nodes = vec![
        node("syntax", &[], CheckClass::Routine, "git", None),
        node("compile", &["syntax"], CheckClass::Routine, "git", None),
        node("unit", &["compile"], CheckClass::Routine, "git", None),
        node(
            "release",
            &["unit"],
            CheckClass::ClaimBoundaryOnly,
            "git",
            None,
        ),
    ];
    let mut routes = vec![
        route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        ),
        route(
            "route-tests",
            PathMatcher::Prefix(path("tests")),
            &["unit"],
            false,
        ),
        route(
            "route-docs",
            PathMatcher::Prefix(path("docs")),
            &["syntax"],
            false,
        ),
        route(
            "route-release",
            PathMatcher::Exact(path("release.json")),
            &["release"],
            true,
        ),
    ];
    let mut claims = vec![
        ClaimBoundary::new("routine", vec!["unit".to_owned()]).unwrap(),
        ClaimBoundary::new("release", vec!["release".to_owned()]).unwrap(),
    ];
    if reverse {
        nodes.reverse();
        routes.reverse();
        claims.reverse();
    }
    ImpactGraph::new(nodes, routes, claims).unwrap()
}

pub fn fallback_graph() -> ImpactGraph {
    ImpactGraph::new(
        vec![node(
            "compile",
            &[],
            CheckClass::Routine,
            "definitely-missing-routine-accelerator",
            Some("git"),
        )],
        vec![route(
            "route-src",
            PathMatcher::Prefix(path("src")),
            &["compile"],
            false,
        )],
        Vec::new(),
    )
    .unwrap()
}

pub fn node(
    id: &str,
    dependencies: &[&str],
    class: CheckClass,
    primary: &str,
    fallback: Option<&str>,
) -> CheckNode {
    CheckNode::new(
        id,
        dependencies.iter().map(|value| (*value).to_owned()),
        class,
        RunnerSpec::new(primary, fallback.map(ToOwned::to_owned)).unwrap(),
    )
    .unwrap()
}

pub fn route(id: &str, matcher: PathMatcher, nodes: &[&str], strict: bool) -> PathRoute {
    PathRoute::new(
        id,
        matcher,
        nodes.iter().map(|value| (*value).to_owned()),
        strict,
    )
    .unwrap()
}

pub fn path(value: &str) -> RepoPath {
    RepoPath::parse(value).unwrap()
}
