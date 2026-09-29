//! Narrow TypeSafe credential selection and macOS Keychain process effect.
use super::{Target, SYNTHETIC_BEARER, DEFAULT_KEYCHAIN_SERVICE};
use crate::{option, process::{run, Process}};
use std::{env, process::Command, time::Duration};

// Provider credential selection is a local host effect. The chosen bytes are
// never placed in a report or command argument, and invalid forms fail closed.
pub(super) struct ProviderCredential { pub(super) secret: String }
#[derive(Debug)]
pub(super) enum CredentialError { LocalDisabled, Argument, MissingService, Lookup, Encoding, Invalid }
impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LocalDisabled => "provider disabled by --local",
            Self::Argument => "credential route argument invalid",
            Self::MissingService => "keychain service required",
            Self::Lookup => "credential lookup unavailable",
            Self::Encoding => "credential encoding",
            Self::Invalid => "invalid credential representation",
        })
    }
}
pub(super) fn validate_credential(raw: String) -> Result<ProviderCredential, CredentialError> {
    let secret = raw.trim().to_string();
    if secret.is_empty() || secret.len() > 1024 || !secret.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(CredentialError::Invalid);
    }
    Ok(ProviderCredential { secret })
}
pub(super) fn key(args: &[String], target: &Target) -> Result<ProviderCredential, CredentialError> {
    if target.name != "typesafe" { return validate_credential(SYNTHETIC_BEARER.into()); }
    if args.iter().any(|arg| arg == "--local") { return Err(CredentialError::LocalDisabled); }
    let explicit = option(args, "--keychain-service").map_err(|_| CredentialError::Argument)?;
    match explicit {
        Some(service) => keychain_key(&service),
        None => match env::var("TYPESAFE_API_KEY") {
            Ok(value) => validate_credential(value),
            Err(env::VarError::NotPresent) => keychain_key(DEFAULT_KEYCHAIN_SERVICE),
            Err(env::VarError::NotUnicode(_)) => Err(CredentialError::Invalid),
        },
    }
}


// Direct Keychain process effect; no subprocess output enters errors or reports.
fn keychain_key(service: &str) -> Result<ProviderCredential, CredentialError> {
    if service.is_empty() { return Err(CredentialError::MissingService); }
    let mut command = Command::new("/usr/bin/security");
    command.args(["find-generic-password", "-s", service, "-w"]);
    let observed = run(command, Vec::new(), Duration::from_secs(2)).map_err(|_| CredentialError::Lookup)?;
    accept_lookup(observed)
}

fn accept_lookup(observed: Process) -> Result<ProviderCredential, CredentialError> {
    if observed.code != Some(0) || observed.timeout || observed.cancelled || observed.truncated
        || !observed.stdout_complete || !observed.stderr_complete
    {
        return Err(CredentialError::Lookup);
    }
    let raw = String::from_utf8(observed.out).map_err(|_| CredentialError::Encoding)?;
    validate_credential(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observed(out: &[u8]) -> Process {
        Process { code: Some(0), signal: None, out: out.to_vec(), err: Vec::new(),
                  timeout: false, cancelled: false, truncated: false, input_complete: true,
                  stdout_complete: true, stderr_complete: true, child_pid: 1,
                  started_unix_ms: 0, ended_unix_ms: 1, ms: 1 }
    }
    #[test]
    fn keychain_observation_must_be_complete_and_validated_before_use() {
        assert!(matches!(accept_lookup(observed(b"bad key")), Err(CredentialError::Invalid)));
        assert!(matches!(accept_lookup(observed(b"\xff")), Err(CredentialError::Encoding)));
        let mut truncated = observed(b"valid-key");
        truncated.truncated = true;
        assert!(matches!(accept_lookup(truncated), Err(CredentialError::Lookup)));
        let mut failed = observed(b"valid-key");
        failed.code = Some(1);
        assert!(matches!(accept_lookup(failed), Err(CredentialError::Lookup)));
        assert_eq!(accept_lookup(observed(b"valid-key\n")).unwrap().secret, "valid-key");
    }
}
