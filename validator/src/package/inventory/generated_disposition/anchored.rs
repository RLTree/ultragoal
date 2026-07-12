#[cfg(all(test, unix))]
pub(super) fn set_before_component_open(action: impl FnOnce() + 'static) {
    super::super::anchored::test_hooks::set_next_component(action);
}
