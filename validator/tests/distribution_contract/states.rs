use serde_json::json;
use std::path::Path;

use crate::distribution::{Capability, HostVerdict, JoinVerdict, Layer, LayerVerdict, verify};
use crate::support::{Fixture, current_platform};

#[test]
fn definition_missing_and_unavailable_states_are_not_promoted() {
    let mut fixture = Fixture::complete("explicit-states");
    *fixture.layer_row_mut(Layer::SourcePluginMetadata) = json!({
        "layer": Layer::SourcePluginMetadata.as_str(),
        "state": "definition-only",
        "expected": {
            "plugin_id": "harness-ultragoal",
            "version": "0.0.11",
            "artifact_sha256": null
        }
    });
    *fixture.layer_row_mut(Layer::AppRegistryUi) = json!({
        "layer": Layer::AppRegistryUi.as_str(),
        "state": "missing"
    });
    *fixture.capability_row_mut(Capability::Runtime) = json!({
        "capability": Capability::Runtime.as_str(),
        "state": "unavailable"
    });
    *fixture.layer_row_mut(Layer::Runtime) = json!({
        "layer": Layer::Runtime.as_str(),
        "state": "unavailable",
        "capability": Capability::Runtime.as_str(),
        "reason_id": "runtime-api-unavailable"
    });

    let report = verify(&fixture.root, &fixture.bytes()).expect("explicit state report");

    assert!(!report.accepted());
    assert_eq!(
        report.layer(Layer::SourcePluginMetadata).verdict(),
        LayerVerdict::DefinitionOnly
    );
    assert_eq!(
        report.layer(Layer::SourcePackageInput).verdict(),
        LayerVerdict::ObservedUnjoined
    );
    assert_eq!(
        report.layer(Layer::AppRegistryUi).verdict(),
        LayerVerdict::Missing
    );
    assert_eq!(
        report.layer(Layer::Runtime).verdict(),
        LayerVerdict::Unavailable
    );
    assert!(
        report
            .joins()
            .iter()
            .any(|join| join.verdict() == JoinVerdict::NotEvaluated)
    );
}

#[test]
fn unsupported_platform_is_an_explicit_report_not_simulated_proof() {
    let mut request = crate::support::request();
    request["host"]["platform"] = json!(if current_platform() == "windows" {
        "linux"
    } else {
        "windows"
    });
    for row in request["host"]["capabilities"].as_array_mut().unwrap() {
        row["state"] = json!("unavailable");
    }
    for row in request["layers"].as_array_mut().unwrap() {
        let layer = Layer::parse(row["layer"].as_str().unwrap()).unwrap();
        *row = json!({
            "layer": layer.as_str(),
            "state": "unavailable",
            "capability": layer.capability().as_str(),
            "reason_id": "platform-unavailable"
        });
    }
    let bytes = serde_json::to_vec(&request).unwrap();

    let report = verify(Path::new("/missing/platform-root-canary"), &bytes)
        .expect("platform-unavailable report");

    assert_eq!(report.host_verdict(), HostVerdict::PlatformUnavailable);
    assert!(!report.accepted());
    assert!(
        Layer::ALL
            .iter()
            .all(|layer| report.layer(*layer).verdict() == LayerVerdict::Unavailable)
    );
}
