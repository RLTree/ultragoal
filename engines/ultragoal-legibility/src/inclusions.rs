use crate::{inventory::Inventory, syntax::Report};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

pub struct Inclusions {
    pub bindings: Vec<(String, String, String)>,
    pub failures: Vec<String>,
}

pub fn check(root: &Path, reports: &BTreeMap<String, Report>, inventory: &Inventory) -> Inclusions {
    let mut output = Inclusions {
        bindings: vec![],
        failures: vec![],
    };
    for (source, report) in reports {
        for (kind, operand) in &report.includes {
            let requested = root.join(source).parent().unwrap().join(operand);
            let mut resolved = PathBuf::new();
            for component in requested.components() {
                match component {
                    Component::ParentDir => {
                        resolved.pop();
                    }
                    Component::CurDir => {}
                    _ => resolved.push(component),
                }
            }
            let Ok(relative) = resolved.strip_prefix(root) else {
                output
                    .failures
                    .push(format!("include_data_outside_project:{source}:{operand}"));
                continue;
            };
            let relative = relative.to_string_lossy().into_owned();
            let class = if inventory.files.contains_key(&relative) {
                "governed_source"
            } else {
                output
                    .failures
                    .push(format!("include_data_ungoverned:{source}:{relative}"));
                continue;
            };
            let mut current = root.to_path_buf();
            let mut safe = true;
            for part in Path::new(&relative).components() {
                current.push(part);
                match current.symlink_metadata() {
                    Ok(metadata) if !metadata.file_type().is_symlink() => {}
                    _ => {
                        safe = false;
                        break;
                    }
                }
            }
            if !safe || !resolved.is_file() {
                output.failures.push(format!(
                    "include_data_missing_or_linked:{source}:{relative}"
                ));
                continue;
            }
            output
                .bindings
                .push((source.clone(), relative, format!("{kind}:{class}")));
        }
    }
    output
}
