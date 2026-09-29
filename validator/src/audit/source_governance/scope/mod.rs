#[cfg(test)]
mod exclusions;
mod model;
mod roots;

#[cfg(test)]
pub(crate) use exclusions::EXCLUSIONS;
pub(crate) use model::SourceClass;
pub(super) use model::{ExactFile, FileRule, ScanRoot};
pub(super) use roots::{DIRECTORY_ROOTS, EXACT_FILES};
use std::path::Path;

pub(super) fn governed_file(relative: &str, rule: FileRule) -> bool {
    match rule {
        FileRule::AllRegular => true,
        FileRule::Rust => relative.ends_with(".rs"),
        FileRule::ActiveDocumentation => active_documentation(relative),
    }
}

fn active_documentation(relative: &str) -> bool {
    let path = Path::new(relative);
    let direct = path.parent() == Some(Path::new("docs"));
    direct
        || relative.starts_with("docs/exec-plans/active/")
        || relative.starts_with("docs/generated/")
        || relative.starts_with("docs/improvement-loop/")
        || active_successor_document(relative)
}

fn active_successor_document(relative: &str) -> bool {
    let Some(rest) = relative.strip_prefix("docs/ultragoal-successor-live/") else {
        return false;
    };
    !rest.starts_with("acceptance/")
        && !rest.starts_with("reviews/")
        && !rest.starts_with("worker-results/")
}
