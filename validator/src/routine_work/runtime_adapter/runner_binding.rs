use super::*;
use crate::routine_work::{CHILD_MODE_ENV, CHILD_MODE_VALUE};

pub(crate) fn validate_immutable_routine_program(path: &Path) -> Result<(), RoutineError> {
    mediator::validate_routine_program_path(path)
}

pub(crate) fn runner_identity(
    tool: &ToolCapability,
    tool_identity_sha256: String,
) -> Result<RunnerIdentity, RoutineError> {
    let executable = tool
        .executable
        .as_deref()
        .ok_or_else(|| adapter_error("adapter-runner-program-path-missing"))?;
    let executable_sha256 = tool
        .executable_sha256
        .as_deref()
        .ok_or_else(|| adapter_error("adapter-runner-program-identity-missing"))?;
    let program_sha256 = format!("sha256:{executable_sha256}");
    let program_byte_length = tool
        .byte_length
        .filter(|length| *length > 0)
        .ok_or_else(|| adapter_error("adapter-runner-program-length-invalid"))?;
    if !valid(&tool_identity_sha256) || !valid(&program_sha256) {
        return Err(adapter_error("adapter-runner-program-identity-invalid"));
    }
    Ok(RunnerIdentity {
        tool_name: tool.name.clone(),
        tool_identity_sha256,
        program_path_hex: hex(executable.as_bytes()),
        program_sha256,
        program_byte_length,
        program_unix_mode: tool.unix_mode,
        program_path: executable.to_owned(),
    })
}

pub(crate) fn validate_bound_invocation(
    invocation: &RoutineInvocationSpec,
    runner: &RunnerIdentity,
    check: &PlannedCheck,
) -> Result<(), RoutineError> {
    let expected_environment = default_environment(runner)?;
    if invocation.behavior_id != RUST_SOURCE_SYNTAX_BEHAVIOR
        || invocation.tool_name != "ultragoal"
        || invocation.arguments != RUST_SOURCE_SYNTAX_ARGUMENTS
        || invocation.environment != expected_environment
        || invocation.read_sources.is_empty()
        || invocation.node_id != check.node_id()
        || invocation.tool_name != runner.tool_name
        || invocation.tool_identity_sha256 != runner.tool_identity_sha256
        || invocation.program_path_hex != runner.program_path_hex
        || invocation.program_sha256 != runner.program_sha256
        || invocation.program_byte_length != runner.program_byte_length
        || invocation.program_unix_mode != runner.program_unix_mode
        || environment_digest(&invocation.environment)? != invocation.environment_sha256
        || read_authority_digest(&invocation.read_sources)? != invocation.read_authority_sha256
    {
        return Err(adapter_error("adapter-runner-binding-mismatch"));
    }
    Ok(())
}

pub(crate) fn validate_and_normalize_bound_invocation(
    mut invocation: RoutineInvocationSpec,
) -> Result<RoutineInvocationSpec, RoutineError> {
    invocation.declared_output_scopes =
        normalized_output_scopes(invocation.declared_output_scopes)?;
    validate_execution_policy(
        &invocation.arguments,
        &invocation.environment,
        invocation.timeout_ms,
        invocation.output_budget_bytes,
        &invocation.declared_output_scopes,
    )?;
    if invocation.behavior_id != RUST_SOURCE_SYNTAX_BEHAVIOR
        || invocation.tool_name != "ultragoal"
        || invocation.arguments != RUST_SOURCE_SYNTAX_ARGUMENTS
        || invocation.read_sources.is_empty()
    {
        return Err(adapter_error("adapter-closed-behavior-binding-invalid"));
    }
    Ok(invocation)
}

pub(crate) fn validate_execution_policy(
    arguments: &[String],
    environment: &BTreeMap<String, String>,
    timeout_ms: u64,
    output_budget_bytes: u64,
    output_scopes: &[RepoPath],
) -> Result<(), RoutineError> {
    let total_argument_bytes = arguments.iter().map(String::len).sum::<usize>();
    if arguments.len() > MAX_ARGUMENTS
        || total_argument_bytes > MAX_ARGUMENT_BYTES
        || arguments.iter().any(|argument| {
            argument.is_empty()
                || argument.len() > 4_096
                || argument.bytes().any(|byte| byte.is_ascii_control())
        })
    {
        return Err(adapter_error("adapter-argv-invalid"));
    }
    validate_environment(environment)?;
    if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
        return Err(adapter_error("adapter-timeout-invalid"));
    }
    if output_budget_bytes == 0 || output_budget_bytes > MAX_OUTPUT_BUDGET_BYTES {
        return Err(adapter_error("adapter-output-budget-invalid"));
    }
    if output_scopes.len() > MAX_OUTPUT_SCOPES {
        return Err(adapter_error("adapter-output-scope-limit-exceeded"));
    }
    Ok(())
}

pub(crate) fn default_environment(
    runner: &RunnerIdentity,
) -> Result<BTreeMap<String, String>, RoutineError> {
    fixed_environment(Path::new(&runner.program_path))
}

pub(crate) fn fixed_environment(program: &Path) -> Result<BTreeMap<String, String>, RoutineError> {
    let parent = program
        .parent()
        .and_then(Path::to_str)
        .ok_or_else(|| adapter_error("adapter-runner-program-parent-invalid"))?;
    Ok(BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("PATH".to_owned(), parent.to_owned()),
        (CHILD_MODE_ENV.to_owned(), CHILD_MODE_VALUE.to_owned()),
    ]))
}

pub(crate) fn validate_environment(
    environment: &BTreeMap<String, String>,
) -> Result<(), RoutineError> {
    let total_bytes = environment
        .iter()
        .map(|(key, value)| key.len().saturating_add(value.len()))
        .sum::<usize>();
    if environment.len() > MAX_ENVIRONMENT_ENTRIES
        || total_bytes > MAX_ENVIRONMENT_BYTES
        || environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 128
                || (key.starts_with("HUL_ROUTINE_")
                    && (key != CHILD_MODE_ENV || value != CHILD_MODE_VALUE))
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
                || value.len() > 4_096
                || value
                    .bytes()
                    .any(|byte| byte == 0 || byte.is_ascii_control())
        })
    {
        return Err(adapter_error("adapter-environment-invalid"));
    }
    if environment
        .keys()
        .any(|key| startup_loader_environment_key(key))
    {
        return Err(adapter_error("adapter-environment-startup-loader-refused"));
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

pub(crate) fn environment_digest(
    environment: &BTreeMap<String, String>,
) -> Result<String, RoutineError> {
    validate_environment(environment)?;
    digest_of(environment)
}

pub(crate) fn read_authority_digest(
    read_sources: &[RoutineReadSource],
) -> Result<String, RoutineError> {
    digest_of(read_sources)
}

pub(crate) fn normalized_read_source_paths(
    mut sources: Vec<RepoPath>,
) -> Result<Vec<RepoPath>, RoutineError> {
    if sources.len() > MAX_READ_SOURCES {
        return Err(adapter_error("adapter-read-source-limit-exceeded"));
    }
    sources.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    let mut case_keys = BTreeSet::new();
    if sources
        .iter()
        .any(|source| !case_keys.insert(source.case_key()))
    {
        return Err(adapter_error("adapter-read-source-duplicated"));
    }
    Ok(sources)
}
