use std::cell::RefCell;

enum Phase {
    BeforeRoot,
    BeforeComponent,
    AfterRead,
    BeforeFinish,
}

struct Hook {
    phase: Phase,
    relative: Option<String>,
    component: Option<usize>,
    action: Box<dyn FnOnce()>,
}

thread_local! {
    static HOOKS: RefCell<Vec<Hook>> = const { RefCell::new(Vec::new()) };
}

fn push(hook: Hook) {
    HOOKS.with(|hooks| hooks.borrow_mut().push(hook));
}

fn run(phase: Phase, relative: Option<&str>, component: Option<usize>) {
    let hook = HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        let position = hooks.iter().position(|hook| {
            std::mem::discriminant(&hook.phase) == std::mem::discriminant(&phase)
                && hook
                    .relative
                    .as_deref()
                    .is_none_or(|expected| Some(expected) == relative)
                && hook
                    .component
                    .is_none_or(|expected| Some(expected) == component)
        });
        position.map(|position| hooks.remove(position))
    });
    if let Some(hook) = hook {
        (hook.action)();
    }
}

pub(crate) fn set_before_root(action: impl FnOnce() + 'static) {
    push(Hook {
        phase: Phase::BeforeRoot,
        relative: None,
        component: None,
        action: Box::new(action),
    });
}

pub(crate) fn set_before_component(
    relative: &str,
    component: usize,
    action: impl FnOnce() + 'static,
) {
    push(Hook {
        phase: Phase::BeforeComponent,
        relative: Some(relative.to_string()),
        component: Some(component),
        action: Box::new(action),
    });
}

pub(crate) fn set_next_component(action: impl FnOnce() + 'static) {
    push(Hook {
        phase: Phase::BeforeComponent,
        relative: None,
        component: None,
        action: Box::new(action),
    });
}

pub(crate) fn set_after_read(relative: &str, action: impl FnOnce() + 'static) {
    push(Hook {
        phase: Phase::AfterRead,
        relative: Some(relative.to_string()),
        component: None,
        action: Box::new(action),
    });
}

pub(crate) fn set_before_finish(action: impl FnOnce() + 'static) {
    push(Hook {
        phase: Phase::BeforeFinish,
        relative: None,
        component: None,
        action: Box::new(action),
    });
}

pub(super) fn before_root() {
    run(Phase::BeforeRoot, None, None);
}

pub(super) fn before_component(relative: &str, component: usize) {
    run(Phase::BeforeComponent, Some(relative), Some(component));
}

pub(super) fn after_read(relative: &str) {
    run(Phase::AfterRead, Some(relative), None);
}

pub(super) fn before_finish() {
    run(Phase::BeforeFinish, None, None);
}
