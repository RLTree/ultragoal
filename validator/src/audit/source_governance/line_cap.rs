use super::{GovernedInventory, GovernedSource};

pub(crate) const MAX_AUTHORED_LINES: usize = 250;

pub(crate) fn failures(inventory: &GovernedInventory) -> Vec<String> {
    inventory
        .sources
        .iter()
        .filter(|source| !inventory.generated_projections.contains(&source.relative))
        .filter_map(failure)
        .collect()
}

pub(crate) fn failure(source: &GovernedSource) -> Option<String> {
    let lines = if source.bytes.is_empty() {
        0
    } else {
        source.bytes.iter().filter(|byte| **byte == b'\n').count()
            + usize::from(source.bytes.last() != Some(&b'\n'))
    };
    (lines > MAX_AUTHORED_LINES).then(|| {
        format!(
            "plugin_self_law_line_cap_exceeded:{}:{lines}:class={}",
            source.relative,
            source.class.id()
        )
    })
}
