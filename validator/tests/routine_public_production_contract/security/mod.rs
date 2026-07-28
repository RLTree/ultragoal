use super::scenario::{Fixture, pass_node, prefix_route, tree};
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::symlink;

mod admission;
mod checkpoint;
mod substitution;

fn assert_diagnostic(output: &std::process::Output, id: &str, fixture: &Fixture) {
    assert_ne!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "diagnostic is not JSON: {error}; stderr={:?}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
    assert_eq!(value["diagnostic_id"], id);
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(!text.contains(fixture.root.to_str().unwrap()));
    assert!(!text.contains(fixture.home.to_str().unwrap()));
}
