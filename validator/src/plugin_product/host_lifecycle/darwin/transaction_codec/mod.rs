use super::surface_codec::{PackageBinding, digest};
use super::transaction::{TransactionJournal, TransactionLineage, TransactionLineageAnchor};
use super::{DarwinHostError, DarwinHostErrorId, DarwinHostOperation, DarwinHostSurface};
use serde::Serialize;

#[derive(Clone, Copy)]
pub(super) enum DarwinTransactionCodecRequest<'a> {
    EncodeJournal(&'a TransactionJournal),
    DecodeJournal(&'a [u8]),
    EncodeLineage(&'a TransactionLineage),
    DecodeLineage(&'a [u8]),
    EncodeLineageAnchor(&'a TransactionLineageAnchor),
    DecodeLineageAnchor(&'a [u8]),
    PlanDigest {
        root_id: &'a str,
        operation: DarwinHostOperation,
        target: &'a PackageBinding,
        prior: &'a Option<PackageBinding>,
        marketplace: &'a str,
        steps: &'a [(DarwinHostSurface, Option<&'a str>, Option<&'a str>)],
    },
}

pub(super) enum DarwinTransactionCodecResponse {
    Bytes(Vec<u8>),
    Journal(TransactionJournal),
    Lineage(TransactionLineage),
    LineageAnchor(TransactionLineageAnchor),
    Digest(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DarwinTransactionCodecError {
    EncodeJournal,
    DecodeJournal,
    EncodeLineage,
    DecodeLineage,
    EncodeLineageAnchor,
    DecodeLineageAnchor,
    EncodePlan,
}

#[derive(Serialize)]
struct PlanDigest<'a> {
    schema: &'static str,
    root_id: &'a str,
    operation: DarwinHostOperation,
    target: &'a PackageBinding,
    prior: &'a Option<PackageBinding>,
    marketplace: &'a str,
    steps: &'a [(DarwinHostSurface, Option<&'a str>, Option<&'a str>)],
}

pub(super) fn execute(
    request: DarwinTransactionCodecRequest<'_>,
) -> Result<DarwinTransactionCodecResponse, DarwinTransactionCodecError> {
    match request {
        DarwinTransactionCodecRequest::EncodeJournal(record) => serde_json::to_vec(record)
            .map(DarwinTransactionCodecResponse::Bytes)
            .map_err(|_| DarwinTransactionCodecError::EncodeJournal),
        DarwinTransactionCodecRequest::DecodeJournal(bytes) => serde_json::from_slice(bytes)
            .map(DarwinTransactionCodecResponse::Journal)
            .map_err(|_| DarwinTransactionCodecError::DecodeJournal),
        DarwinTransactionCodecRequest::EncodeLineage(record) => serde_json::to_vec(record)
            .map(DarwinTransactionCodecResponse::Bytes)
            .map_err(|_| DarwinTransactionCodecError::EncodeLineage),
        DarwinTransactionCodecRequest::DecodeLineage(bytes) => serde_json::from_slice(bytes)
            .map(DarwinTransactionCodecResponse::Lineage)
            .map_err(|_| DarwinTransactionCodecError::DecodeLineage),
        DarwinTransactionCodecRequest::EncodeLineageAnchor(record) => serde_json::to_vec(record)
            .map(DarwinTransactionCodecResponse::Bytes)
            .map_err(|_| DarwinTransactionCodecError::EncodeLineageAnchor),
        DarwinTransactionCodecRequest::DecodeLineageAnchor(bytes) => serde_json::from_slice(bytes)
            .map(DarwinTransactionCodecResponse::LineageAnchor)
            .map_err(|_| DarwinTransactionCodecError::DecodeLineageAnchor),
        DarwinTransactionCodecRequest::PlanDigest {
            root_id,
            operation,
            target,
            prior,
            marketplace,
            steps,
        } => serde_json::to_vec(&PlanDigest {
            schema: "harness-ultragoal.darwin-host-transaction-plan.v1",
            root_id,
            operation,
            target,
            prior,
            marketplace,
            steps,
        })
        .map(|bytes| DarwinTransactionCodecResponse::Digest(digest(&bytes)))
        .map_err(|_| DarwinTransactionCodecError::EncodePlan),
    }
}

pub(super) fn journal_bytes(record: &TransactionJournal) -> Result<Vec<u8>, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::EncodeJournal(record))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::JournalCorrupt))?
    {
        DarwinTransactionCodecResponse::Bytes(bytes) => Ok(bytes),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::JournalCorrupt)),
    }
}

pub(super) fn decode_journal(bytes: &[u8]) -> Result<TransactionJournal, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::DecodeJournal(bytes))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::JournalCorrupt))?
    {
        DarwinTransactionCodecResponse::Journal(record) => Ok(record),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::JournalCorrupt)),
    }
}

pub(super) fn lineage_bytes(record: &TransactionLineage) -> Result<Vec<u8>, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::EncodeLineage(record))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageCorrupt))?
    {
        DarwinTransactionCodecResponse::Bytes(bytes) => Ok(bytes),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt)),
    }
}

pub(super) fn decode_lineage(bytes: &[u8]) -> Result<TransactionLineage, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::DecodeLineage(bytes))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageCorrupt))?
    {
        DarwinTransactionCodecResponse::Lineage(record) => Ok(record),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt)),
    }
}

pub(super) fn lineage_anchor_bytes(
    record: &TransactionLineageAnchor,
) -> Result<Vec<u8>, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::EncodeLineageAnchor(record))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageCorrupt))?
    {
        DarwinTransactionCodecResponse::Bytes(bytes) => Ok(bytes),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt)),
    }
}

pub(super) fn decode_lineage_anchor(
    bytes: &[u8],
) -> Result<TransactionLineageAnchor, DarwinHostError> {
    match execute(DarwinTransactionCodecRequest::DecodeLineageAnchor(bytes))
        .map_err(|_| DarwinHostError::new(DarwinHostErrorId::LineageCorrupt))?
    {
        DarwinTransactionCodecResponse::LineageAnchor(record) => Ok(record),
        _ => Err(DarwinHostError::new(DarwinHostErrorId::LineageCorrupt)),
    }
}
