mod recursive_snapshot;
mod scope_policy;

pub(crate) use recursive_snapshot::capture;

#[cfg(test)]
mod tests;
