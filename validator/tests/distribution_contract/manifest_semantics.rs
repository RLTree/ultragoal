use crate::distribution::{DistributionErrorId as ErrorId, plan_package};
use crate::package_manifest::{add, error, manifest, spec_value, write_manifest, write_sources};
use crate::support::Fixture;
use serde_json::{Value, json};

#[test]
fn canonical_metadata_interface_and_url_issues_reach_distribution() {
    let fixture = Fixture::complete("manifest-semantics-metadata");
    write_sources(&fixture);
    let spec = spec_value(false);
    let cases: Vec<Box<dyn Fn(&mut Value)>> = vec![
        Box::new(|v| v["name"] = json!("Bad Name")),
        Box::new(|v| v["version"] = json!("01.2.3")),
        Box::new(|v| v["description"] = json!("TODO")),
        Box::new(|v| v["author"]["email"] = json!("tree@@terrynoblin.dev")),
        Box::new(|v| v["author"]["email"] = json!("tree@127.1")),
        Box::new(|v| v["author"]["email"] = json!("tree@0177.1")),
        Box::new(|v| v["author"]["email"] = json!("tree@example.net")),
        Box::new(|v| v["author"]["email"] = json!("tree@sub.example.org")),
        Box::new(|v| v["author"]["email"] = json!("tree@home.arpa")),
        Box::new(|v| v["author"]["email"] = json!("tree@sub.home.arpa")),
        Box::new(|v| v["author"]["email"] = json!("tree@HOME.ARPA")),
        Box::new(|v| v["homepage"] = json!("https://127.0.0.1")),
        Box::new(|v| v["homepage"] = json!("https://127.1")),
        Box::new(|v| v["homepage"] = json!("https://0177.1")),
        Box::new(|v| v["homepage"] = json!("https://0x7f.1")),
        Box::new(|v| v["homepage"] = json!("https://example.net")),
        Box::new(|v| v["homepage"] = json!("https://sub.example.org")),
        Box::new(|v| v["homepage"] = json!("https://home.arpa")),
        Box::new(|v| v["homepage"] = json!("https://sub.home.arpa")),
        Box::new(|v| v["homepage"] = json!("https://HOME.ARPA:443/path")),
        Box::new(|v| v["interface"]["defaultPrompt"] = json!([])),
        Box::new(|v| v["interface"]["defaultPrompt"] = json!(["one", "two", "three", "four"])),
        Box::new(|v| v["interface"]["brandColor"] = json!("blue")),
    ];
    for mutate in cases {
        let mut value = manifest();
        mutate(&mut value);
        assert_eq!(error(&fixture, &value, &spec), ErrorId::InvalidSpec);
    }
}

#[test]
fn canonical_mcp_semantics_reject_every_distribution_false_pass() {
    let fixture = Fixture::complete("manifest-semantics-mcp");
    write_sources(&fixture);
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
    for servers in invalid {
        let mut value = manifest();
        value["mcpServers"] = servers;
        assert_eq!(
            error(&fixture, &value, &spec_value(false)),
            ErrorId::InvalidSpec
        );
    }
}

#[test]
fn supported_mcp_shapes_pass_semantics_and_bind_package_content() {
    let fixture = Fixture::complete("manifest-semantics-positive");
    write_sources(&fixture);
    let mut value = manifest();
    value["author"]["email"] = json!("tree@home.arpa.example.dev");
    value["homepage"] = json!("https://home.arpa.example.dev");
    value["mcpServers"] = json!({
        "local-server": {"type":"stdio","command":"./bin/server","args":["--serve"],"env":{"_MODE_2":"test"}},
        "remote-server": {"type":"http","url":"https://mcp.terrynoblin.dev","headers":{"X-Mode":"test","X_Proof":"yes"}}
    });
    write_manifest(&fixture, &value);
    let mut spec = spec_value(false);
    add(&fixture, &mut spec, "bin/server", "executable", true);
    plan_package(&fixture.root, &serde_json::to_vec(&spec).unwrap()).unwrap();
}

#[test]
fn ambient_stdio_command_is_rejected_only_at_package_reachability_boundary() {
    let fixture = Fixture::complete("manifest-semantics-ambient-command");
    write_sources(&fixture);
    let mut value = manifest();
    value["mcpServers"] = json!({"server":{"type":"stdio","command":"node"}});
    assert_eq!(
        error(&fixture, &value, &spec_value(false)),
        ErrorId::InvalidSpec
    );
}
