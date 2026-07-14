use sha2::{Digest, Sha256};

pub(super) struct ContentDigestRequest<'a> {
    bytes: &'a [u8],
}

pub(super) struct ContentDigestResponse {
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContentDigestError {
    Unavailable,
}

pub(super) fn sha256(bytes: &[u8]) -> Result<String, ContentDigestError> {
    execute(ContentDigestRequest { bytes }).map(|response| response.sha256)
}

fn execute(request: ContentDigestRequest<'_>) -> Result<ContentDigestResponse, ContentDigestError> {
    Ok(ContentDigestResponse {
        sha256: format!("sha256:{:x}", Sha256::digest(request.bytes)),
    })
}
