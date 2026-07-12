use crate::context::LiveContext;
use crate::inventory::{AuthorityCatalog, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};
use serde_json::{Value, json};

fn manifest() -> Value {
    json!({
        "name": "harness-ultragoal",
        "version": "0.0.11",
        "description": "Evidence-bound repository work.",
        "author": {
            "name": "Terry Noblin",
            "email": "tree@terrynoblin.dev",
            "url": "https://terrynoblin.dev"
        },
        "homepage": "https://terrynoblin.dev/harness-ultragoal",
        "repository": "https://github.com/terrynoblin/harness-ultragoal",
        "license": "UNLICENSED",
        "keywords": ["agent-first", "verification"],
        "skills": "./skills/",
        "interface": {
            "displayName": "Harness Ultragoal",
            "shortDescription": "One evidence-bound front door.",
            "longDescription": "Route repository work through explicit authority.",
            "developerName": "Terry Noblin",
            "category": "Productivity",
            "capabilities": ["Read", "Write"],
            "websiteURL": "https://terrynoblin.dev/harness-ultragoal",
            "defaultPrompt": ["Inspect this repository safely."],
            "brandColor": "#3B82F6"
        }
    })
}

fn catalog(label: &str, value: &Value) -> AuthorityCatalog {
    let repo = TestRepo::new(label);
    repo.write(
        ".codex-plugin/plugin.json",
        &serde_json::to_vec(value).unwrap(),
    );
    repo.commit();
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn has(catalog: &AuthorityCatalog, code: &str) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == code)
}

#[test]
fn canonical_metadata_interface_and_url_issues_reach_inventory() {
    let cases: Vec<(&str, Box<dyn Fn(&mut Value) + Send>, &str)> = vec![
        (
            "name",
            Box::new(|v| v["name"] = json!("Bad Name")),
            "invalid_source_plugin_manifest",
        ),
        (
            "version",
            Box::new(|v| v["version"] = json!("01.2.3")),
            "invalid_source_plugin_manifest",
        ),
        (
            "placeholder",
            Box::new(|v| v["description"] = json!("TODO")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email",
            Box::new(|v| v["author"]["email"] = json!("tree@@terrynoblin.dev")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-short-ip",
            Box::new(|v| v["author"]["email"] = json!("tree@127.1")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-octal-ip",
            Box::new(|v| v["author"]["email"] = json!("tree@0177.1")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-example-net",
            Box::new(|v| v["author"]["email"] = json!("tree@example.net")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-sub-example-org",
            Box::new(|v| v["author"]["email"] = json!("tree@sub.example.org")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-home-arpa",
            Box::new(|v| v["author"]["email"] = json!("tree@home.arpa")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-sub-home-arpa",
            Box::new(|v| v["author"]["email"] = json!("tree@sub.home.arpa")),
            "invalid_source_plugin_manifest",
        ),
        (
            "email-home-arpa-case",
            Box::new(|v| v["author"]["email"] = json!("tree@HOME.ARPA")),
            "invalid_source_plugin_manifest",
        ),
        (
            "url",
            Box::new(|v| v["homepage"] = json!("https://127.0.0.1")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-short-ip",
            Box::new(|v| v["homepage"] = json!("https://127.1")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-octal-ip",
            Box::new(|v| v["homepage"] = json!("https://0177.1")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-hex-ip",
            Box::new(|v| v["homepage"] = json!("https://0x7f.1")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-example-net",
            Box::new(|v| v["homepage"] = json!("https://example.net")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-sub-example-org",
            Box::new(|v| v["homepage"] = json!("https://sub.example.org")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-home-arpa",
            Box::new(|v| v["homepage"] = json!("https://home.arpa")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-sub-home-arpa",
            Box::new(|v| v["homepage"] = json!("https://sub.home.arpa")),
            "invalid_plugin_manifest_url",
        ),
        (
            "url-home-arpa-case-port",
            Box::new(|v| v["homepage"] = json!("https://HOME.ARPA:443/path")),
            "invalid_plugin_manifest_url",
        ),
        (
            "prompt-empty",
            Box::new(|v| v["interface"]["defaultPrompt"] = json!([])),
            "invalid_plugin_default_prompt",
        ),
        (
            "prompt-host-limit",
            Box::new(|v| v["interface"]["defaultPrompt"] = json!(["one", "two", "three", "four"])),
            "plugin_default_prompt_truncated",
        ),
        (
            "brand",
            Box::new(|v| v["interface"]["brandColor"] = json!("blue")),
            "invalid_plugin_brand_color",
        ),
    ];
    std::thread::scope(|scope| {
        for (label, mutate, expected) in cases {
            scope.spawn(move || {
                let mut value = manifest();
                mutate(&mut value);
                assert!(
                    has(&catalog(label, &value), expected),
                    "missing {expected} for {label}"
                );
            });
        }
    });
}

#[test]
fn canonical_mcp_semantics_reject_every_consumer_false_pass() {
    let invalid = [
        json!({"Bad Name":{"type":"bogus","url":"http://insecure"}}),
        json!({"server":{"type":"unknown","url":"https://mcp.terrynoblin.dev"}}),
        json!({"server":{"type":"http","url":"http://mcp.terrynoblin.dev"}}),
        json!({"server":{"type":"http","url":"https://user@mcp.terrynoblin.dev"}}),
        json!({"server":{"type":"http","url":"https://127.0.0.1"}}),
        json!({"server":{"type":"http","url":"https://mcp.example.com"}}),
        json!({"server":{"type":"http","command":"./bin/server"}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","args":["bad"]}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","env":{"MODE":"bad"}}}),
        json!({"server":{"type":"stdio"}}),
        json!({"server":{"type":"stdio","command":""}}),
        json!({"server":{"type":"stdio","command":"./bin/server","url":"https://mcp.terrynoblin.dev"}}),
        json!({"server":{"type":"stdio","command":"./bin/server","headers":{"X":"bad"}}}),
        json!({"server":{"type":"stdio","command":"./bin/server","args":["MODE", "mode"]}}),
        json!({"server":{"type":"stdio","command":"./bin/server","env":{"BAD KEY":"bad"}}}),
        json!({"server":{"type":"stdio","command":"./bin/server","env":{"A=B":"bad"}}}),
        json!({"server":{"type":"stdio","command":"./bin/server","env":{"A:B":"bad"}}}),
        json!({"server":{"type":"stdio","command":"./bin/server","env":{"BAD\r\nKEY":"bad"}}}),
        json!({"server":{"type":"stdio","command":"./bin/server","env":{"MODE":"one","mode":"two"}}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X Header":"bad"}}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X=Header":"bad"}}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X:Header":"bad"}}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X\r\nHeader":"bad"}}}),
        json!({"server":{"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X-Mode":"one","x-mode":"two"}}}),
    ];
    std::thread::scope(|scope| {
        for (index, servers) in invalid.into_iter().enumerate() {
            scope.spawn(move || {
                let mut value = manifest();
                value["mcpServers"] = servers;
                assert!(
                    has(
                        &catalog(&format!("mcp-semantic-{index}"), &value),
                        "invalid_plugin_mcp_server"
                    ),
                    "MCP case {index} false-passed inventory"
                );
            });
        }
    });
}

#[test]
fn canonical_supported_stdio_and_remote_mcp_semantics_pass_inventory() {
    let mut value = manifest();
    value["author"]["email"] = json!("tree@home.arpa.example.dev");
    value["homepage"] = json!("https://home.arpa.example.dev");
    value["mcpServers"] = json!({
        "local-server": {"type":"stdio","command":"./bin/server","args":["--serve"],"env":{"_MODE_2":"test"}},
        "remote-server": {"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X-Mode":"test","X_Proof":"yes"}}
    });
    let catalog = catalog("mcp-semantic-positive", &value);
    assert!(!has(&catalog, "invalid_source_plugin_manifest"));
    assert!(!has(&catalog, "invalid_plugin_manifest_url"));
    assert!(!has(&catalog, "invalid_plugin_mcp_server"));
}
