use super::environment::InvocationSensitivity;
use super::util::digest_bytes;
use crate::context::{EffectClass, LiveContext, ToolCapability};
use serde::Serialize;
use std::path::PathBuf;

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

const BASE_PROFILE: &str = "(version 1)\n\
(allow default)\n\
(deny network*)\n\
(deny file-write*)\n\
(deny file-clone file-link)\n\
(deny process-fork)\n\
(deny file-read* (literal (param \"PROHIBITED_ENV\")))\n";

#[derive(Clone, Debug)]
pub(super) struct SandboxPlan {
    executable: PathBuf,
    profile: String,
    executable_sha256: String,
    worktree_read_scope: String,
    prohibited_env: String,
    write_scopes: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct EnforcementRecord {
    substrate: &'static str,
    executable: String,
    executable_sha256: String,
    pub(super) profile_sha256: String,
    network_allowed: bool,
    process_fork_allowed: bool,
    external_write_allowed: bool,
    execution_allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    worktree_read_scope: Option<String>,
    prohibited_read_path: &'static str,
    write_scopes: Vec<String>,
    preflight_passed: bool,
}

impl SandboxPlan {
    #[cfg(target_os = "macos")]
    pub fn prepare(context: &LiveContext, effect: EffectClass) -> Result<Self, String> {
        match effect {
            EffectClass::Read => {}
            EffectClass::PlannedWrite => {
                return Err(
                    "capture effect unsupported: PlannedWrite requires an accepted typed-catalog binding"
                        .to_owned(),
                );
            }
            EffectClass::WorkspaceWrite => {
                return Err(
                    "capture effect unsupported: WorkspaceWrite requires a descriptor-brokered write substrate"
                        .to_owned(),
                );
            }
            EffectClass::ExternalWrite => {
                return Err(
                    "capture effect unsupported: ExternalWrite has no explicit target authority"
                        .to_owned(),
                );
            }
            EffectClass::Destructive => {
                return Err(
                    "capture effect unsupported: Destructive requires a descriptor-brokered write substrate"
                        .to_owned(),
                );
            }
        }
        let sandbox = fixed_capability(context, "sandbox-exec", SANDBOX_EXEC)?;
        let profile = profile_for(effect)?;
        let worktree = context
            .worktree_root()
            .to_str()
            .ok_or_else(|| "capture worktree path is not UTF-8 for sandbox binding".to_owned())?;
        let prohibited_env = context
            .worktree_root()
            .join(".codex-worktree/env.sh")
            .to_str()
            .ok_or_else(|| "capture prohibited path is not UTF-8".to_owned())?
            .to_owned();
        Ok(Self {
            executable: PathBuf::from(SANDBOX_EXEC),
            profile,
            executable_sha256: sandbox
                .executable_sha256
                .clone()
                .expect("fixed capability requires digest"),
            worktree_read_scope: worktree.to_owned(),
            prohibited_env,
            write_scopes: Vec::new(),
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub fn prepare(_context: &LiveContext, _effect: EffectClass) -> Result<Self, String> {
        Err("capture enforcement unsupported: no proven host sandbox on this platform".to_owned())
    }

    pub fn record(
        &self,
        preflight_passed: bool,
        sensitivity: InvocationSensitivity,
    ) -> EnforcementRecord {
        EnforcementRecord {
            substrate: "macos-sandbox-native-read-v1",
            executable: self.executable.display().to_string(),
            executable_sha256: self.executable_sha256.clone(),
            profile_sha256: digest_bytes(self.profile.as_bytes()),
            network_allowed: false,
            process_fork_allowed: false,
            external_write_allowed: false,
            execution_allowed: true,
            worktree_read_scope: (!sensitivity.is_secret_bearing())
                .then(|| self.worktree_read_scope.clone()),
            prohibited_read_path: ".codex-worktree/env.sh",
            write_scopes: self.write_scopes.clone(),
            preflight_passed,
        }
    }

    pub fn executable(&self) -> &std::path::Path {
        &self.executable
    }

    pub fn profile(&self) -> &str {
        &self.profile
    }

    pub fn prohibited_env(&self) -> &str {
        &self.prohibited_env
    }
}

fn fixed_capability<'a>(
    context: &'a LiveContext,
    name: &str,
    expected: &str,
) -> Result<&'a ToolCapability, String> {
    let tool = context
        .capabilities()
        .tool(name)
        .filter(|tool| tool.available)
        .ok_or_else(|| {
            format!("capture enforcement unavailable: LiveContext lacks {name} capability")
        })?;
    if tool.executable.as_deref() != Some(expected)
        || tool.executable_sha256.as_deref().is_none_or(str::is_empty)
        || tool.byte_length.is_none_or(|length| length == 0)
        || tool.unix_mode.is_none_or(|mode| mode & 0o111 == 0)
    {
        return Err(format!(
            "capture enforcement unavailable: {name} capability is not fixed at {expected}"
        ));
    }
    Ok(tool)
}

fn profile_for(effect: EffectClass) -> Result<String, String> {
    match effect {
        EffectClass::Read => Ok(BASE_PROFILE.to_owned()),
        EffectClass::PlannedWrite => Err(
            "capture effect unsupported: PlannedWrite requires an accepted typed-catalog binding"
                .to_owned(),
        ),
        EffectClass::WorkspaceWrite => Err(
            "capture effect unsupported: WorkspaceWrite requires a descriptor-brokered write substrate"
                .to_owned(),
        ),
        EffectClass::ExternalWrite => {
            Err("capture effect unsupported: external target authority is absent".to_owned())
        }
        EffectClass::Destructive => Err(
            "capture effect unsupported: Destructive requires a descriptor-brokered write substrate"
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::profile_for;
    use crate::context::EffectClass;

    #[test]
    fn read_profile_has_no_write_network_or_fork_allowance() {
        let profile = profile_for(EffectClass::Read).unwrap();
        assert!(profile.starts_with("(version 1)\n(allow default)"));
        assert!(profile.contains("(deny file-write*)"));
        assert!(profile.contains("(deny network*)"));
        assert!(profile.contains("(deny file-clone file-link)"));
        assert!(profile.contains("(deny process-fork)"));
        assert!(profile.contains("(deny file-read* (literal (param \"PROHIBITED_ENV\")))"));
        assert!(!profile.contains("(allow file-write"));
        assert!(!profile.contains("/bin/sh"));
        assert!(!profile.contains("/System"));
        assert!(!profile.contains("(allow network"));
        assert!(!profile.contains("(allow process-fork"));
        assert!(!profile.contains("(allow file-link"));
    }

    #[test]
    fn write_effects_reject_until_descriptor_broker_exists() {
        assert!(profile_for(EffectClass::WorkspaceWrite).is_err());
        assert!(profile_for(EffectClass::ExternalWrite).is_err());
        assert!(profile_for(EffectClass::Destructive).is_err());
        assert!(profile_for(EffectClass::PlannedWrite).is_err());
    }
}
