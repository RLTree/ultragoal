use sha2::{Digest, Sha256};

pub(super) struct ContentDigestRequest<'a> {
    bytes: &'a [u8],
}

pub(super) struct ContentDigestResponse {
    sha256: String,
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    execute(ContentDigestRequest { bytes }).sha256
}

fn execute(request: ContentDigestRequest<'_>) -> ContentDigestResponse {
    ContentDigestResponse {
        sha256: format!("sha256:{:x}", Sha256::digest(request.bytes)),
    }
}
