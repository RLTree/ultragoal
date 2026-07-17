use super::debt;
use super::overlap;
use super::root;
use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::path::Path;

pub(super) fn validate_record(
    record: &Value,
    registry: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
) {
    let authority_root = registry["lease_state"]
        .get("worktree_root")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let required = [
        "lane_id",
        "owner",
        "scope_ids",
        "claim_ceiling",
        "commands",
        "parent_handoff",
        "plan_ref",
        "plan_digest",
        "base_commit",
        "base_tree",
        "branch",
        "worktree",
        "owned_files",
        "owned_symbols",
        "generated_outputs",
        "fixtures",
        "effects",
        "forbidden_roots",
        "root_only_surfaces",
        "isolated_roots",
        "ready",
        "handoff",
        "teardown",
        "retention",
        "status",
    ];
    for key in required {
        if record.get(key).is_none() {
            out.push(Failure::new("authority-lease", "lease_field_missing", key));
        }
    }
    let roots = record.get("isolated_roots").and_then(Value::as_object);
    if roots.is_none_or(|object| object.len() != 10) {
        out.push(Failure::new(
            "authority-lease",
            "lease_isolation_root_count",
            "expected 10",
        ));
    }
    if !matches!(
        str_field(record, "status").as_str(),
        "unissued" | "issued" | "ready" | "closing" | "closed"
    ) {
        out.push(Failure::new(
            "authority-lease",
            "lease_status_unknown",
            str_field(record, "status"),
        ));
    }
    let path_authority = if str_field(record, "status") != "unissued" {
        let p0 = record.get("exception_id").and_then(Value::as_str) == Some("P0-DEBT-REPAIR");
        root::validate(record, registry, root, authority_root, p0, out)
    } else {
        authority_root.to_owned()
    };
    if str_field(record, "status") != "unissued" {
        let p0 = record.get("exception_id").and_then(Value::as_str) == Some("P0-DEBT-REPAIR");
        let lane_is_p0 = record.get("lane_id").and_then(Value::as_str) == Some("P0");
        debt::check(record, registry, root, out);
        let mut identity = vec!["lease_id", "lane_id", "plan_ref", "branch"];
        if !p0 {
            identity.push("worktree");
        }
        for key in identity {
            if record
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                out.push(Failure::new(
                    "authority-lease",
                    "active_lease_identity_missing",
                    key,
                ));
            }
        }
        if record
            .get("plan_digest")
            .and_then(Value::as_str)
            .is_none_or(|digest| !digest.starts_with("sha256:") || digest.len() != 71)
        {
            out.push(Failure::new(
                "authority-lease",
                "active_plan_digest_missing",
                "plan_digest",
            ));
        }
        if record.get("base_commit").and_then(Value::as_str)
            != registry
                .pointer("/root/base_commit")
                .and_then(Value::as_str)
            || record.get("base_tree").and_then(Value::as_str)
                != registry.pointer("/root/base_tree").and_then(Value::as_str)
        {
            out.push(Failure::new(
                "authority-lease",
                "lease_base_binding_mismatch",
                "base_commit/base_tree",
            ));
        }
        let plan_ref = record
            .get("plan_ref")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let safe = !plan_ref.is_empty()
            && plan_ref.starts_with("docs/exec-plans/active/")
            && !Path::new(plan_ref).is_absolute()
            && !Path::new(plan_ref).components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::RootDir
                )
            });
        let plan_path = root.join(plan_ref);
        let contained = plan_path
            .canonicalize()
            .ok()
            .zip(root.canonicalize().ok())
            .is_some_and(|(candidate, root)| candidate.starts_with(root));
        if !safe || !plan_path.is_file() || !contained {
            out.push(Failure::new(
                "authority-lease",
                "lease_plan_ref_unavailable",
                plan_ref,
            ));
        } else if crate::digest::file(&plan_path).unwrap_or_default()
            != record
                .get("plan_digest")
                .and_then(Value::as_str)
                .unwrap_or_default()
        {
            out.push(Failure::new(
                "authority-lease",
                "lease_plan_digest_mismatch",
                plan_ref,
            ));
        }
        if record
            .get("commands")
            .and_then(Value::as_array)
            .is_none_or(|commands| commands.is_empty())
            || record.get("parent_handoff").is_none_or(Value::is_null)
        {
            out.push(Failure::new(
                "authority-lease",
                "lease_handoff_binding_missing",
                "commands/parent_handoff",
            ));
        }
        if p0 {
            if !lane_is_p0
                || str_field(record, "owner") != "OWN-ULTRA-ROOT"
                || !overlap::array_set(record, "scope_ids").is_empty()
            {
                out.push(Failure::new(
                    "authority-lease",
                    "p0_exception_binding_mismatch",
                    "lane_id/owner/scope_ids",
                ));
            }
        } else if lane_is_p0 {
            out.push(Failure::new(
                "authority-lease",
                "p0_exception_binding_mismatch",
                "lane_id/exception_id",
            ));
        } else if record
            .get("exception_id")
            .is_some_and(|value| !value.is_null())
        {
            out.push(Failure::new(
                "authority-lease",
                "lease_exception_unknown",
                "exception_id",
            ));
        }
        if let Some(lane_id) = record.get("lane_id").and_then(Value::as_str)
            && lane_id != "P0"
        {
            if let Some(lane) = registry["lanes"]
                .as_array()
                .and_then(|lanes| lanes.iter().find(|lane| str_field(lane, "id") == lane_id))
            {
                if str_field(record, "owner") != str_field(lane, "owner")
                    || overlap::array_set(record, "scope_ids")
                        != overlap::array_set(lane, "scope_ids")
                {
                    out.push(Failure::new(
                        "authority-lease",
                        "lease_lane_binding_mismatch",
                        lane_id,
                    ));
                }
            } else {
                out.push(Failure::new(
                    "authority-lease",
                    "lease_lane_unknown",
                    lane_id,
                ));
            }
        }
    }
    if str_field(record, "status") != "unissued" {
        let authority =
            if record.get("exception_id").and_then(Value::as_str) == Some("P0-DEBT-REPAIR") {
                record
                    .get("shared_root")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            } else {
                authority_root
            };
        overlap::surface_union(record, root, authority, out);
        overlap::check_protected(record, root, authority, out);
    }
    for key in ["owned_files", "generated_outputs", "fixtures"] {
        for path in overlap::array_set(record, key) {
            if Path::new(&path).is_absolute()
                || overlap::normalize_path(root, &path_authority, &path).is_none()
            {
                out.push(Failure::new("authority-lease", "lease_path_escape", path));
            }
        }
    }
}
