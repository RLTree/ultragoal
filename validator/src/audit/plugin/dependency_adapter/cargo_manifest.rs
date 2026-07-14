use super::model::DirectDependency;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CargoDependencyDocument {
    package: CargoPackage,
    dependencies: BTreeMap<String, DependencyDeclaration>,
    #[serde(rename = "bin", default)]
    binaries: Vec<CargoBinary>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CargoPackage {
    name: String,
    version: String,
    edition: String,
    license: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DependencyDeclaration {
    Version(String),
    Detailed(DependencyDetail),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DependencyDetail {
    version: Option<String>,
    #[serde(rename = "default-features")]
    default_features: Option<bool>,
    features: Option<Vec<String>>,
    optional: Option<bool>,
    package: Option<String>,
    path: Option<String>,
    git: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CargoBinary {
    name: String,
    path: String,
}

pub(super) fn load(path: &Path) -> Result<Vec<DirectDependency>, String> {
    let bytes = crate::digest::read_file_bytes(path)
        .map_err(|error| format!("dependency_adapter_cargo_unreadable:{error}"))?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("dependency_adapter_cargo_non_utf8:{error}"))?;
    let document: CargoDependencyDocument = toml::from_str(text)
        .map_err(|error| format!("dependency_adapter_cargo_invalid:{error}"))?;
    let _manifest_identity = (
        document.package.name,
        document.package.version,
        document.package.edition,
        document.package.license,
        document
            .binaries
            .into_iter()
            .map(|binary| (binary.name, binary.path))
            .collect::<Vec<_>>(),
    );
    let mut dependencies = document
        .dependencies
        .into_iter()
        .map(|(crate_name, declaration)| {
            let version_requirement = match declaration {
                DependencyDeclaration::Version(version) => version,
                DependencyDeclaration::Detailed(detail) => {
                    let _source_shape = (
                        detail.default_features,
                        detail.features,
                        detail.optional,
                        detail.package,
                        detail.path,
                        detail.git,
                        detail.branch,
                        detail.tag,
                        detail.rev,
                    );
                    detail
                        .version
                        .unwrap_or_else(|| "non_registry_version_source".to_string())
                }
            };
            DirectDependency {
                crate_name,
                version_requirement,
            }
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| left.crate_name.cmp(&right.crate_name));
    Ok(dependencies)
}
