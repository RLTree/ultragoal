use crate::audit::source_governance::GovernedSource;
use serde::Deserialize;
use std::collections::BTreeMap;

pub(crate) enum ToolAuthorityParseRequest<'a> {
    Cargo(&'a [u8]),
    Pnpm(&'a [u8]),
}

pub(crate) struct ToolAuthorityParseResponse {
    pub(crate) version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ToolAuthorityParseError {
    Encoding,
    InvalidCargoAuthority,
    InvalidPnpmAuthority,
}

#[derive(Deserialize)]
struct PackageAuthority {
    #[serde(rename = "packageManager")]
    package_manager: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RustToolchainAuthority {
    toolchain: RustToolchain,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RustToolchain {
    channel: String,
    components: Vec<String>,
    profile: String,
}

pub(crate) fn parse(
    request: ToolAuthorityParseRequest<'_>,
) -> Result<ToolAuthorityParseResponse, ToolAuthorityParseError> {
    match request {
        ToolAuthorityParseRequest::Cargo(bytes) => {
            let text = std::str::from_utf8(bytes).map_err(|_| ToolAuthorityParseError::Encoding)?;
            let authority = toml::from_str::<RustToolchainAuthority>(text)
                .map_err(|_| ToolAuthorityParseError::InvalidCargoAuthority)?;
            if authority.toolchain.components.is_empty()
                || authority.toolchain.profile.trim().is_empty()
            {
                return Err(ToolAuthorityParseError::InvalidCargoAuthority);
            }
            Ok(ToolAuthorityParseResponse {
                version: authority.toolchain.channel,
            })
        }
        ToolAuthorityParseRequest::Pnpm(bytes) => {
            let authority = serde_json::from_slice::<PackageAuthority>(bytes)
                .map_err(|_| ToolAuthorityParseError::InvalidPnpmAuthority)?;
            let version = authority
                .package_manager
                .strip_prefix("pnpm@")
                .filter(|value| !value.is_empty())
                .ok_or(ToolAuthorityParseError::InvalidPnpmAuthority)?;
            Ok(ToolAuthorityParseResponse {
                version: version.to_string(),
            })
        }
    }
}

pub(super) fn pinned(
    sources: &BTreeMap<&str, &GovernedSource>,
    tool: &str,
    expected: &str,
) -> bool {
    let request = match tool {
        "cargo" => sources
            .get("rust-toolchain.toml")
            .map(|source| ToolAuthorityParseRequest::Cargo(&source.bytes)),
        "pnpm" => sources
            .get("package.json")
            .map(|source| ToolAuthorityParseRequest::Pnpm(&source.bytes)),
        _ => None,
    };
    request
        .and_then(|request| parse(request).ok())
        .is_some_and(|response| response.version == expected)
}
