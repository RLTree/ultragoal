use super::*;

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
