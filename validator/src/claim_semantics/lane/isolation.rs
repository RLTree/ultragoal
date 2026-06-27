use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, lane::paths, str_field};
use serde_json::Value;

pub(crate) fn mutable_resources(lanes: &[Value], out: &mut Vec<Failure>) {
    let mut paths: Vec<ResourcePath> = Vec::new();
    let mut ports: Vec<ResourcePort> = Vec::new();
    let mut branches: Vec<ResourceBranch> = Vec::new();
    for lane in lanes
        .iter()
        .filter(|lane| crate::claim_semantics::lane::status::blocks_isolation(lane))
    {
        let lane_id = str_field(lane, "id");
        collect_paths(lane, &lane_id, &mut paths, out);
        collect_ports(lane, &lane_id, &mut ports, out);
        collect_branch(lane, &lane_id, &mut branches, out);
    }
}

fn collect_paths(
    lane: &Value,
    lane_id: &str,
    seen: &mut Vec<ResourcePath>,
    out: &mut Vec<Failure>,
) {
    for (kind, path) in resource_paths(lane) {
        let path = match paths::normalize_resource(&path) {
            Ok(path) => path,
            Err(err) => {
                out.push(Failure::new(
                    "lane-scope-overlap",
                    "invalid_lane_resource_path",
                    format!("{kind}:{path}: {err}"),
                ));
                continue;
            }
        };
        if let Some(prior) = seen.iter().find(|prior| {
            prior.lane_id != lane_id
                && (paths::contains(&prior.path, &path).unwrap_or(false)
                    || paths::contains(&path, &prior.path).unwrap_or(false))
        }) {
            out.push(Failure::new(
                "lane-scope-overlap",
                "active_lane_mutable_resource_overlap",
                format!("{}:{} overlaps {}:{}", kind, path, prior.kind, prior.path),
            ));
        }
        seen.push(ResourcePath {
            lane_id: lane_id.to_string(),
            kind,
            path,
        });
    }
}

fn collect_ports(
    lane: &Value,
    lane_id: &str,
    seen: &mut Vec<ResourcePort>,
    out: &mut Vec<Failure>,
) {
    for port in lane
        .get("port_allocations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("port").and_then(Value::as_u64))
    {
        if let Some(prior) = seen
            .iter()
            .find(|prior| prior.lane_id != lane_id && prior.port == port)
        {
            out.push(Failure::new(
                "lane-scope-overlap",
                "active_lane_mutable_resource_overlap",
                format!("port:{port} overlaps lane {}", prior.lane_id),
            ));
        }
        seen.push(ResourcePort {
            lane_id: lane_id.to_string(),
            port,
        });
    }
}

fn resource_paths(lane: &Value) -> Vec<(String, String)> {
    let mut paths = Vec::new();
    let workspace = str_field(lane, "workspace");
    if !workspace.is_empty() {
        paths.push(("workspace".to_string(), workspace));
    }
    for key in [
        "state_roots",
        "scratch_roots",
        "tool_cache_roots",
        "browser_profile_roots",
    ] {
        for path in array_strings(lane, key) {
            paths.push((key.to_string(), path));
        }
    }
    let artifact_root = str_field(lane, "artifact_root");
    if !artifact_root.is_empty() {
        paths.push(("artifact_root".to_string(), artifact_root));
    }
    paths
}

fn collect_branch(
    lane: &Value,
    lane_id: &str,
    seen: &mut Vec<ResourceBranch>,
    out: &mut Vec<Failure>,
) {
    let branch = str_field(lane, "branch");
    if branch.is_empty() {
        return;
    }
    if let Some(prior) = seen
        .iter()
        .find(|prior| prior.lane_id != lane_id && prior.branch == branch)
    {
        out.push(Failure::new(
            "lane-scope-overlap",
            "active_lane_mutable_resource_overlap",
            format!("branch:{branch} overlaps lane {}", prior.lane_id),
        ));
    }
    seen.push(ResourceBranch {
        lane_id: lane_id.to_string(),
        branch,
    });
}

struct ResourcePath {
    lane_id: String,
    kind: String,
    path: String,
}

struct ResourcePort {
    lane_id: String,
    port: u64,
}

struct ResourceBranch {
    lane_id: String,
    branch: String,
}
