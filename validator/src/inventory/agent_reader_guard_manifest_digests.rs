#[derive(Clone, Copy)]
pub(crate) struct ManifestSpec {
    pub(crate) name: &'static str,
    pub(crate) path: &'static str,
    pub(crate) bytes: &'static [u8],
}

macro_rules! manifest {
    ($name:literal) => {
        ManifestSpec {
            name: $name,
            path: concat!(".codex/agents/", $name, ".toml"),
            bytes: include_bytes!(concat!("../../../.codex/agents/", $name, ".toml")),
        }
    };
}

pub(crate) const MANIFESTS: &[ManifestSpec] = &[
    manifest!("claim-falsifier"),
    manifest!("orchestration-recovery-reviewer"),
    manifest!("product-journey-reviewer"),
    manifest!("repo-recon"),
    manifest!("research-verifier"),
    manifest!("security-reviewer"),
];
