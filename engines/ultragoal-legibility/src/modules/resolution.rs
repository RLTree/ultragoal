use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use syn::visit::Visit;

/// A relative path checked for root escape and existing symlink components.
pub(super) struct SafeSourcePath {
    relative: String,
}

impl SafeSourcePath {
    pub(super) fn as_path(&self) -> &Path {
        Path::new(&self.relative)
    }
    fn into_string(self) -> String {
        self.relative
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourcePathError {
    SymlinkForbidden,
    OutsideProject,
    NonUtf8Path,
}

impl std::fmt::Display for SourcePathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SymlinkForbidden => "symlink_forbidden",
            Self::OutsideProject => "outside_project",
            Self::NonUtf8Path => "non_utf8_path",
        })
    }
}

pub(super) fn safe_path(root: &Path, relative: &Path) -> Result<SafeSourcePath, SourcePathError> {
    let mut clean = PathBuf::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => {
                clean.push(value);
                if std::fs::symlink_metadata(root.join(&clean))
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    return Err(SourcePathError::SymlinkForbidden);
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !clean.pop() {
                    return Err(SourcePathError::OutsideProject);
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(SourcePathError::OutsideProject);
            }
        }
    }
    clean
        .to_str()
        .map(|relative| SafeSourcePath {
            relative: relative.into(),
        })
        .ok_or(SourcePathError::NonUtf8Path)
}

pub(super) fn collect(
    root: &Path,
    source: &str,
    file: &syn::File,
    inventory: &BTreeSet<String>,
    entrypoint: bool,
    failures: &mut Vec<String>,
) -> Vec<String> {
    let path = Path::new(source);
    let parent = path.parent().unwrap_or(Path::new(""));
    let module_base = if entrypoint || path.file_name().is_some_and(|name| name == "mod.rs") {
        parent.to_path_buf()
    } else {
        parent.join(path.file_stem().unwrap_or_default())
    };
    if let Err(reason) = super::attributes::path(&file.attrs) {
        failures.push(format!("module_attribute_invalid:{source}:{reason}"));
    }
    let mut collector = Collector {
        root,
        source,
        inventory,
        failures,
        targets: Vec::new(),
        path_base: parent.to_path_buf(),
        module_base,
    };
    collector.visit_file(file);
    collector.targets
}

struct Collector<'a> {
    root: &'a Path,
    source: &'a str,
    inventory: &'a BTreeSet<String>,
    failures: &'a mut Vec<String>,
    targets: Vec<String>,
    path_base: PathBuf,
    module_base: PathBuf,
}

impl Collector<'_> {
    fn fail(&mut self, kind: &str, module: &syn::ItemMod, detail: &str) {
        self.failures.push(format!(
            "module_{kind}:{}:{}:{detail}",
            self.source, module.ident
        ));
    }

    fn normalized(&mut self, module: &syn::ItemMod, path: PathBuf) -> Option<String> {
        match safe_path(self.root, &path) {
            Ok(path) => Some(path.into_string()),
            Err(reason) => {
                self.fail("path_invalid", module, &reason.to_string());
                None
            }
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let override_path = match super::attributes::path(&module.attrs) {
            Ok(value) => value,
            Err(reason) => {
                self.fail("attribute_invalid", module, reason);
                return;
            }
        };
        let ident = module.ident.to_string();
        let name = ident.strip_prefix("r#").unwrap_or(&ident);
        if let Some((_, items)) = &module.content {
            let directory = match override_path {
                Some(path) => self.path_base.join(path),
                None => self.module_base.join(name),
            };
            let Some(directory) = self.normalized(module, directory) else {
                return;
            };
            let old_path = std::mem::replace(&mut self.path_base, PathBuf::from(&directory));
            let old_module = std::mem::replace(&mut self.module_base, PathBuf::from(directory));
            for item in items {
                self.visit_item(item);
            }
            self.path_base = old_path;
            self.module_base = old_module;
            return;
        }
        let candidates = match override_path {
            Some(path) => vec![self.path_base.join(path)],
            None => vec![
                self.module_base.join(format!("{name}.rs")),
                self.module_base.join(name).join("mod.rs"),
            ],
        };
        let mut found = Vec::new();
        for candidate in candidates {
            let Some(path) = self.normalized(module, candidate) else {
                continue;
            };
            if self.root.join(&path).is_file() {
                if self.inventory.contains(&path) {
                    found.push(path);
                } else {
                    self.fail("outside_inventory", module, &path);
                }
            }
        }
        match found.len() {
            0 => self.fail(
                "missing",
                module,
                "all_cfg_branches_require_resolvable_source",
            ),
            1 => self.targets.extend(found),
            _ => self.fail("ambiguous", module, &found.join(",")),
        }
    }
}
