pub(crate) fn active_api_identifiers() -> BTreeSet<&'static str> {
    BINDINGS
        .iter()
        .flat_map(|binding| binding.apis)
        .copied()
        .collect()
}
