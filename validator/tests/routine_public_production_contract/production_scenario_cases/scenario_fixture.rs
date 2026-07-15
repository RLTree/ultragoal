use super::*;

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::env::var_os("HUL_KEEP_FIXTURES").is_some_and(|value| !value.is_empty()) {
            eprintln!("kept routine fixture at {}", self.container.display());
            return;
        }
        let _ = fs::remove_dir_all(&self.container);
    }
}

pub(crate) fn tree(root: &Path) -> BTreeMap<String, String> {
    fn visit(root: &Path, current: &Path, rows: &mut BTreeMap<String, String>) {
        if !current.exists() {
            return;
        }
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                rows.insert(relative, format!("dir:{:o}", metadata.mode()));
                visit(root, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.insert(
                    relative,
                    format!(
                        "link:{}",
                        sha(fs::read_link(&path).unwrap().as_os_str().as_encoded_bytes())
                    ),
                );
            } else if metadata.is_file() {
                rows.insert(
                    relative,
                    format!(
                        "file:{:o}:{}",
                        metadata.mode(),
                        sha(&fs::read(&path).unwrap())
                    ),
                );
            } else {
                rows.insert(relative, "special".to_owned());
            }
        }
    }
    let mut rows = BTreeMap::new();
    visit(root, root, &mut rows);
    rows
}

pub(crate) fn graph(nodes: &[NodeSpec], routes: &[RouteSpec]) -> ImpactGraph {
    ImpactGraph::new(
        nodes
            .iter()
            .map(|node| {
                CheckNode::new(
                    node.id,
                    node.dependencies.iter().map(|value| (*value).to_owned()),
                    CheckClass::Routine,
                    RunnerSpec::new("ultragoal", None).unwrap(),
                )
                .unwrap()
            })
            .collect(),
        routes
            .iter()
            .map(|route| {
                let path = RepoPath::parse(route.path.to_owned()).unwrap();
                PathRoute::new(
                    route.id,
                    match route.kind {
                        "exact" => PathMatcher::Exact(path),
                        "prefix" => PathMatcher::Prefix(path),
                        _ => panic!("unknown matcher"),
                    },
                    route.nodes.iter().map(|value| (*value).to_owned()),
                    false,
                )
                .unwrap()
            })
            .collect(),
        Vec::new(),
    )
    .unwrap()
}

pub(crate) fn catalog_bytes(nodes: &[NodeSpec], graph_id: &str) -> Vec<u8> {
    let routines = nodes
        .iter()
        .map(|node| {
            json!({
                "node_id": node.id,
                "behavior_id": "rust-source-syntax-v1",
                "depends_on": node.dependencies,
                "read_sources": node.read_sources,
                "timeout_ms": 60000,
                "output_budget_bytes": 1048576,
                "output_scopes": [format!("target/routine/{}", node.id)]
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&json!({
        "schema_version": "RoutineProductionCatalog-v2",
        "graph_id": graph_id,
        "routines": routines
    }))
    .unwrap()
}

pub(crate) fn manifest_bytes(nodes: &[NodeSpec], routes: &[RouteSpec], catalog: &[u8]) -> Vec<u8> {
    let nodes = nodes
        .iter()
        .map(|node| {
            json!({
                "node_id": node.id,
                "depends_on": node.dependencies,
                "behavior_id": "rust-source-syntax-v1",
                "read_sources": node.read_sources,
                "timeout_ms": 60000,
                "output_budget_bytes": 1048576
            })
        })
        .collect::<Vec<_>>();
    let routes = routes
        .iter()
        .map(|route| {
            json!({
                "row_id": route.id,
                "matcher": route.kind,
                "path": route.path,
                "node_ids": route.nodes,
                "requires_strict": false
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&json!({
        "schema_version": "RoutinePublicProduction-v2",
        "command": "check-routine",
        "catalog": {
            "path": "config/routines.json",
            "sha256": sha(catalog),
            "byte_length": catalog.len()
        },
        "nodes": nodes,
        "routes": routes
    }))
    .unwrap()
}
