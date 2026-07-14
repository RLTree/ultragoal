use crate::plugin_product::agent_discovery::filesystem::digest;

pub(super) enum RootIdentityCodecRequest<'a> {
    SupportedSet {
        package: &'a str,
        installed: &'a str,
        cache: &'a str,
        global: &'a str,
        project: &'a str,
    },
    PluginRoot {
        root: &'a str,
        agents: &'a str,
        plugin: &'a str,
    },
    GlobalRoot {
        root: &'a str,
        agents: &'a str,
    },
    LayerFiles {
        authority_root_sha256: &'a str,
        manifest_sha256: Option<&'a str>,
        agents: &'a [(Vec<u8>, &'a str)],
    },
}

pub(super) struct RootIdentityCodecResponse {
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RootIdentityCodecError {
    Encode,
}

impl RootIdentityCodecResponse {
    pub(super) fn sha256(self) -> String {
        self.sha256
    }
}

pub(super) fn encode(
    request: RootIdentityCodecRequest<'_>,
) -> Result<RootIdentityCodecResponse, RootIdentityCodecError> {
    let bytes = match request {
        RootIdentityCodecRequest::SupportedSet {
            package,
            installed,
            cache,
            global,
            project,
        } => serde_json::to_vec(&(
            "SupportedHostAgentRoots-v1",
            package,
            installed,
            cache,
            global,
            project,
        )),
        RootIdentityCodecRequest::PluginRoot {
            root,
            agents,
            plugin,
        } => serde_json::to_vec(&("PluginAuthorityRoot-v1", root, agents, plugin)),
        RootIdentityCodecRequest::GlobalRoot { root, agents } => {
            serde_json::to_vec(&("GlobalAuthorityRoot-v1", root, agents))
        }
        RootIdentityCodecRequest::LayerFiles {
            authority_root_sha256,
            manifest_sha256,
            agents,
        } => serde_json::to_vec(&(
            "SupportedAgentLayerFiles-v1",
            authority_root_sha256,
            manifest_sha256,
            agents,
        )),
    }
    .map_err(|_| RootIdentityCodecError::Encode)?;
    Ok(RootIdentityCodecResponse {
        sha256: digest(&bytes),
    })
}
