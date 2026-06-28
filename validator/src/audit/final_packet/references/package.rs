use serde_json::Value;
use std::path::Path;

const FIT_REPO: &str = "validation_artifacts/harness/fit-repo-receipt.json";
const PRODUCT_FITNESS: &str = "validation_artifacts/harness/product-fitness-receipt.json";
const PRODUCT_JOURNEY: &str = "validation_artifacts/harness/plugin-product-journey-receipt.json";
const REQUIRED_PACKAGE_RECEIPTS: &[&str] = &[FIT_REPO, PRODUCT_FITNESS, PRODUCT_JOURNEY];

pub(super) fn failures(root: &Path, receipt: &Value, expected: &str, out: &mut Vec<String>) {
    let Some(rows) = receipt.get("package_receipts").and_then(Value::as_array) else {
        out.push("final_packet_proof_package_receipts_missing".to_string());
        return;
    };
    required_receipt_failures(rows, out);
    for (idx, _) in rows.iter().enumerate() {
        let ptr = format!("/package_receipts/{idx}");
        if let Some(value) = super::load_pass_ref(root, receipt, &ptr, "package", out) {
            value_failures(root, receipt, &ptr, &value, expected, out);
        }
    }
}

fn required_receipt_failures(rows: &[Value], out: &mut Vec<String>) {
    for required in REQUIRED_PACKAGE_RECEIPTS {
        if !rows
            .iter()
            .any(|row| row.get("path").and_then(Value::as_str) == Some(*required))
        {
            out.push(format!(
                "final_packet_proof_package_receipt_missing:{required}"
            ));
        }
    }
}

fn value_failures(
    root: &Path,
    receipt: &Value,
    ptr: &str,
    value: &Value,
    expected: &str,
    out: &mut Vec<String>,
) {
    if value
        .pointer("/target_revision/value")
        .and_then(Value::as_str)
        != Some(expected)
    {
        out.push("final_packet_proof_package_target_digest_mismatch".to_string());
    }
    match receipt
        .pointer(&format!("{ptr}/path"))
        .and_then(Value::as_str)
        .unwrap_or("")
    {
        FIT_REPO => fit_repo_failures(root, value, out),
        PRODUCT_FITNESS => product_fitness_failures(root, value, out),
        PRODUCT_JOURNEY => product_journey_failures(root, value, out),
        path => out.push(format!("final_packet_proof_package_receipt_unknown:{path}")),
    }
}

fn fit_repo_failures(root: &Path, value: &Value, out: &mut Vec<String>) {
    for failure in crate::audit::fit_repo_receipt::failures(root, value) {
        out.push(format!("final_packet_proof_fit_repo_ref:{failure}"));
    }
}

fn product_fitness_failures(root: &Path, value: &Value, out: &mut Vec<String>) {
    for failure in
        crate::audit::product::fitness::canonical_package_receipt_value_failures(root, value)
    {
        out.push(format!("final_packet_proof_product_fitness_ref:{failure}"));
    }
}

fn product_journey_failures(root: &Path, value: &Value, out: &mut Vec<String>) {
    for failure in crate::audit::plugin::product::cohesion::journey_value_failures(root, value) {
        out.push(format!("final_packet_proof_product_journey_ref:{failure}"));
    }
}
