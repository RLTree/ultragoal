#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EffectPoint {
    OpenDirectory,
    OpenFile,
    CreateFile,
    Mkdir,
    Rename,
    Quarantine,
    Unlink,
}

#[cfg(test)]
thread_local! {
    static HOOK: std::cell::RefCell<Option<(EffectPoint, Option<String>, Box<dyn FnOnce(&str)>)>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_test_effect_hook(point: EffectPoint, hook: impl FnOnce(&str) + 'static) {
    HOOK.with(|slot| {
        assert!(
            slot.borrow_mut()
                .replace((point, None, Box::new(hook)))
                .is_none(),
            "only one descriptor-effect hook may be armed per thread",
        );
    });
}

#[cfg(test)]
pub(crate) fn set_test_effect_hook_matching(
    point: EffectPoint,
    detail_fragment: &str,
    hook: impl FnOnce(&str) + 'static,
) {
    HOOK.with(|slot| {
        assert!(
            slot.borrow_mut()
                .replace((point, Some(detail_fragment.to_owned()), Box::new(hook),))
                .is_none(),
            "only one descriptor-effect hook may be armed per thread",
        );
    });
}

#[cfg(test)]
pub(crate) fn assert_test_effect_hook_consumed() {
    HOOK.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "descriptor-effect hook was not reached"
        )
    });
}

#[cfg(test)]
pub(super) fn before(point: EffectPoint, detail: &str) {
    let hook = HOOK.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|(expected, matcher, _)| {
            *expected == point
                && matcher
                    .as_ref()
                    .is_none_or(|fragment| detail.contains(fragment))
        }) {
            slot.take().map(|(_, _, hook)| hook)
        } else {
            None
        }
    });
    if let Some(hook) = hook {
        hook(detail);
    }
}

#[cfg(not(test))]
pub(super) fn before(_point: EffectPoint, _detail: &str) {}
