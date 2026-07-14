use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DarwinHostSurface {
    Installed,
    Cache,
    Marketplace,
    AppRegistry,
    PluginsUi,
    Discovery,
    Runtime,
}

impl DarwinHostSurface {
    pub const ALL: [Self; 7] = [
        Self::Installed,
        Self::Cache,
        Self::Marketplace,
        Self::AppRegistry,
        Self::PluginsUi,
        Self::Discovery,
        Self::Runtime,
    ];

    pub const fn relative_path(self) -> &'static str {
        match self {
            Self::Installed => "host-lifecycle/surfaces/installed",
            Self::Cache => "host-lifecycle/surfaces/cache",
            Self::Marketplace => "host-lifecycle/surfaces/marketplace",
            Self::AppRegistry => "host-lifecycle/surfaces/app-registry",
            Self::PluginsUi => "host-lifecycle/surfaces/plugins-ui",
            Self::Discovery => "host-lifecycle/surfaces/discovery",
            Self::Runtime => "host-lifecycle/surfaces/runtime",
        }
    }
}
