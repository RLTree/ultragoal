use crate::context::LiveContext;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(super) const MISSING_CAUSE: &str = "supported Codex help/listing could not be observed through the OS-enforced zero-write boundary; no action was emitted";
pub(super) const MISSING_CODE: &str = "supported-host-zero-write-oracle-unavailable";

const ADD_USAGE: &str = "Usage: codex plugin add [OPTIONS] <PLUGIN[@MARKETPLACE]>";

pub(super) struct SupportedHostOracleObservation {
    pub(super) executable: PathBuf,
    pub(super) executable_content_sha256: String,
    pub(super) executable_binding_sha256: String,
    pub(super) plugin_add_help_sha256: String,
    pub(super) plugin_list_json: Vec<u8>,
    pub(super) marketplace_list_json: Vec<u8>,
}

/// Observes one exact selected Codex executable through the reviewed macOS
/// runtime boundary. The child receives only fixed help/list commands and an
/// exact HOME/CODEX_HOME pair; the OS denies all writes, network, and forks.
pub(super) fn observe(
    context: &LiveContext,
    home: &Path,
    working_directory: &Path,
) -> Result<SupportedHostOracleObservation, &'static str> {
    context
        .revalidate()
        .map_err(|_| "host context changed before supported-host observation")?;
    let tool = context
        .capabilities()
        .tool("codex")
        .filter(|tool| tool.available)
        .ok_or("selected Codex executable is unavailable")?;
    let executable = tool
        .executable
        .as_deref()
        .map(Path::new)
        .ok_or("selected Codex executable is unavailable")?;
    let content_sha256 = tool
        .executable_sha256
        .as_deref()
        .filter(|value| valid_sha256(value))
        .map(|value| format!("sha256:{value}"))
        .ok_or("selected Codex executable identity is unavailable")?;
    let (
        observed_executable,
        observed_content_sha256,
        executable_binding_sha256,
        plugin_add_help,
        plugin_list_json,
        marketplace_list_json,
    ) = crate::distribution::host_effect::SelectedCodexExecutable::observe_supported_read_only(
        executable,
        &content_sha256,
        home,
        working_directory,
    )
    .map_err(|_| MISSING_CAUSE)?;
    if observed_executable != executable
        || observed_content_sha256 != content_sha256
        || !valid_prefixed_sha256(&executable_binding_sha256)
    {
        return Err("selected Codex executable changed during supported-host observation");
    }
    let help = std::str::from_utf8(&plugin_add_help)
        .map_err(|_| "supported Codex plugin-add help is not UTF-8")?;
    if !help.lines().any(|line| line.trim() == ADD_USAGE)
        || !help.lines().any(|line| line.trim() == "Arguments:")
        || !help.contains("<PLUGIN[@MARKETPLACE]>")
    {
        return Err("supported Codex plugin-add grammar is unavailable");
    }
    context
        .revalidate()
        .map_err(|_| "host context changed during supported-host observation")?;
    Ok(SupportedHostOracleObservation {
        executable: observed_executable,
        executable_content_sha256: observed_content_sha256,
        executable_binding_sha256,
        plugin_add_help_sha256: digest(&plugin_add_help),
        plugin_list_json,
        marketplace_list_json,
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_prefixed_sha256(value: &str) -> bool {
    value.len() == 71 && value.starts_with("sha256:") && valid_sha256(&value[7..])
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
