use crate::audit::contract::Failure;
use crate::claim_semantics::array_strings;
use serde_json::Value;

pub(super) fn check(registry: &Value, out: &mut Vec<Failure>) {
    let expected = [
        (vec!["N03"], vec!["N04", "N05", "N06", "N07"]),
        (vec!["N02", "N05"], vec!["N09"]),
        (vec!["N04", "N05", "N06", "N07"], vec!["N08"]),
        (vec!["N03", "N06", "N09"], vec!["N10"]),
        (vec!["N06", "N07", "N08"], vec!["N11"]),
        (vec!["N10", "N11"], vec!["N12"]),
    ];
    let actual = registry
        .pointer("/execution_policy/frontier")
        .and_then(Value::as_array);
    let matches = actual.is_some_and(|rows| {
        rows.len() == expected.len()
            && rows.iter().zip(expected).all(|(row, (after, ready))| {
                array_strings(row, "after") == after && array_strings(row, "ready") == ready
            })
    });
    if !matches {
        out.push(Failure::new(
            "authority-graph",
            "frontier_mismatch",
            "execution_policy.frontier",
        ));
    }
    let route = registry
        .pointer("/execution_policy/post_n12_route")
        .and_then(Value::as_array);
    let expected_route = [
        "N12-A",
        "N14",
        "N14-proof",
        "N12-B-invalidate-and-reproof",
        "parallel:N13,N15",
        "N16",
        "N17",
    ]
    .map(str::to_owned);
    let route_tokens = route.and_then(|rows| {
        rows.iter()
            .map(|row| {
                row.as_str().map(str::to_owned).or_else(|| {
                    row.as_array().map(|values| {
                        format!(
                            "parallel:{}",
                            values
                                .iter()
                                .filter_map(Value::as_str)
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    })
                })
            })
            .collect::<Option<Vec<_>>>()
    });
    let route_ok = route_tokens.as_deref() == Some(expected_route.as_slice());
    if !route_ok {
        out.push(Failure::new(
            "authority-graph",
            "postroute_mismatch",
            "execution_policy.post_n12_route",
        ));
    }
    let invalidation = &registry["execution_policy"]["n14_invalidation"];
    let closure = array_strings(invalidation, "closure");
    let expected_closure = [
        "N02", "N03", "N04", "N08", "N09", "N12", "N13", "N14", "N15", "N16", "N17",
    ];
    if closure != expected_closure {
        out.push(Failure::new(
            "authority-graph",
            "n14_closure_mismatch",
            "N14",
        ));
    }
    let n10 = invalidation
        .pointer("/conditional_reproof/N10/consumed_set_intersection")
        .and_then(Value::as_str);
    let n11 = invalidation
        .pointer("/conditional_reproof/N11/consumed_set_intersection")
        .and_then(Value::as_str);
    if n10 != Some("N10.consumed_set ∩ N09.invalidated_set")
        || n11 != Some("N11.consumed_set ∩ N08.invalidated_set")
    {
        out.push(Failure::new(
            "authority-graph",
            "n14_consumed_set_rule_missing",
            "N10/N11",
        ));
    }
}

pub(super) fn check_lane_contracts(registry: &Value, out: &mut Vec<Failure>) {
    let pre_adoption = &registry["pre_adoption_source"];
    if pre_adoption.get("source_ceiling").and_then(Value::as_str)
        != Some("source-local-speculative")
        || pre_adoption
            .get("eligible_scheduler_nodes")
            .and_then(Value::as_array)
            .is_none_or(|rows| !rows.is_empty())
        || pre_adoption
            .get("eligible_claim_ids")
            .and_then(Value::as_array)
            .is_none_or(|rows| !rows.is_empty())
    {
        out.push(Failure::new(
            "authority-graph",
            "pre_adoption_scheduler_or_claim_eligibility",
            "pre_adoption_source",
        ));
    }
    for lane in registry["lanes"].as_array().into_iter().flatten() {
        let id = lane.get("id").and_then(Value::as_str).unwrap_or_default();
        let dependencies = array_strings(lane, "dependencies");
        let consumed = lane
            .pointer("/consumption_contract/dependency_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let invalidated = lane
            .pointer("/invalidation_contract/invalidated_by")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let expected_ceiling = if id == "N00" {
            "pre_adoption_context_only"
        } else {
            "pre_adoption_source_only"
        };
        if consumed != dependencies
            || invalidated != dependencies
            || lane.get("ceiling").and_then(Value::as_str) != Some(expected_ceiling)
        {
            out.push(Failure::new(
                "authority-graph",
                "lane_consumption_or_ceiling_mismatch",
                id,
            ));
        }
    }
    if registry
        .pointer("/execution_policy/n02_reobservation/before")
        .and_then(Value::as_str)
        != Some("N03")
    {
        out.push(Failure::new(
            "authority-graph",
            "n02_reobservation_missing",
            "execution_policy.n02_reobservation",
        ));
    }
}
