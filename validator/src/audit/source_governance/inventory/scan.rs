use super::{GovernedInventory, GovernedSource};
use crate::audit::source_governance::generated::authority_file::AuthorityRoot;
use crate::audit::source_governance::scope::{
    DIRECTORY_ROOTS, EXACT_FILES, FileRule, SourceClass, governed_file,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) fn once(root: &Path) -> (GovernedInventory, Vec<String>) {
    let mut sources = Vec::new();
    let mut failures = Vec::new();
    let authority = match AuthorityRoot::open(root) {
        Ok(value) => value,
        Err(error) => {
            failures.push(format!(
                "governed_source_authority_unavailable:{}",
                error.stable_text()
            ));
            return (empty_inventory(), failures);
        }
    };
    for exact in EXACT_FILES {
        failures.extend(exact_file_ambiguities(root, exact.relative));
        if !exact.required && missing(&root.join(exact.relative)) {
            continue;
        }
        read_exact(
            &authority,
            exact.relative,
            exact.class,
            &mut sources,
            &mut failures,
        );
    }
    for scan in DIRECTORY_ROOTS {
        let directory = root.join(scan.relative);
        if !scan.required && missing(&directory) {
            continue;
        }
        walk(
            root,
            &authority,
            &directory,
            scan.class,
            scan.rule,
            &mut sources,
            &mut failures,
        );
    }
    sources.sort_by(|left, right| left.relative.cmp(&right.relative));
    reject_ambiguous_paths(&sources, &mut failures);
    failures.sort();
    failures.dedup();
    (
        GovernedInventory {
            sources,
            generated_projections: std::collections::BTreeSet::new(),
        },
        failures,
    )
}

fn exact_file_ambiguities(root: &Path, relative: &str) -> Vec<String> {
    let path = Path::new(relative);
    let Some(expected) = path.file_name().and_then(|value| value.to_str()) else {
        return vec![format!(
            "governed_source_exact_file_path_invalid:{relative}"
        )];
    };
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let Ok(entries) = fs::read_dir(root.join(parent)) else {
        return Vec::new();
    };
    let mut failures = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|observed| observed != expected && observed.eq_ignore_ascii_case(expected))
        .map(|observed| {
            let observed = parent.join(observed).to_string_lossy().replace('\\', "/");
            format!("governed_source_exact_file_ambiguous:{relative}:{observed}")
        })
        .collect::<Vec<_>>();
    failures.sort();
    failures
}

fn empty_inventory() -> GovernedInventory {
    GovernedInventory {
        sources: Vec::new(),
        generated_projections: std::collections::BTreeSet::new(),
    }
}

fn missing(path: &Path) -> bool {
    matches!(fs::symlink_metadata(path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
}

fn walk(
    root: &Path,
    authority: &AuthorityRoot,
    directory: &Path,
    class: SourceClass,
    rule: FileRule,
    sources: &mut Vec<GovernedSource>,
    failures: &mut Vec<String>,
) {
    let relative = relative(root, directory).unwrap_or_else(|| directory.display().to_string());
    let metadata = match fs::symlink_metadata(directory) {
        Ok(value) => value,
        Err(error) => {
            failures.push(format!(
                "governed_source_root_unreadable:{relative}:{error}"
            ));
            return;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        failures.push(format!("governed_source_root_not_directory:{relative}"));
        return;
    }
    let entries = match fs::read_dir(directory) {
        Ok(values) => values,
        Err(error) => {
            failures.push(format!(
                "governed_source_directory_unreadable:{relative}:{error}"
            ));
            return;
        }
    };
    let mut paths = entries
        .filter_map(|entry| match entry {
            Ok(value) => Some(value.path()),
            Err(error) => {
                failures.push(format!(
                    "governed_source_entry_unreadable:{relative}:{error}"
                ));
                None
            }
        })
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        visit(root, authority, &path, class, rule, sources, failures);
    }
}

fn visit(
    root: &Path,
    authority: &AuthorityRoot,
    path: &Path,
    class: SourceClass,
    rule: FileRule,
    sources: &mut Vec<GovernedSource>,
    failures: &mut Vec<String>,
) {
    let Some(relative) = relative(root, path) else {
        failures.push(format!(
            "governed_source_path_outside_root:{}",
            path.display()
        ));
        return;
    };
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) => {
            failures.push(format!(
                "governed_source_metadata_unreadable:{relative}:{error}"
            ));
            return;
        }
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        failures.push(format!("governed_source_symlink_rejected:{relative}"));
    } else if file_type.is_dir() {
        walk(root, authority, path, class, rule, sources, failures);
    } else if !file_type.is_file() {
        failures.push(format!("governed_source_special_file_rejected:{relative}"));
    } else if governed_file(&relative, rule) {
        read_exact(authority, &relative, class, sources, failures);
    }
}

fn read_exact(
    authority: &AuthorityRoot,
    relative: &str,
    class: SourceClass,
    sources: &mut Vec<GovernedSource>,
    failures: &mut Vec<String>,
) {
    match authority.capture(Path::new(relative)) {
        Ok(captured) => sources.push(GovernedSource {
            relative: relative.to_string(),
            class,
            authority: captured.baseline,
            bytes: captured.bytes,
        }),
        Err(error) => failures.push(format!(
            "governed_source_file_unreadable:{relative}:{}",
            error.stable_text()
        )),
    }
}

fn reject_ambiguous_paths(sources: &[GovernedSource], failures: &mut Vec<String>) {
    let mut folded = BTreeMap::new();
    for source in sources {
        let key = source.relative.to_ascii_lowercase();
        if let Some(prior) = folded.insert(key, source.relative.as_str()) {
            failures.push(format!(
                "governed_source_ambiguous_path:{prior}:{}",
                source.relative
            ));
        }
    }
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()?
        .to_str()
        .map(|value| value.replace('\\', "/"))
}
