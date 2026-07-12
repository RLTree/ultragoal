mod json;
mod network;
mod semantics;
mod validation;
mod version;

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) use network::{email, https};
pub(crate) use semantics::{SemanticIssue, semantic_issues};
pub(crate) use validation::{
    bounded_text, environment_map, header_map, kebab, semver, unique_list,
};
pub(crate) use version::Version;

pub(crate) const ITEM_LIMIT: usize = 256;
pub(crate) const MANIFEST_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DecodeError {
    ObjectTooLarge,
    InvalidJson,
    InvalidShape,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<PluginAuthor>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub skills: Option<String>,
    pub apps: Option<String>,
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Option<PluginMcpServers>,
    pub interface: Option<PluginInterface>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PluginAuthor {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PluginInterface {
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "shortDescription")]
    pub short_description: Option<String>,
    #[serde(rename = "longDescription")]
    pub long_description: Option<String>,
    #[serde(rename = "developerName")]
    pub developer_name: Option<String>,
    pub category: Option<String>,
    pub capabilities: Vec<String>,
    #[serde(rename = "websiteURL")]
    pub website_url: Option<String>,
    #[serde(rename = "privacyPolicyURL")]
    pub privacy_policy_url: Option<String>,
    #[serde(rename = "termsOfServiceURL")]
    pub terms_url: Option<String>,
    #[serde(rename = "defaultPrompt")]
    pub default_prompt: Option<Vec<String>>,
    #[serde(rename = "brandColor")]
    pub brand_color: Option<String>,
    #[serde(rename = "composerIcon")]
    pub composer_icon: Option<String>,
    pub logo: Option<String>,
    #[serde(rename = "logoDark")]
    pub logo_dark: Option<String>,
    pub screenshots: Vec<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum PluginMcpServers {
    Path(String),
    Inline(BTreeMap<String, PluginMcpServer>),
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PluginMcpServer {
    #[serde(rename = "type")]
    pub kind: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub url: Option<String>,
    pub headers: BTreeMap<String, String>,
}

pub(crate) fn parse(bytes: &[u8], maximum: usize) -> Result<PluginManifest, DecodeError> {
    decode_value(parse_value(bytes, maximum)?)
}

pub(crate) fn parse_value(bytes: &[u8], maximum: usize) -> Result<Value, DecodeError> {
    json::parse(bytes, maximum)
}

pub(crate) fn decode_value(value: Value) -> Result<PluginManifest, DecodeError> {
    serde_json::from_value(value).map_err(|_| DecodeError::InvalidShape)
}

#[cfg(test)]
mod tests {
    use super::{DecodeError, MANIFEST_LIMIT, parse};

    #[test]
    fn supported_shape_is_closed_and_hooks_are_not_supported_authority() {
        let supported =
            br#"{"name":"fixture","version":"1.0.0","description":"fixture","skills":"./skills/"}"#;
        assert!(parse(supported, MANIFEST_LIMIT).is_ok());
        let hooks = br#"{"name":"fixture","version":"1.0.0","description":"fixture","hooks":{}}"#;
        assert!(matches!(
            parse(hooks, MANIFEST_LIMIT),
            Err(DecodeError::InvalidShape)
        ));
    }
}
