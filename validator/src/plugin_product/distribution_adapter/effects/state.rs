pub(super) struct Effects {
    root: ConfinedRoot,
    package: BoundPackage,
    before: LifecycleState,
    expected_after: LifecycleState,
    effects: Vec<LifecycleEffect>,
    next_effect: usize,
    logical: LifecycleState,
    mutations: Vec<Mutation>,
    execution_attempted: bool,
    restored: bool,
}
