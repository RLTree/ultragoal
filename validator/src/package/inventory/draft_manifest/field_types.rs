use serde::Deserialize;

const MAX_TEXT_BYTES: usize = 16 * 1024;

macro_rules! identifier_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub(super) struct $name(String);

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                if !valid_identifier(&value) {
                    return Err(serde::de::Error::custom(concat!("invalid ", $label)));
                }
                Ok(Self(value))
            }
        }

        impl $name {
            pub(super) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

macro_rules! text_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(super) struct $name(String);

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                if !valid_text(&value) {
                    return Err(serde::de::Error::custom(concat!("invalid ", $label)));
                }
                Ok(Self(value))
            }
        }

        impl $name {
            pub(super) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identifier_type!(PackageName, "package name");
identifier_type!(SkillId, "skill identifier");
identifier_type!(AgentId, "agent identifier");
identifier_type!(ConnectorId, "connector identifier");
text_type!(PackagePurpose, "package purpose");
text_type!(SkillRole, "skill role");
text_type!(NonGoal, "non-goal");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum DraftStatus {
    Implementation,
    Test,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PackageVersion(String);

impl<'de> Deserialize<'de> for PackageVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if crate::plugin_manifest::Version::parse(&value).is_none() {
            return Err(serde::de::Error::custom("invalid package version"));
        }
        Ok(Self(value))
    }
}

impl PackageVersion {
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct RepoPath(String);

impl<'de> Deserialize<'de> for RepoPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if super::super::package_path_syntax_error(&value).is_some() {
            return Err(serde::de::Error::custom("invalid repository path"));
        }
        Ok(Self(value))
    }
}

impl RepoPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_TEXT_BYTES && !value.contains('\0')
}
