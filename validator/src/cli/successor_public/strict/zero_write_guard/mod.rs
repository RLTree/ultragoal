mod recursive_snapshot;
mod scope_policy;

pub(super) use recursive_snapshot::capture;

#[cfg(test)]
mod tests;
