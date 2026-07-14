use super::contracts::{ExpectedFailure, RedCatalogError};
use serde::Deserialize;
use serde::de::IgnoredAny;

pub(crate) struct RedPacketParseRequest<'a> {
    pub(crate) bytes: &'a [u8],
}

pub(crate) struct RedPacketParseResponse {
    pub(crate) id: String,
    pub(crate) expected_failure: ExpectedFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RedPacketParseError {
    InvalidContract,
    InvalidValues,
    TrailingData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    id: String,
    expected_failure: Failure,
    base_fixture_path: String,
    json_patch: Vec<Patch>,
    materialization: Materialization,
    preconditions: Vec<Precondition>,
    postconditions: Vec<Postcondition>,
    #[serde(default)]
    filesystem_fixtures: Vec<FilesystemFixture>,
    notes: Option<String>,
    review_round_anchor_overrides: Option<ReviewRoundOverrides>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    check_id: String,
    error: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    op: PatchOperation,
    path: String,
    value: Option<IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PatchOperation {
    Add,
    Remove,
    Replace,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Materialization {
    expected_validation_layer: ValidationLayer,
    first_failure_must_match_expected: bool,
    post_patch_schema_valid: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum ValidationLayer {
    Schema,
    Semantic,
    Package,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Precondition {
    exists: bool,
    path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Postcondition {
    expectation: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum FilesystemFixture {
    File { path: String, contents: String },
    Symlink { path: String, target: String },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewRoundOverrides {
    archive_receipt: Option<String>,
    review_target_receipt: Option<String>,
    validator_receipt: Option<String>,
}

pub(crate) fn parse(
    request: RedPacketParseRequest<'_>,
) -> Result<RedPacketParseResponse, RedPacketParseError> {
    let mut deserializer = serde_json::Deserializer::from_slice(request.bytes);
    let packet =
        Packet::deserialize(&mut deserializer).map_err(|_| RedPacketParseError::InvalidContract)?;
    deserializer
        .end()
        .map_err(|_| RedPacketParseError::TrailingData)?;
    validate(packet)
}

fn validate(packet: Packet) -> Result<RedPacketParseResponse, RedPacketParseError> {
    if packet.schema != "harness-ultragoal.red-packet.v1"
        || !semantic_id(&packet.id)
        || invalid_text(&packet.expected_failure.check_id)
        || invalid_text(&packet.expected_failure.error)
        || invalid_text(&packet.base_fixture_path)
        || packet.json_patch.is_empty()
        || packet.preconditions.is_empty()
        || packet.postconditions.is_empty()
        || packet.json_patch.iter().any(|row| invalid_text(&row.path))
        || packet
            .preconditions
            .iter()
            .any(|row| invalid_text(&row.path))
        || packet
            .postconditions
            .iter()
            .any(|row| invalid_text(&row.path) || invalid_text(&row.expectation))
        || packet.filesystem_fixtures.iter().any(invalid_filesystem)
        || packet
            .review_round_anchor_overrides
            .as_ref()
            .is_some_and(invalid_overrides)
    {
        return Err(RedPacketParseError::InvalidValues);
    }
    consume_contract_fields(&packet);
    Ok(RedPacketParseResponse {
        id: packet.id,
        expected_failure: ExpectedFailure {
            check_id: packet.expected_failure.check_id,
            error: packet.expected_failure.error,
        },
    })
}

fn consume_contract_fields(packet: &Packet) {
    for patch in &packet.json_patch {
        let _ = (&patch.op, &patch.value);
    }
    let _ = (
        &packet.materialization.expected_validation_layer,
        packet.materialization.first_failure_must_match_expected,
        packet.materialization.post_patch_schema_valid,
        packet.preconditions.iter().map(|row| row.exists).count(),
        packet.notes.as_deref(),
    );
}

fn invalid_filesystem(fixture: &FilesystemFixture) -> bool {
    match fixture {
        FilesystemFixture::File { path, contents } => {
            invalid_text(path) || contents.is_empty() || contents.contains('\0')
        }
        FilesystemFixture::Symlink { path, target } => invalid_text(path) || invalid_text(target),
    }
}

fn invalid_overrides(overrides: &ReviewRoundOverrides) -> bool {
    [
        overrides.archive_receipt.as_deref(),
        overrides.review_target_receipt.as_deref(),
        overrides.validator_receipt.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(invalid_text)
}

fn invalid_text(value: &str) -> bool {
    value.trim().is_empty() || value.contains(['\n', '\r', '\0'])
}

fn semantic_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 192
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(super) fn packet(
    bytes: &[u8],
    relative: &str,
) -> Result<RedPacketParseResponse, RedCatalogError> {
    parse(RedPacketParseRequest { bytes }).map_err(|error| {
        let code = match error {
            RedPacketParseError::InvalidContract => "red_catalog_packet_contract_invalid",
            RedPacketParseError::InvalidValues => "red_catalog_packet_values_invalid",
            RedPacketParseError::TrailingData => "red_catalog_packet_trailing_data",
        };
        RedCatalogError::new(code, Some(relative.to_string()))
    })
}
