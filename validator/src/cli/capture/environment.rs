use super::inputs::{ArgumentInput, ArtifactExpectation, EnvironmentInput};
use super::spec::CommandSpec;
use super::util::{bytes_hex, contains, os_bytes};
use crate::context::LiveContext;
use serde::Serialize;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};

const MIN_SECRET_BYTES: usize = 4;
const MAX_SECRET_BYTES: usize = 4096;
const MAX_ENVIRONMENT_ENTRIES: usize = 1024;
const MAX_ENVIRONMENT_NAME_BYTES: usize = 256;
const MAX_PUBLIC_VALUE_BYTES: usize = 64 * 1024;
const MAX_ENVIRONMENT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
pub(super) struct ArgumentRecord {
    channel: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_version: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct EnvironmentRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    name_hex: Option<String>,
    channel: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secret_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_version: Option<String>,
}

pub(super) struct PreparedEnvironment {
    pub records: Vec<EnvironmentRecord>,
    pub argument_records: Vec<ArgumentRecord>,
    pub arguments: Vec<OsString>,
    pub values: Vec<(OsString, OsString)>,
    pub sensitivity: InvocationSensitivity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InvocationSensitivity {
    Public,
    SecretBearing,
}

impl InvocationSensitivity {
    pub(super) fn from_bound_secrets(secrets: &[Vec<u8>]) -> Self {
        if secrets.iter().any(|secret| !secret.is_empty()) {
            Self::SecretBearing
        } else {
            Self::Public
        }
    }

    pub(super) const fn is_secret_bearing(self) -> bool {
        matches!(self, Self::SecretBearing)
    }
}

fn validate_name(name: &OsStr) -> Result<Vec<u8>, String> {
    let bytes = os_bytes(name);
    if bytes.is_empty()
        || bytes.len() > MAX_ENVIRONMENT_NAME_BYTES
        || bytes.contains(&0)
        || bytes.contains(&b'=')
    {
        return Err("environment name is empty or contains a forbidden byte".to_owned());
    }
    Ok(bytes.to_vec())
}

fn secret_version(context: &LiveContext, source: &str) -> Result<String, String> {
    context
        .configuration()
        .secret_sources
        .iter()
        .find(|identity| identity.name == source)
        .map(|identity| identity.public_version.clone())
        .ok_or_else(|| "secret input source is not context-bound".to_owned())
}

fn validate_secret(value: &OsStr) -> Result<Vec<u8>, String> {
    let bytes = os_bytes(value);
    if !(MIN_SECRET_BYTES..=MAX_SECRET_BYTES).contains(&bytes.len()) || bytes.contains(&0) {
        return Err("secret input has an unsafe length or NUL".to_owned());
    }
    Ok(bytes.to_vec())
}

fn collect_secrets(spec: &CommandSpec, context: &LiveContext) -> Result<Vec<Vec<u8>>, String> {
    let mut secrets = Vec::new();
    for input in &spec.arguments {
        if let ArgumentInput::Secret(item) = input {
            secret_version(context, &item.source)?;
            secrets.push(validate_secret(&item.value)?);
        }
    }
    for input in &spec.environment {
        if let EnvironmentInput::Secret(item) = input {
            secret_version(context, &item.source)?;
            secrets.push(validate_secret(&item.value)?);
        }
    }
    secrets.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    secrets.dedup();
    Ok(secrets)
}

pub(super) fn prepare(
    spec: &CommandSpec,
    context: &LiveContext,
) -> Result<PreparedEnvironment, String> {
    if spec.environment.len() > MAX_ENVIRONMENT_ENTRIES {
        return Err("environment allowlist exceeds entry bound".to_owned());
    }
    let secrets = collect_secrets(spec, context)?;
    let sensitivity = InvocationSensitivity::from_bound_secrets(&secrets);
    reject_secret_in_public_fields(spec, context, &secrets)?;
    let mut argument_records = Vec::with_capacity(spec.arguments.len());
    for input in &spec.arguments {
        if sensitivity.is_secret_bearing() {
            argument_records.push(ArgumentRecord {
                channel: "withheld-secret-bearing-invocation",
                value_hex: None,
                secret_source: None,
                public_version: None,
            });
        } else {
            match input {
                ArgumentInput::Public(item) => {
                    argument_records.push(ArgumentRecord {
                        channel: "public",
                        value_hex: Some(bytes_hex(os_bytes(&item.value))),
                        secret_source: None,
                        public_version: None,
                    });
                }
                ArgumentInput::Secret(_) => unreachable!("secret input establishes sensitivity"),
            }
        }
    }
    let records = prepare_environment(spec, context, sensitivity)?;
    let arguments = spec
        .arguments
        .iter()
        .map(|input| match input {
            ArgumentInput::Public(item) => item.value.clone(),
            ArgumentInput::Secret(item) => item.value.clone(),
        })
        .collect();
    let values = spec
        .environment
        .iter()
        .map(|input| match input {
            EnvironmentInput::Public(item) => (item.name.clone(), item.value.clone()),
            EnvironmentInput::Secret(item) => (item.name.clone(), item.value.clone()),
        })
        .collect();
    Ok(PreparedEnvironment {
        records,
        argument_records,
        arguments,
        values,
        sensitivity,
    })
}

fn prepare_environment(
    spec: &CommandSpec,
    context: &LiveContext,
    sensitivity: InvocationSensitivity,
) -> Result<Vec<EnvironmentRecord>, String> {
    let mut records = Vec::with_capacity(spec.environment.len());
    let mut names = BTreeSet::new();
    let mut total_bytes = 0_usize;
    for input in &spec.environment {
        let (name, value, source) = match input {
            EnvironmentInput::Public(item) => (&item.name, &item.value, None),
            EnvironmentInput::Secret(item) => (&item.name, &item.value, Some(&item.source)),
        };
        let name_bytes = validate_name(name)?;
        if !names.insert(name_bytes.clone()) {
            return Err("duplicate environment name".to_owned());
        }
        if source.is_none()
            && (os_bytes(value).contains(&0) || os_bytes(value).len() > MAX_PUBLIC_VALUE_BYTES)
        {
            return Err("public environment value has a forbidden byte or size".to_owned());
        }
        total_bytes = total_bytes
            .saturating_add(name_bytes.len())
            .saturating_add(os_bytes(value).len());
        if sensitivity.is_secret_bearing() {
            records.push(EnvironmentRecord {
                name_hex: None,
                channel: "withheld-secret-bearing-invocation",
                value_hex: None,
                secret_source: None,
                public_version: None,
            });
        } else {
            let version = source
                .map(|source| secret_version(context, source))
                .transpose()?;
            records.push(EnvironmentRecord {
                name_hex: Some(bytes_hex(&name_bytes)),
                channel: if source.is_some() {
                    "secret-source"
                } else {
                    "public"
                },
                value_hex: source.is_none().then(|| bytes_hex(os_bytes(value))),
                secret_source: source.cloned(),
                public_version: version,
            });
        }
    }
    if total_bytes > MAX_ENVIRONMENT_BYTES {
        return Err("environment allowlist exceeds byte bound".to_owned());
    }
    if !sensitivity.is_secret_bearing() {
        records.sort_by(|left, right| left.name_hex.cmp(&right.name_hex));
    }
    Ok(records)
}

fn reject_secret_in_public_fields(
    spec: &CommandSpec,
    context: &LiveContext,
    secrets: &[Vec<u8>],
) -> Result<(), String> {
    let public = std::iter::once(os_bytes(spec.program.as_os_str()))
        .chain(std::iter::once(os_bytes(spec.cwd.as_os_str())))
        .chain(spec.arguments.iter().filter_map(|input| match input {
            ArgumentInput::Public(item) => Some(os_bytes(&item.value)),
            ArgumentInput::Secret(_) => None,
        }))
        .chain(spec.environment.iter().flat_map(|input| match input {
            EnvironmentInput::Public(item) => [os_bytes(&item.name), os_bytes(&item.value)],
            EnvironmentInput::Secret(item) => [os_bytes(&item.name), item.source.as_bytes()],
        }))
        .chain(
            spec.artifacts
                .iter()
                .flat_map(|input| match input {
                    ArtifactExpectation::Public(item) => [
                        Some(os_bytes(item.path.as_os_str())),
                        item.expected_sha256.as_deref().map(str::as_bytes),
                    ],
                    ArtifactExpectation::Secret(_) => [None, None],
                })
                .flatten(),
        )
        .chain(
            context
                .configuration()
                .secret_sources
                .iter()
                .flat_map(|item| [item.name.as_bytes(), item.public_version.as_bytes()]),
        );
    if public
        .into_iter()
        .any(|field| secrets.iter().any(|secret| contains(field, secret)))
    {
        return Err("secret value appeared outside an explicit secret channel".to_owned());
    }
    Ok(())
}
