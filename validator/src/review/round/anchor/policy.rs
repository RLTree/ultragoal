use serde_json::Value;
use std::path::{Component, Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnchorSource {
    StaticFixture,
    Live,
}

pub(crate) const STATIC_ANCHORS: [&str; 3] = [
    "fixtures/review-round/anchors/validator-receipt.json",
    "fixtures/review-round/anchors/review-target-receipt.json",
    "fixtures/review-round/anchors/archive-receipt.json",
];
pub(crate) const LIVE_ANCHORS: [&str; 3] = [
    "validation_artifacts/ultragoal-audit/validator-receipt.json",
    "validation_artifacts/review/review-target-receipt.json",
    "validation_artifacts/review/candidate-archive-receipt.json",
];
pub(crate) const STATIC_REGISTRY: &str =
    "fixtures/review-round/anchors/live-registry-exposure.json";
pub(crate) const LIVE_REGISTRY: &str = "validation_artifacts/review/live-registry-exposure.json";
const STATIC_REGISTRY_SET: [&str; 1] = [STATIC_REGISTRY];
const LIVE_REGISTRY_SET: [&str; 1] = [LIVE_REGISTRY];

pub(crate) fn exact_source(paths: &[String; 3]) -> Option<AnchorSource> {
    if exact(paths, &STATIC_ANCHORS) {
        Some(AnchorSource::StaticFixture)
    } else if exact(paths, &LIVE_ANCHORS) {
        Some(AnchorSource::Live)
    } else {
        None
    }
}

pub(crate) fn receipt_source(receipt: &Value) -> Option<AnchorSource> {
    exact_source(&[
        text_at(receipt, "/validator_receipt/path"),
        text_at(receipt, "/review_target/path"),
        text_at(receipt, "/archive/path"),
    ])
}

pub(crate) fn expected_anchors(source: AnchorSource) -> &'static [&'static str; 3] {
    match source {
        AnchorSource::StaticFixture => &STATIC_ANCHORS,
        AnchorSource::Live => &LIVE_ANCHORS,
    }
}

pub(crate) fn expected_registry(source: AnchorSource) -> &'static [&'static str; 1] {
    match source {
        AnchorSource::StaticFixture => &STATIC_REGISTRY_SET,
        AnchorSource::Live => &LIVE_REGISTRY_SET,
    }
}

pub(crate) fn relative_label(root: &Path, path: &Path) -> Result<String, String> {
    let relative = if path.is_absolute() {
        path.strip_prefix(root)
            .map_err(|_| "review_round_anchor_path_invalid".to_string())?
    } else {
        path
    };
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("review_round_anchor_path_invalid".to_string());
    }
    relative
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "review_round_anchor_path_invalid".to_string())
}

pub(crate) fn static_fixture_variant(path: &str) -> bool {
    path.starts_with("fixtures/review-round/anchors/")
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn exact(paths: &[String; 3], expected: &[&str; 3]) -> bool {
    paths.iter().zip(expected).all(|(got, want)| got == want)
}

fn text_at(value: &Value, pointer: &str) -> String {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}
