use std::path::{Component, Path};

pub(crate) fn claim_artifact_path(
    root: &Path,
    path: &Path,
    label: &str,
) -> Result<std::path::PathBuf, String> {
    if path.is_absolute() {
        return Err(format!(
            "{}: {label} must be a root-relative claim artifact path; absolute outputs are external debug only and cannot support claims",
            path.display()
        ));
    }
    if path.components().any(|part| {
        matches!(
            part,
            Component::CurDir | Component::ParentDir | Component::Prefix(_) | Component::RootDir
        )
    }) {
        return Err(format!(
            "{}: {label} must stay inside the package root",
            path.display()
        ));
    }
    if !governed_claim_artifact_root(path) {
        return Err(format!(
            "{}: {label} must use a governed claim artifact root such as validation_artifacts/...; arbitrary root-relative outputs are external debug only and cannot support claims",
            path.display()
        ));
    }
    Ok(root.join(path))
}

fn governed_claim_artifact_root(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(Component::Normal(name)) if name == "validation_artifacts"
    )
}

pub(crate) fn literal_claim_artifact_path(
    root: &Path,
    path: &'static str,
    label: &str,
) -> std::path::PathBuf {
    match claim_artifact_path(root, Path::new(path), label) {
        Ok(path) => path,
        Err(err) => panic!("invalid literal claim artifact path: {err}"),
    }
}
