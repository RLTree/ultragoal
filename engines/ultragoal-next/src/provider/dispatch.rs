//! Only a screened request can reach the fixed TypeSafe transport process.
use super::{Target, contains_credential, MAX_RESPONSE};
use super::credential::{self, CredentialError, ProviderCredential};
use crate::{evaluate_until, process::Process, row};
use serde_json::Value;
use std::{process::Command, time::{Duration, Instant}};

pub(super) enum PrepareError {
    Credential(CredentialError),
    Disclosure(&'static str),
    Core(String),
}

pub(super) struct Screened<'a> {
    target: &'a Target,
    raw: &'a [u8],
    credential: ProviderCredential,
}

pub(super) fn prepare<'a>(
    args: &[String], target: &'a Target, request: &Value, raw: &'a [u8], deadline: Instant,
) -> Result<Screened<'a>, PrepareError> {
    let credential = credential::key(args, target).map_err(PrepareError::Credential)?;
    let key = &credential.secret;
    if raw.windows(key.len()).any(|part| part == key.as_bytes()) || contains_credential(request, key) {
        return Err(PrepareError::Disclosure("packet contains the active credential; nothing sent"));
    }
    let text = std::str::from_utf8(raw).map_err(|_| PrepareError::Disclosure("request UTF-8"))?;
    let screen = evaluate_until("SECRET_SCREEN\n".to_string() + &row(&[text]), deadline)
        .map_err(PrepareError::Core)?;
    if screen != vec![vec!["SCREEN".to_string(), "clear".to_string()]] {
        return Err(PrepareError::Disclosure("packet contains a credential-shaped value; nothing sent"));
    }
    Ok(Screened { target, raw, credential })
}

impl Screened<'_> {
    pub(super) fn key(&self) -> &str { &self.credential.secret }

    pub(super) fn send(&self, remaining: Duration) -> Result<Process, String> {
        let config = format!(
            "url = {}\nrequest = \"POST\"\nheader = {}\nheader = \"Content-Type: application/json\"\ndata = {}\n",
            serde_json::to_string(&self.target.url).unwrap(),
            serde_json::to_string(&format!("Authorization: Bearer {}", self.key())).unwrap(),
            serde_json::to_string(std::str::from_utf8(self.raw).map_err(|_| "request UTF-8")?).unwrap()
        );
        let mut command = Command::new("/usr/bin/curl");
        command.env_clear().args([
            "-q", "--silent", "--show-error", "--noproxy", "*", "--max-redirs", "0",
            "--proto", self.target.protocol, "--max-time", &format!("{:.3}", remaining.as_secs_f64()),
            "--write-out", "\n%{http_code}", "--config", "-",
        ]);
        crate::process::run_bounded(command, config.into_bytes(), remaining, MAX_RESPONSE + 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_screened_dispatch_handle_after_explicit_local_refusal() {
        let target = Target { name: "typesafe", url: "https://api.typesafe.ai/v1/systemone".into(), protocol: "=https" };
        let request = serde_json::json!({"safe": "synthetic"});
        let result = prepare(&["--local".into()], &target, &request, b"{}", Instant::now() + Duration::from_secs(1));
        assert!(matches!(result, Err(PrepareError::Credential(CredentialError::LocalDisabled))));
    }
    #[test]
    fn secret_shaped_body_never_yields_a_dispatch_handle() {
        let target = Target { name: "synthetic-loopback-v1", url: "http://127.0.0.1:1024/v1/systemone".into(), protocol: "=http" };
        let raw = br#"{"material":"-----BEGIN RSA PRIVATE KEY-----"}"#;
        let request = serde_json::json!({"material":"synthetic"});
        let result = prepare(&[], &target, &request, raw, Instant::now() + Duration::from_secs(5));
        assert!(matches!(result, Err(PrepareError::Disclosure(_))));
    }
}
