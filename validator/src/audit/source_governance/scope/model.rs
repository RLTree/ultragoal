#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum SourceClass {
    ActiveDocumentation,
    AgentStandard,
    BuildConfiguration,
    LiveStandard,
    LiveRootDocument,
    MigrationRegistry,
    PackageTemplate,
    PluginAgent,
    PluginCommand,
    PluginConfiguration,
    PluginSkill,
    RepoCheck,
    RustBuild,
    RustBuildSupport,
    RustExample,
    RustProduction,
    RustTest,
    Schema,
}

impl SourceClass {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::ActiveDocumentation => "active_documentation",
            Self::AgentStandard => "agent_standard",
            Self::BuildConfiguration => "build_configuration",
            Self::LiveStandard => "live_standard",
            Self::LiveRootDocument => "live_root_document",
            Self::MigrationRegistry => "migration_registry",
            Self::PackageTemplate => "package_template",
            Self::PluginAgent => "plugin_agent",
            Self::PluginCommand => "plugin_command",
            Self::PluginConfiguration => "plugin_configuration",
            Self::PluginSkill => "plugin_skill",
            Self::RepoCheck => "repo_check",
            Self::RustBuild => "rust_build",
            Self::RustBuildSupport => "rust_build_support",
            Self::RustExample => "rust_example",
            Self::RustProduction => "rust_production",
            Self::RustTest => "rust_test",
            Self::Schema => "schema",
        }
    }
}

pub(in crate::audit::source_governance) struct ScanRoot {
    pub(in crate::audit::source_governance) relative: &'static str,
    pub(in crate::audit::source_governance) class: SourceClass,
    pub(in crate::audit::source_governance) rule: FileRule,
    pub(in crate::audit::source_governance) required: bool,
}

pub(in crate::audit::source_governance) struct ExactFile {
    pub(in crate::audit::source_governance) relative: &'static str,
    pub(in crate::audit::source_governance) class: SourceClass,
    pub(in crate::audit::source_governance) required: bool,
}

#[derive(Clone, Copy)]
pub(in crate::audit::source_governance) enum FileRule {
    ActiveDocumentation,
    AllRegular,
    Rust,
}
