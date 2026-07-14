const MARKETPLACE_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarketplaceScope {
    Repository,
    Personal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MarketplaceVerdict {
    Verified,
    Declared,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct MarketplaceExpectation {
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) scope: MarketplaceScope,
    pub(crate) plugin_id: String,
    pub(crate) version: String,
    pub(crate) origin: String,
    pub(crate) package_sha256: String,
}

impl MarketplaceExpectation {
    pub fn new(
        context_id: String,
        candidate_id: String,
        scope: MarketplaceScope,
        plugin_id: String,
        version: String,
        origin: String,
        package_sha256: String,
    ) -> Result<Self, DistributionError> {
        validate_relative_path(&origin)?;
        if !digest(&context_id)
            || !digest(&candidate_id)
            || plugin_id != "harness-ultragoal"
            || crate::plugin_manifest::Version::parse(&version).is_none()
            || !digest(&package_sha256)
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        Ok(Self {
            context_id,
            candidate_id,
            scope,
            plugin_id,
            version,
            origin,
            package_sha256,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MarketplaceSnapshot {
    context_id: String,
    candidate_id: String,
    scope: MarketplaceScope,
    plugin_id: String,
    version: String,
    verdict: MarketplaceVerdict,
    catalog_sha256: Option<String>,
    package_sha256: String,
}

impl MarketplaceSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub const fn scope(&self) -> MarketplaceScope {
        self.scope
    }
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub const fn verdict(&self) -> MarketplaceVerdict {
        self.verdict
    }
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }
    pub fn catalog_sha256(&self) -> Option<&str> {
        self.catalog_sha256.as_deref()
    }
}

pub(crate) fn snapshot(
    expected: &MarketplaceExpectation,
    verdict: MarketplaceVerdict,
    catalog_sha256: Option<String>,
) -> MarketplaceSnapshot {
    MarketplaceSnapshot {
        context_id: expected.context_id.clone(),
        candidate_id: expected.candidate_id.clone(),
        scope: expected.scope,
        plugin_id: expected.plugin_id.clone(),
        version: expected.version.clone(),
        verdict,
        catalog_sha256,
        package_sha256: expected.package_sha256.clone(),
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexMarketplace {
    name: String,
    interface: CodexInterface,
    plugins: Vec<CodexPlugin>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexInterface {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodexPlugin {
    name: String,
    source: CodexSource,
    policy: CodexPolicy,
    category: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexSource {
    source: String,
    path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CodexPolicy {
    installation: String,
    authentication: String,
}

impl CodexPlugin {
    pub fn harness_ultragoal() -> Self {
        Self {
            name: "harness-ultragoal".into(),
            source: CodexSource {
                source: "local".into(),
                path: "./plugins/harness-ultragoal".into(),
            },
            policy: CodexPolicy {
                installation: "AVAILABLE".into(),
                authentication: "ON_INSTALL".into(),
            },
            category: "Productivity".into(),
        }
    }
}

#[derive(Clone)]
pub struct MarketplacePlan {
    expected_sha256: Option<String>,
    replacement: Vec<u8>,
    pub(crate) rollback: Option<Vec<u8>>,
    package: PackageIdentity,
}

impl MarketplacePlan {
    pub fn replacement(&self) -> &[u8] {
        &self.replacement
    }
    pub fn expected_sha256(&self) -> Option<&str> {
        self.expected_sha256.as_deref()
    }
    pub fn package(&self) -> &PackageIdentity {
        &self.package
    }
}
