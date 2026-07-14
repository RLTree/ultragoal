pub fn catalog_bytes(
    source: &SourceAgentCatalog,
    request: &HostAgentAuthorityRequest,
    layer: AgentAuthorityLayer,
    global_agents: Vec<Value>,
) -> Vec<u8> {
    let authority_root_sha256 = digest(format!("fixture-root:{layer:?}").as_bytes());
    let authority_generation_sha256 = digest(b"fixture-generation:7");
    let agents = if layer == AgentAuthorityLayer::Global {
        global_agents
    } else {
        source
            .canonical_agents()
            .into_iter()
            .map(|row| {
                let descriptor =
                    std::str::from_utf8(source.descriptor_bytes(row.name()).expect("descriptor"))
                        .unwrap();
                json!({
                    "name": row.name(),
                    "manifest_path": row.manifest_path(),
                    "descriptor_sha256": row.descriptor_sha256(),
                    "descriptor_toml": descriptor,
                    "file_kind": "regular",
                    "link_count": 1
                })
            })
            .collect()
    };
    serde_json::to_vec(&json!({
        "schema_version": "HostAgentAuthorityCatalog-v1",
        "layer": serde_json::to_value(layer).unwrap(),
        "plugin_name": "harness-ultragoal",
        "plugin_version": source.plugin_version(),
        "plugin_manifest_sha256": source.plugin_manifest_sha256(),
        "plugin_manifest_json": std::str::from_utf8(source.plugin_manifest_bytes()).unwrap(),
        "project_root_sha256": request.project_root_sha256(),
        "candidate_id": request.candidate_id(),
        "session_id": request.session_id(),
        "session_issuance_sha256": request.session_issuance_sha256(),
        "observation_nonce_sha256": request.observation_nonce_sha256(),
        "authority_root_sha256": authority_root_sha256,
        "authority_generation_sha256": authority_generation_sha256,
        "transaction_provenance_sha256": request.provenance_sha256(),
        "new_session": true,
        "agents": agents
    }))
    .unwrap()
}

pub fn replace_source_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
}

pub fn tree_snapshot(root: &Path) -> Vec<(String, String)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(|entry| entry.unwrap())
        .map(|entry| {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let kind = entry.file_type();
            let identity = if kind.is_file() {
                digest(&fs::read(entry.path()).unwrap())
            } else if kind.is_dir() {
                "directory".to_owned()
            } else if kind.is_symlink() {
                format!(
                    "symlink:{}",
                    fs::read_link(entry.path()).unwrap().to_string_lossy()
                )
            } else {
                "special".to_owned()
            };
            (relative, identity)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
