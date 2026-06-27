use crate::{digest, json_boundary, review::round::ReviewFailure};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct AnchorPaths {
    pub validator_receipt: PathBuf,
    pub review_target_receipt: PathBuf,
    pub archive_receipt: PathBuf,
}

pub(crate) struct AnchorValues {
    pub(crate) validator_path: String,
    pub(crate) review_target_path: String,
    pub(crate) archive_path: String,
    pub(crate) validator_digest: String,
    pub(crate) review_target_digest: String,
    pub(crate) archive_digest: String,
    pub(crate) validator_run_id: String,
    pub(crate) package_digest: String,
    pub(crate) source_errors: Vec<String>,
}

impl AnchorValues {
    pub(crate) fn read(
        current_root: Option<&Path>,
        validator_path: &Path,
        review_path: &Path,
        archive_path: &Path,
    ) -> Result<Self, String> {
        Self::read_labeled(
            current_root,
            current_root,
            validator_path,
            review_path,
            archive_path,
        )
    }

    fn read_labeled(
        current_root: Option<&Path>,
        label_root: Option<&Path>,
        validator_path: &Path,
        review_path: &Path,
        archive_path: &Path,
    ) -> Result<Self, String> {
        let validator = json_boundary::read_json(validator_path)?;
        let review = json_boundary::read_json(review_path)?;
        let archive = json_boundary::read_json(archive_path)?;
        let validator_digest = digest::file(validator_path)?;
        let validator_label = path_label(label_root, validator_path);
        let review_label = path_label(label_root, review_path);
        let archive_label = path_label(label_root, archive_path);
        let package_digest = string(&validator, "/target_revision/value");
        let mut source_errors = Vec::new();
        crate::review::round::anchor::sources::validate_anchor_sources(
            &validator_digest,
            &package_digest,
            current_root,
            &review,
            &archive,
            &mut source_errors,
        );
        Ok(Self {
            validator_path: validator_label,
            review_target_path: review_label,
            archive_path: archive_label,
            validator_digest,
            review_target_digest: string(&review, "/review_target_digest"),
            archive_digest: string(&archive, "/archive/digest"),
            validator_run_id: string(&validator, "/run_id"),
            package_digest,
            source_errors,
        })
    }

    fn zero() -> Self {
        Self {
            validator_path: String::new(),
            review_target_path: String::new(),
            archive_path: String::new(),
            validator_digest: digest::ZERO.to_string(),
            review_target_digest: digest::ZERO.to_string(),
            archive_digest: digest::ZERO.to_string(),
            validator_run_id: String::new(),
            package_digest: digest::ZERO.to_string(),
            source_errors: Vec::new(),
        }
    }
}

pub(crate) fn fixture_anchor_values(root: &Path) -> AnchorValues {
    fixture_anchor_values_for(
        root,
        "fixtures/review-round/anchors/validator-receipt.json",
        "fixtures/review-round/anchors/review-target-receipt.json",
        "fixtures/review-round/anchors/archive-receipt.json",
    )
}

pub(crate) fn fixture_anchor_values_for(
    root: &Path,
    validator: &str,
    review_target: &str,
    archive: &str,
) -> AnchorValues {
    AnchorValues::read_labeled(
        None,
        Some(root),
        &root.join(validator),
        &root.join(review_target),
        &root.join(archive),
    )
    .unwrap_or_else(|_| AnchorValues::zero())
}

pub(crate) fn anchor_errors(value: &Value, anchors: &AnchorValues, out: &mut Vec<ReviewFailure>) {
    for error in &anchors.source_errors {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_anchor_source_mismatch",
            error,
        ));
    }
    if value.get("status").and_then(Value::as_str) != Some("pass") {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_status_not_pass",
            "status",
        ));
    }
    let round_phase = string(value, "/round_phase");
    let anchor_policy = string(value, "/anchor_policy");
    let full_anchor_required =
        crate::review::round::config::full_anchor_required(&round_phase, &anchor_policy);
    let mut required = vec![("/validator_receipt/digest", &anchors.validator_digest)];
    let mut required_paths = vec![("/validator_receipt/path", &anchors.validator_path)];
    if full_anchor_required {
        for ptr in ["/review_target", "/archive"] {
            if value.pointer(ptr).is_none() {
                out.push(ReviewFailure::new(
                    "validator-execution-provenance",
                    "review_round_missing_required_anchor",
                    ptr,
                ));
            }
        }
        required.push(("/review_target/digest", &anchors.review_target_digest));
        required.push(("/archive/digest", &anchors.archive_digest));
        required_paths.push(("/review_target/path", &anchors.review_target_path));
        required_paths.push(("/archive/path", &anchors.archive_path));
    }
    for (ptr, digest) in required {
        if value.pointer(ptr).and_then(Value::as_str) != Some(digest) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                "review_round_stale_anchor_digest",
                ptr,
            ));
        }
    }
    for (ptr, path) in required_paths {
        if value.pointer(ptr).and_then(Value::as_str) != Some(path) {
            out.push(ReviewFailure::new(
                "validator-execution-provenance",
                "review_round_stale_anchor_path",
                ptr,
            ));
        }
    }
    validator_receipt_errors(value, anchors, out);
}

fn validator_receipt_errors(value: &Value, anchors: &AnchorValues, out: &mut Vec<ReviewFailure>) {
    if value
        .pointer("/validator_receipt/run_id")
        .and_then(Value::as_str)
        != Some(&anchors.validator_run_id)
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_stale_validator_run",
            "validator run",
        ));
    }
    if value
        .pointer("/validator_receipt/package_digest")
        .and_then(Value::as_str)
        != Some(&anchors.package_digest)
    {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_stale_package_digest",
            "package digest",
        ));
    }
}

fn path_label(root: Option<&Path>, path: &Path) -> String {
    if let Some(root) = root {
        if let Ok(stripped) = path.strip_prefix(root) {
            return stripped.to_string_lossy().to_string();
        }
    }
    path.to_string_lossy().to_string()
}

fn string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn path_label_uses_relative_label_when_possible_and_full_path_otherwise() {
        assert_eq!(
            super::path_label(Some(Path::new("/repo")), Path::new("/repo/a/b.json")),
            "a/b.json"
        );
        assert_eq!(
            super::path_label(Some(Path::new("/repo")), Path::new("/other/b.json")),
            "/other/b.json"
        );
        assert_eq!(
            super::path_label(None, Path::new("local.json")),
            "local.json"
        );
    }
}
