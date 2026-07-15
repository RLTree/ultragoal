use super::*;
#[cfg(test)]
use std::cell::Cell;

pub(crate) fn incomplete_node(
    token: &RoutineMediatedIntent,
    disposition: RoutineNodeDisposition,
    failure_code: impl Into<String>,
) -> RoutineNodeMediation {
    RoutineNodeMediation {
        intent_id: token.intent().intent_id().to_owned(),
        node_id: token.intent().node_id().to_owned(),
        plan_order: token.intent().plan_order(),
        disposition,
        result_artifact_sha256: None,
        failure_code: Some(failure_code.into()),
    }
}

pub(super) fn collect_generated_witnesses(
    values: Vec<(String, Vec<u8>)>,
    attempt: &AttemptReservation,
) -> BTreeMap<String, String> {
    let mut authenticated = BTreeMap::new();
    for (digest, bytes) in values {
        if let Ok(wire) = serde_json::from_slice::<ReuseArtifactWire>(&bytes) {
            authenticated.insert(digest, wire.mediator_witness_sha256);
        }
    }
    attempt.retain_non_durable_authentication(&authenticated);
    authenticated
}

pub(crate) fn recovery_identity(grant_id: &str, protocol_id: &str, request_id: &str) -> String {
    framed(&[
        RECOVERY_DOMAIN,
        grant_id.as_bytes(),
        protocol_id.as_bytes(),
        request_id.as_bytes(),
    ])
}

pub(crate) fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

pub(crate) fn concurrent(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::ConcurrentMutation, cause, None)
}

#[cfg(test)]
pub(crate) type TestHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
pub(crate) fn test_hook() -> &'static Mutex<Option<TestHook>> {
    static HOOK: OnceLock<Mutex<Option<TestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn post_spawn_test_hook() -> &'static Mutex<Option<TestHook>> {
    static HOOK: OnceLock<Mutex<Option<TestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
thread_local! {
    // This injection is scoped to the calling test thread. A process-global
    // flag can be consumed by an unrelated concurrent mediation and turn a
    // deterministic reconciliation test into a race.
    static FINISH_FAILURE_TEST_HOOK: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn set_test_mediator_pre_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_mediator_post_spawn_hook(hook: impl FnOnce() + Send + 'static) {
    *post_spawn_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_mediator_finish_failure() {
    FINISH_FAILURE_TEST_HOOK.with(|hook| hook.set(true));
}

#[cfg(test)]
pub(crate) fn run_test_pre_spawn_hook() {
    if let Some(hook) = test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
pub(crate) fn run_test_post_spawn_hook() {
    if let Some(hook) = post_spawn_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
pub(crate) fn run_test_finish_failure_hook(authority: &RoutineMediationAuthority) {
    let force = FINISH_FAILURE_TEST_HOOK.with(|hook| hook.replace(false));
    if force {
        authority
            .finish()
            .expect("test finish-failure hook requires complete authority");
    }
}

#[cfg(not(test))]
pub(crate) fn run_test_pre_spawn_hook() {}

#[cfg(not(test))]
pub(crate) fn run_test_post_spawn_hook() {}

#[cfg(not(test))]
pub(crate) fn run_test_finish_failure_hook(_authority: &RoutineMediationAuthority) {}

#[cfg(test)]
pub(crate) use filesystem::{set_test_output_capture_hook, set_test_read_source_capture_hook};
