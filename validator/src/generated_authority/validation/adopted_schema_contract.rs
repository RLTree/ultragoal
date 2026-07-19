use super::super::contracts::{RepositoryPath, Sha256Digest};
use super::super::path;
use super::value;

pub(in crate::generated_authority) struct AdoptedSchemaFields {
    pub(in crate::generated_authority) output: RepositoryPath,
    pub(in crate::generated_authority) sha256: Sha256Digest,
    pub(in crate::generated_authority) schema: RepositoryPath,
    pub(in crate::generated_authority) source_contract: RepositoryPath,
    pub(in crate::generated_authority) source_contract_sha256: Sha256Digest,
    pub(in crate::generated_authority) amendment_log: RepositoryPath,
    pub(in crate::generated_authority) amendment_id: String,
    pub(in crate::generated_authority) amendment_hash: Sha256Digest,
}

pub(in crate::generated_authority) struct AdoptedSchemaInput {
    pub(in crate::generated_authority) output: String,
    pub(in crate::generated_authority) sha256: String,
    pub(in crate::generated_authority) schema: String,
    pub(in crate::generated_authority) source_contract: String,
    pub(in crate::generated_authority) source_contract_sha256: String,
    pub(in crate::generated_authority) amendment_log: String,
    pub(in crate::generated_authority) amendment_id: String,
    pub(in crate::generated_authority) amendment_hash: String,
    pub(in crate::generated_authority) claim_ceiling: String,
}

pub(in crate::generated_authority) fn validate(
    input: AdoptedSchemaInput,
) -> Result<AdoptedSchemaFields, &'static str> {
    if input.claim_ceiling != "contract_authority_only"
        || !input
            .amendment_id
            .strip_prefix("AMEND-")
            .is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            })
    {
        return Err("generated_authority_adopted_schema_contract_invalid");
    }
    let output = path::parse(input.output)?;
    let schema = path::parse(input.schema)?;
    let source_contract = path::parse(input.source_contract)?;
    let amendment_log = path::parse(input.amendment_log)?;
    if output == schema || output == source_contract || output == amendment_log {
        return Err("generated_authority_adopted_schema_contract_invalid");
    }
    Ok(AdoptedSchemaFields {
        output,
        sha256: value::digest(&input.sha256)?,
        schema,
        source_contract,
        source_contract_sha256: value::digest(&input.source_contract_sha256)?,
        amendment_log,
        amendment_id: input.amendment_id,
        amendment_hash: value::digest(&input.amendment_hash)?,
    })
}
