use super::{ExactFile, FileRule, ScanRoot, SourceClass};

pub(in crate::audit::source_governance) const EXACT_FILES: &[ExactFile] = &[
    exact("validator/build.rs", SourceClass::RustBuild),
    exact("Cargo.toml", SourceClass::BuildConfiguration),
    exact("Cargo.lock", SourceClass::BuildConfiguration),
    exact("validator/Cargo.toml", SourceClass::BuildConfiguration),
    exact("rust-toolchain.toml", SourceClass::BuildConfiguration),
    exact(
        "plugin-manifest-draft.json",
        SourceClass::PluginConfiguration,
    ),
    exact("package.json", SourceClass::PluginConfiguration),
    exact("pnpm-workspace.yaml", SourceClass::BuildConfiguration),
    exact("pnpm-lock.yaml", SourceClass::BuildConfiguration),
    exact("AGENTS.md", SourceClass::LiveStandard),
    exact("AGENT_STANDARDS.md", SourceClass::LiveStandard),
    root_document("README.md"),
    root_document("ARCHITECTURE.md"),
    root_document("PLANS.md"),
    root_document("SECURITY.md"),
    root_document("DESIGN.md"),
    root_document("FRONTEND.md"),
    root_document("RELIABILITY.md"),
    root_document("PRODUCT_SENSE.md"),
    root_document("PRODUCT_FITNESS.md"),
    root_document("QUALITY_SCORE.md"),
    root_document("PRODUCT_SUCCESS_CONTRACT.md"),
];

const fn exact(relative: &'static str, class: SourceClass) -> ExactFile {
    ExactFile {
        relative,
        class,
        required: true,
    }
}

const fn root_document(relative: &'static str) -> ExactFile {
    exact(relative, SourceClass::LiveRootDocument)
}

pub(in crate::audit::source_governance) const DIRECTORY_ROOTS: &[ScanRoot] = &[
    required("validator/src", SourceClass::RustProduction, FileRule::Rust),
    required("validator/tests", SourceClass::RustTest, FileRule::Rust),
    required(
        "validator/build_support",
        SourceClass::RustBuildSupport,
        FileRule::Rust,
    ),
    required(
        "validator/examples",
        SourceClass::RustExample,
        FileRule::Rust,
    ),
    required("scripts", SourceClass::RepoCheck, FileRule::AllRegular),
    required(".harness", SourceClass::RepoCheck, FileRule::AllRegular),
    required(
        "agent-standards",
        SourceClass::AgentStandard,
        FileRule::AllRegular,
    ),
    required(
        "templates",
        SourceClass::PackageTemplate,
        FileRule::AllRegular,
    ),
    required("schemas", SourceClass::Schema, FileRule::AllRegular),
    required(
        "migration",
        SourceClass::MigrationRegistry,
        FileRule::AllRegular,
    ),
    required(
        "docs",
        SourceClass::ActiveDocumentation,
        FileRule::ActiveDocumentation,
    ),
    required(
        ".codex-plugin",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        ".cargo",
        SourceClass::BuildConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        ".codex/agents",
        SourceClass::PluginAgent,
        FileRule::AllRegular,
    ),
    optional(
        ".codex/environments",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional("agents", SourceClass::PluginAgent, FileRule::AllRegular),
    optional(
        "custom-agents",
        SourceClass::PluginAgent,
        FileRule::AllRegular,
    ),
    optional("commands", SourceClass::PluginCommand, FileRule::AllRegular),
    optional("skills", SourceClass::PluginSkill, FileRule::AllRegular),
    optional(
        "hooks",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        "mcp",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        "config",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        "connectors",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
    optional(
        "install",
        SourceClass::PluginConfiguration,
        FileRule::AllRegular,
    ),
];

const fn required(relative: &'static str, class: SourceClass, rule: FileRule) -> ScanRoot {
    ScanRoot {
        relative,
        class,
        rule,
        required: true,
    }
}

const fn optional(relative: &'static str, class: SourceClass, rule: FileRule) -> ScanRoot {
    ScanRoot {
        relative,
        class,
        rule,
        required: false,
    }
}
