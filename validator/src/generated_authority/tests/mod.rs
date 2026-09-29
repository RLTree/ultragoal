mod acceptance;
mod registry_samples;
mod rejection;
#[path = "shard/acceptance.rs"]
mod shard_acceptance;
#[path = "shard/rejection.rs"]
mod shard_rejection;
#[path = "shard/samples.rs"]
mod shard_samples;

use super::{GeneratedAuthorityParseRequest, parse};

fn rejection_code(bytes: &[u8]) -> &'static str {
    parse(GeneratedAuthorityParseRequest { bytes })
        .expect_err("registry must be rejected")
        .stable_text()
}
