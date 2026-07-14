use super::*;

pub(crate) fn absolute_program_path(value: String) -> CatalogResult<PathBuf> {
    let path = PathBuf::from(&value);
    if value.len() > 4096
        || value.ends_with('/')
        || value.contains("//")
        || value.bytes().any(|byte| byte.is_ascii_control())
        || !path.is_absolute()
        || path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
    {
        return Err(error("catalog-runner-path-invalid"));
    }
    Ok(path)
}

pub(crate) fn validate_global_runner_compatibility(
    definitions: &BTreeMap<String, RoutineDefinition>,
) -> CatalogResult<()> {
    let mut authorities = BTreeMap::<String, (&str, (&str, &Path, &str, u64, u32))>::new();
    for recipe in definitions.values().flat_map(|definition| {
        std::iter::once(&definition.primary).chain(definition.fallback.as_ref())
    }) {
        let authority = (
            recipe.tool_identity_sha256.as_str(),
            recipe.executable_path.as_path(),
            recipe.program_sha256.as_str(),
            recipe.program_byte_length,
            recipe.program_unix_mode,
        );
        let canonical_tool = recipe.tool.to_ascii_lowercase();
        if let Some((accepted_spelling, accepted_authority)) = authorities.get(&canonical_tool) {
            if *accepted_spelling != recipe.tool {
                return Err(error("catalog-runner-spelling-ambiguous"));
            }
            if *accepted_authority != authority {
                return Err(error("catalog-runner-authority-ambiguous"));
            }
        } else {
            authorities.insert(canonical_tool, (recipe.tool.as_str(), authority));
        }
    }
    Ok(())
}

pub(crate) fn validate_arguments(arguments: &[String]) -> CatalogResult<()> {
    let total = arguments.iter().map(String::len).sum::<usize>();
    if arguments.len() > MAX_ARGUMENTS
        || total > MAX_ARGUMENT_BYTES
        || arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > 4_096
                || argument.bytes().any(|byte| byte.is_ascii_control())
        })
    {
        return Err(error("catalog-arguments-invalid"));
    }
    Ok(())
}

pub(crate) fn validate_environment(environment: &BTreeMap<String, String>) -> CatalogResult<()> {
    let total = environment
        .iter()
        .map(|(key, value)| key.len().saturating_add(value.len()))
        .sum::<usize>();
    if environment.len() > MAX_ENVIRONMENT_ENTRIES
        || total > MAX_ENVIRONMENT_BYTES
        || environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || matches!(key.as_str(), "LANG" | "LC_ALL" | "PATH")
                || key.starts_with("HUL_ROUTINE_")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || value.len() > 4_096
                || value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
                || startup_loader_environment_key(key)
        })
    {
        return Err(error("catalog-environment-invalid"));
    }
    Ok(())
}

pub(crate) fn validate_bound_environment(
    environment: &BTreeMap<String, String>,
) -> CatalogResult<()> {
    let total = environment
        .iter()
        .map(|(key, value)| key.len().saturating_add(value.len()))
        .sum::<usize>();
    if environment.len() > 64
        || total > 64 * 1024
        || environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || key.starts_with("HUL_ROUTINE_")
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || value.len() > 4_096
                || value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
                || startup_loader_environment_key(key)
        })
    {
        return Err(error("catalog-bound-environment-invalid"));
    }
    Ok(())
}

pub(crate) fn startup_loader_environment_key(key: &str) -> bool {
    const EXACT: &[&str] = &[
        "BASH_ENV",
        "BASH_LOADABLES_PATH",
        "CLASSPATH",
        "ENV",
        "GEM_HOME",
        "GEM_PATH",
        "JDK_JAVA_OPTIONS",
        "LD_AUDIT",
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "LIBPATH",
        "NODE_OPTIONS",
        "NODE_PATH",
        "PERL5LIB",
        "PERL5OPT",
        "PERLLIB",
        "PHP_INI_SCAN_DIR",
        "PHPRC",
        "PYTHONBREAKPOINT",
        "PYTHONHOME",
        "PYTHONINSPECT",
        "PYTHONPATH",
        "PYTHONSTARTUP",
        "PYTHONUSERBASE",
        "RUBYLIB",
        "RUBYOPT",
        "RUBYPATH",
        "SHLIB_PATH",
        "ZDOTDIR",
    ];
    EXACT.contains(&key)
        || key.starts_with("DYLD_")
        || key.starts_with("LD_PRELOAD_")
        || key.ends_with("_STARTUP")
        || key.ends_with("_TOOL_OPTIONS")
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct CatalogPath(String);

impl CatalogPath {
    pub(crate) fn parse(value: String) -> CatalogResult<Self> {
        let depth = value.split('/').count();
        if value.is_empty()
            || value.len() > 4_096
            || depth > 64
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains("//")
            || value.contains('\\')
            || value.bytes().any(|byte| byte.is_ascii_control())
            || value
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
        {
            return Err(error("catalog-repository-relative-path-required"));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
