use super::*;

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::env::var_os("HUL_KEEP_FIXTURES").is_some() {
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

pub(crate) fn wait_for_started(authority: &Path) {
    let state = authority.join("routine-authority.state");
    // Initial owner-only ledger creation and fsync can exceed five seconds on
    // a loaded Darwin host. Wait for the durable Started phase, not a timing
    // assumption about how quickly the authority files reach disk.
    for _ in 0..3_000 {
        if fs::read(&state)
            .ok()
            .is_some_and(|bytes| String::from_utf8_lossy(&bytes).contains("started"))
        {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("routine authority did not reach started state");
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
                    RunnerSpec::new(node.primary, node.fallback.map(str::to_owned)).unwrap(),
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
            let primary = runner_recipe(node.primary, node);
            let fallback = node.fallback.map(|tool| {
                let mut value = runner_recipe(tool, node);
                value["equivalence"] = Value::String("same-node-semantics-v1".to_owned());
                value
            });
            let mut value = json!({
                "definition_id": format!("routine.{}.v1", node.id),
                "node_id": node.id,
                "depends_on": node.dependencies,
                "working_directory": ".",
                "runner_policy": "immutable-single-process-exact-executable-v1",
                "read_policy": "selected-transitive-exact-regular-files-v1",
                "read_sources": node.read_sources,
                "environment": {},
                "timeout_ms": 60000,
                "output_budget_bytes": 1048576,
                "output_scopes": [format!("target/routine/{}", node.id)],
                "primary": primary
            });
            if let Some(fallback) = fallback {
                value["fallback"] = fallback;
            }
            value
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&json!({
        "schema_version": "RoutineProductionCatalog-v1",
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
                "primary_tool": node.primary,
                "fallback_tool": node.fallback,
                "action": node.action,
                "delay_seconds": node.delay_seconds,
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
        "schema_version": "RoutinePublicProduction-v1",
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

pub(crate) fn runner_recipe(tool: &str, node: &NodeSpec) -> Value {
    let capability = capability(tool);
    let executable = capability
        .executable
        .clone()
        .unwrap_or_else(|| format!("/bin/{tool}"));
    json!({
        "tool": tool,
        "tool_identity_sha256": sha(&serde_json::to_vec(&capability).unwrap()),
        "executable_path": executable,
        "program_sha256": capability.executable_sha256.as_deref().map(|value| format!("sha256:{value}")).unwrap_or_else(|| format!("sha256:{}", "0".repeat(64))),
        "program_byte_length": capability.byte_length.unwrap_or(1),
        "program_unix_mode": capability.unix_mode.unwrap_or(0o100755),
        "arguments": canonical_arguments(node)
    })
}

pub(crate) fn capability(tool: &str) -> ToolCapability {
    let path = PathBuf::from(format!("/bin/{tool}"));
    let Ok(executable) = fs::canonicalize(&path) else {
        return ToolCapability {
            name: tool.to_owned(),
            available: false,
            executable: None,
            executable_sha256: None,
            byte_length: None,
            unix_mode: None,
        };
    };
    let metadata = fs::metadata(&executable).unwrap();
    ToolCapability {
        name: tool.to_owned(),
        available: true,
        executable: Some(executable.to_str().unwrap().to_owned()),
        executable_sha256: Some(raw_sha(&fs::read(&executable).unwrap())),
        byte_length: Some(metadata.len()),
        unix_mode: Some(metadata.permissions().mode()),
    }
}
