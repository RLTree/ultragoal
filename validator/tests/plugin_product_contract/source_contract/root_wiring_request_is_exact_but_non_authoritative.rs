#[test]
fn root_wiring_request_is_exact_but_non_authoritative() {
    let request = read("fixtures/plugin-product/root-wiring-request.json");
    assert!(request.contains("\"authoritative\": false"));
    assert!(request.contains("\"value\": \"0.0.12\""));
    assert!(request.contains("./plugins/harness-ultragoal"));
    assert!(request.contains("path_absent"));
    let skills = request
        .split_once("\"pointer\": \"/skills\"")
        .unwrap_or_else(|| panic!("skills operation missing"))
        .1
        .split_once("\"pointer\": \"/purpose\"")
        .unwrap_or_else(|| panic!("purpose operation missing"))
        .0;
    for skill in SKILLS {
        assert_eq!(
            skills.matches(&format!("\"name\": \"{skill}\"")).count(),
            1,
            "root request skill {skill}"
        );
    }
    for agent in AGENTS {
        assert!(request.contains(&format!("\"{}\"", agent.name)));
        assert!(request.contains(&agent.path()));
    }
    assert_eq!(
        read(".codex-plugin/plugin.json").matches("0.0.11").count(),
        1
    );
    assert!(!root().join(".agents/plugins/marketplace.json").exists());
}

#[test]
fn supported_host_wiring_is_root_owned_and_preserves_each_proof_surface() {
    let request: serde_json::Value =
        serde_json::from_str(&read("fixtures/plugin-product/root-wiring-request.json"))
            .unwrap_or_else(|error| panic!("root wiring request invalid: {error}"));
    assert_eq!(
        request["root_owned_wiring_requests"],
        serde_json::json!([
            {
                "id": "plugin-product-host-lifecycle-module",
                "path": "validator/src/plugin_product/mod.rs",
                "preimage_sha256": "sha256:b12632b6f0a1363adda7ae2d87d568cb24c202c6a29af57bc120e25af97e98b8",
                "requested_change": "Declare `pub(crate) mod host_lifecycle;` so the candidate-closed supported-host transaction and its crate-visible adapter binding are compiled only for in-crate root adapters.",
                "required_guard": "Do not make the module or adapter externally public. Preserve the existing typed package, confined-root, lifecycle, capability, and claim boundaries."
            },
            {
                "id": "successor-supported-host-dispatch",
                "path": "validator/src/cli/successor_public/mod.rs",
                "preimage_sha256": "sha256:29bf29fc044e7eba2f48d6c3cecfe8fb7baad68b404cf7e334744c9a62006795",
                "requested_change": "Add one typed successor adapter only after it consumes a root-issued distribution effect permit and the exact in-crate host lifecycle binding; read-only diagnosis must remain zero-write and all effectful requests must fail closed when host capability is absent.",
                "required_guard": "No direct CLI construction of DarwinHostTransactionAdapter, no package/install/runtime claim effect, and no fallback from unsupported host behavior to source-only success."
            },
            {
                "id": "package-install-registry-reconciliation",
                "paths": ["plugin-manifest-draft.json", "validator/src/distribution/", "validator/src/cli/successor/catalog/mod.rs"],
                "requested_change": "After the module and dispatcher are integrated, recompute package membership and package bytes, then bind install, cache, marketplace, app-registry, Plugins UI, discovery, and runtime observations separately to that exact candidate.",
                "required_guard": "This request authorizes no package, install, registry, discovery, or runtime claim. Each surface requires fresh same-surface proof."
            }
        ])
    );
}
