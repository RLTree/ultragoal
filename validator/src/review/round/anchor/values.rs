use crate::{digest, review::round::ReviewFailure};
use serde_json::Value;
use std::path::{Path, PathBuf};

use super::{
    artifact,
    policy::{self, AnchorSource},
    refs::VerifiedRef,
    semantics,
};

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
    pub(crate) materiality_anchors: Vec<VerifiedRef>,
}

impl AnchorValues {
    pub(crate) fn read(
        current_root: Option<&Path>,
        validator_path: &Path,
        review_path: &Path,
        archive_path: &Path,
    ) -> Result<Self, String> {
        let root = current_root
            .ok_or_else(|| "review_round_trusted_anchor_source_unavailable".to_string())?;
        let labels = labels(root, validator_path, review_path, archive_path)?;
        let source = policy::exact_source(&labels)
            .ok_or_else(|| "review_round_trusted_anchor_source_unavailable".to_string())?;
        if source != AnchorSource::Live {
            return Err("review_round_trusted_anchor_source_unavailable".to_string());
        }
        Self::read_bound(root, source, labels)
    }

    fn read_bound(root: &Path, source: AnchorSource, labels: [String; 3]) -> Result<Self, String> {
        let validator = artifact::read(root, &labels[0])?;
        let review = artifact::read(root, &labels[1])?;
        let archive = artifact::read(root, &labels[2])?;
        let source_errors = semantics::errors(semantics::Inputs {
            source,
            root,
            labels: &labels,
            validator: &validator,
            review: &review,
            archive: &archive,
        });
        let validator_digest = validator.digest.clone();
        let review_target_digest = string(&review.value, "/review_target_digest");
        let archive_digest = string(&archive.value, "/archive/digest");
        let validator_run_id = string(&validator.value, "/run_id");
        let package_digest = string(&validator.value, "/target_revision/value");
        let materiality_anchors = vec![
            VerifiedRef {
                path: labels[0].clone(),
                digest: validator.digest,
                value: Some(validator.value),
            },
            VerifiedRef {
                path: labels[1].clone(),
                digest: review.digest,
                value: Some(review.value),
            },
            VerifiedRef {
                path: labels[2].clone(),
                digest: archive.digest,
                value: Some(archive.value),
            },
        ];
        Ok(Self {
            validator_path: labels[0].clone(),
            review_target_path: labels[1].clone(),
            archive_path: labels[2].clone(),
            validator_digest,
            review_target_digest,
            archive_digest,
            validator_run_id,
            package_digest,
            source_errors,
            materiality_anchors,
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
            materiality_anchors: Vec::new(),
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
    let labels = [
        validator.to_string(),
        review_target.to_string(),
        archive.to_string(),
    ];
    if labels
        .iter()
        .any(|label| !policy::static_fixture_variant(label))
    {
        return AnchorValues::zero();
    }
    AnchorValues::read_bound(root, AnchorSource::StaticFixture, labels)
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
    if value.get("status").and_then(Value::as_str) != Some("evidence_complete") {
        out.push(ReviewFailure::new(
            "validator-execution-provenance",
            "review_round_evidence_incomplete",
            "status",
        ));
    }
    let review_stage = string(value, "/review_stage");
    let anchor_policy = string(value, "/anchor_policy");
    let full_anchor_required =
        crate::review::round::config::full_anchor_required(&review_stage, &anchor_policy);
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

fn labels(
    root: &Path,
    validator: &Path,
    review: &Path,
    archive: &Path,
) -> Result<[String; 3], String> {
    Ok([
        policy::relative_label(root, validator)?,
        policy::relative_label(root, review)?,
        policy::relative_label(root, archive)?,
    ])
}

fn string(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
