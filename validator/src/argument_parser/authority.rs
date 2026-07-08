use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliRoot {
    path: PathBuf,
}

impl CliRoot {
    pub(super) fn workspace_default() -> Self {
        Self {
            path: PathBuf::from("."),
        }
    }

    pub(super) fn from_option_value(raw: &str) -> Result<Self, String> {
        typed_path(raw, "workspace root").map(|path| Self { path })
    }

    pub(super) fn into_path_buf(self) -> PathBuf {
        self.path
    }

    pub(super) fn as_path(&self) -> &std::path::Path {
        &self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliArtifactPath {
    path: PathBuf,
    product_role: &'static str,
}

impl CliArtifactPath {
    pub(super) fn from_option_value(raw: &str, product_role: &'static str) -> Result<Self, String> {
        typed_path(raw, product_role).map(|path| Self { path, product_role })
    }

    pub(super) fn into_path_buf(self) -> PathBuf {
        self.path
    }

    #[cfg(test)]
    pub(super) fn product_role(&self) -> &'static str {
        self.product_role
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliText {
    value: String,
    product_role: &'static str,
}

impl CliText {
    pub(super) fn from_option_value(raw: &str, product_role: &'static str) -> Result<Self, String> {
        if raw.trim().is_empty() {
            return Err(format!("{product_role} cannot be empty"));
        }
        Ok(Self {
            value: raw.to_string(),
            product_role,
        })
    }

    pub(super) fn into_string(self) -> String {
        self.value
    }

    #[cfg(test)]
    pub(super) fn product_role(&self) -> &'static str {
        self.product_role
    }
}

pub(super) fn required_artifact_path(
    args: &[String],
    key: &str,
    product_role: &'static str,
) -> Result<CliArtifactPath, String> {
    optional_artifact_path(args, key, product_role)?
        .ok_or_else(|| format!("missing required argument {key}"))
}

pub(super) fn optional_artifact_path(
    args: &[String],
    key: &str,
    product_role: &'static str,
) -> Result<Option<CliArtifactPath>, String> {
    optional_text(args, key, product_role)?
        .map(|text| CliArtifactPath::from_option_value(&text.value, product_role))
        .transpose()
}

pub(super) fn optional_text(
    args: &[String],
    key: &str,
    product_role: &'static str,
) -> Result<Option<CliText>, String> {
    Ok(option_value(args, key)
        .map(|raw| CliText::from_option_value(raw, product_role))
        .transpose()?)
}

pub(super) fn option_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == key)
        .map(|window| window[1].as_str())
}

fn typed_path(raw: &str, product_role: &str) -> Result<PathBuf, String> {
    if raw.trim().is_empty() {
        return Err(format!("{product_role} path cannot be empty"));
    }
    Ok(PathBuf::from(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_cli_authority_rejects_empty_paths_and_text() {
        assert!(CliRoot::from_option_value("").is_err());
        assert!(CliArtifactPath::from_option_value(" ", "claim receipt").is_err());
        assert!(CliText::from_option_value("", "source audit mode").is_err());
    }

    #[test]
    fn typed_cli_authority_records_product_roles() {
        let receipt = CliArtifactPath::from_option_value(
            "validation_artifacts/a.json",
            "source audit receipt",
        )
        .expect("receipt path");
        assert_eq!(receipt.product_role(), "source audit receipt");
        let mode =
            CliText::from_option_value("strict_fixtures", "source audit mode").expect("mode text");
        assert_eq!(mode.product_role(), "source audit mode");
    }
}
