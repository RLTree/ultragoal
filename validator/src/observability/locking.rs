use std::fs::{File, TryLockError as FileTryLockError};
use std::sync::{Mutex, MutexGuard, TryLockError as MutexTryLockError};
use std::time::{Duration, Instant};

pub(super) const STORE_LOCK_TIMEOUT: Duration = Duration::from_millis(1_000);
const INITIAL_BACKOFF: Duration = Duration::from_millis(1);
const MAX_BACKOFF: Duration = Duration::from_millis(20);
pub(super) const LOCK_TIMEOUT_ERROR: &str = "observe-store-lock-timeout";

#[derive(Clone, Copy, Debug)]
pub(super) struct LockDeadline {
    expires_at: Instant,
}

impl LockDeadline {
    pub(super) fn for_store_operation() -> Result<Self, String> {
        Instant::now()
            .checked_add(STORE_LOCK_TIMEOUT)
            .map(|expires_at| Self { expires_at })
            .ok_or_else(lock_failed)
    }

    fn remaining(self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }

    fn is_expired(self) -> bool {
        Instant::now() >= self.expires_at
    }
}

#[derive(Clone, Copy)]
enum LockKind {
    Shared,
    Exclusive,
}

pub(super) fn lock_identity<'a, T>(
    mutex: &'a Mutex<T>,
    deadline: &LockDeadline,
) -> Result<MutexGuard<'a, T>, String> {
    let mut backoff = INITIAL_BACKOFF;

    loop {
        if deadline.is_expired() {
            return Err(lock_timeout());
        }
        match mutex.try_lock() {
            Ok(guard) if !deadline.is_expired() => return Ok(guard),
            Ok(guard) => {
                drop(guard);
                return Err(lock_timeout());
            }
            Err(MutexTryLockError::WouldBlock) => {
                backoff = sleep_within_deadline(*deadline, backoff)?;
            }
            Err(MutexTryLockError::Poisoned(_)) => return Err(lock_failed()),
        }
    }
}

pub(super) fn lock_shared(file: &File, deadline: &LockDeadline) -> Result<(), String> {
    lock_file_with_deadline(file, LockKind::Shared, deadline)
}

pub(super) fn lock_exclusive(file: &File, deadline: &LockDeadline) -> Result<(), String> {
    lock_file_with_deadline(file, LockKind::Exclusive, deadline)
}

fn lock_file_with_deadline(
    file: &File,
    kind: LockKind,
    deadline: &LockDeadline,
) -> Result<(), String> {
    let mut backoff = INITIAL_BACKOFF;

    loop {
        if deadline.is_expired() {
            return Err(lock_timeout());
        }
        let attempt = match kind {
            LockKind::Shared => file.try_lock_shared(),
            LockKind::Exclusive => file.try_lock(),
        };
        match attempt {
            Ok(()) if !deadline.is_expired() => return Ok(()),
            Ok(()) => {
                let _ = file.unlock();
                return Err(lock_timeout());
            }
            Err(FileTryLockError::WouldBlock) => {
                backoff = sleep_within_deadline(*deadline, backoff)?;
            }
            Err(FileTryLockError::Error(_)) => return Err(lock_failed()),
        }
    }
}

fn sleep_within_deadline(deadline: LockDeadline, backoff: Duration) -> Result<Duration, String> {
    let remaining = deadline.remaining();
    if remaining.is_zero() {
        return Err(lock_timeout());
    }
    std::thread::sleep(backoff.min(remaining));
    Ok(backoff.saturating_mul(2).min(MAX_BACKOFF))
}

fn lock_timeout() -> String {
    LOCK_TIMEOUT_ERROR.to_owned()
}

fn lock_failed() -> String {
    "observe-store-lock-failed".to_owned()
}
