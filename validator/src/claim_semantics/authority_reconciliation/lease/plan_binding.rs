use crate::audit::contract::Failure;
use serde_json::Value;
use std::path::Path;

pub(super) fn validate(record: &Value, root: &Path, out: &mut Vec<Failure>) {
    let plan_ref = record
        .get("plan_ref")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let plan_path = root.join(plan_ref);
    let safe = !plan_ref.is_empty()
        && plan_ref.starts_with("docs/exec-plans/active/")
        && !Path::new(plan_ref).is_absolute()
        && !Path::new(plan_ref).components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        });
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
}
