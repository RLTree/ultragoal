mod acceptance;
mod registry_samples;
mod rejection;
mod shard_acceptance;
mod shard_rejection;
mod shard_samples;

use super::{GeneratedAuthorityParseRequest, parse};

fn rejection_code(bytes: &[u8]) -> &'static str {
    parse(GeneratedAuthorityParseRequest { bytes })
        .expect_err("registry must be rejected")
        .stable_text()
}
