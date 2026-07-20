use super::lane_binding::load_declared_dependency_identities;

const LANES: &[u8] = include_bytes!("../../../../LANE_REGISTRY.json");

#[test]
fn current_external_blocker_and_dependency_identities_are_accepted() {
    let identities = load_declared_dependency_identities(LANES).unwrap();
    assert_eq!(identities.len(), 7);
}

#[test]
fn authority_and_identity_mutations_fail_closed() {
    let original: serde_json::Value = serde_json::from_slice(LANES).unwrap();
    for mutation in 0..5 {
        let mut value = original.clone();
        let lanes = value["lanes"].as_array_mut().unwrap();
        match mutation {
            0 => {
                lane(lanes, "N11")["outcome"]["execution_outcome"] =
                    serde_json::json!("demonstrated")
            }
            1 => lane(lanes, "N11")["current_identity"]["tree"] = serde_json::json!("A".repeat(40)),
            2 => {
                lane(lanes, "N12")["state"] = serde_json::json!("ready");
            }
            3 => {
                lane(lanes, "N12")["dependencies"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => {
                let duplicate = lanes
                    .iter()
                    .find(|lane| lane["id"] == "N03")
                    .unwrap()
                    .clone();
                lanes.push(duplicate);
            }
        }
        assert!(load_declared_dependency_identities(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

fn lane<'a>(lanes: &'a mut [serde_json::Value], id: &str) -> &'a mut serde_json::Value {
    lanes.iter_mut().find(|lane| lane["id"] == id).unwrap()
}

#[test]
fn declared_staged_state_is_accepted_without_promoting_n11() {
    let mut value: serde_json::Value = serde_json::from_slice(LANES).unwrap();
    let n12 = value["lanes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|lane| lane["id"] == "N12")
        .unwrap();
    n12["state"] = serde_json::json!("integrating");
    n12["ceiling"] = serde_json::json!("source_accepted");
    n12["current_identity"] = serde_json::json!({
        "lane_id": "N12",
        "commit": "0123456789abcdef0123456789abcdef01234567",
        "tree": "89abcdef0123456789abcdef0123456789abcdef"
    });
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(load_declared_dependency_identities(&bytes).is_ok());
}

#[test]
fn invalidated_staging_identity_remains_declared_without_restoring_its_ceiling() {
    let original: serde_json::Value = serde_json::from_slice(LANES).unwrap();
    let n12 = original["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|lane| lane["id"] == "N12")
        .unwrap();
    assert_eq!(n12["state"], "blocked");
    assert_eq!(n12["ceiling"], "adopted_reobservation_required");
    assert!(n12["current_identity"].is_object());
    assert!(load_declared_dependency_identities(LANES).is_ok());

    for mutation in 0..5 {
        let mut value = original.clone();
        let n12 = value["lanes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|lane| lane["id"] == "N12")
            .unwrap();
        match mutation {
            0 => n12["current_identity"] = serde_json::Value::Null,
            1 => n12["current_identity"]["lane_id"] = serde_json::json!("N11"),
            2 => n12["authority"] = serde_json::json!("lane_write"),
            3 => n12["ceiling"] = serde_json::json!("source_accepted"),
            _ => n12["state"] = serde_json::json!("integrating"),
        }
        assert!(load_declared_dependency_identities(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}
