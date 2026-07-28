use crate::observability::EventStore;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cli::successor_public) struct LocalStoreFailure {
    stage: FailureStage,
    class: FailureClass,
}

impl LocalStoreFailure {
    fn classify(stage: FailureStage, error: &str) -> Self {
        let class = if EventStore::is_lock_timeout_error(error) {
            FailureClass::LockTimeout
        } else if error.starts_with("observe-store-path-denied:") {
            FailureClass::PathBoundary
        } else if error.starts_with("observe-store-corrupt:") {
            FailureClass::Corruption
        } else if error.starts_with("observe-binding-") {
            FailureClass::Binding
        } else if error.contains("-limit:") {
            FailureClass::Limit
        } else if error.starts_with("observe-store-") {
            FailureClass::Io
        } else {
            FailureClass::Unavailable
        };
        Self { stage, class }
    }

    pub(super) fn open(error: &str) -> Self {
        Self::classify(FailureStage::Open, error)
    }

    pub(super) fn query(error: &str) -> Self {
        Self::classify(FailureStage::Query, error)
    }

    pub(super) fn explain(error: &str) -> Self {
        Self::classify(FailureStage::Explain, error)
    }

    pub(super) fn append(error: &str) -> Self {
        Self::classify(FailureStage::Append, error)
    }

    pub(super) const fn open_boundary() -> Self {
        Self {
            stage: FailureStage::Open,
            class: FailureClass::PathBoundary,
        }
    }

    pub(super) const fn changed() -> Self {
        Self {
            stage: FailureStage::Revalidate,
            class: FailureClass::Changed,
        }
    }

    pub(in crate::cli::successor_public) const fn binding() -> Self {
        Self {
            stage: FailureStage::Open,
            class: FailureClass::Binding,
        }
    }

    pub(in crate::cli::successor_public) const fn projection() -> Self {
        Self {
            stage: FailureStage::Explain,
            class: FailureClass::Unavailable,
        }
    }

    pub(in crate::cli::successor_public) const fn missing_store() -> Self {
        Self::projection()
    }

    pub(in crate::cli::successor_public) const fn is_lock_timeout(self) -> bool {
        matches!(self.class, FailureClass::LockTimeout)
    }

    pub(in crate::cli::successor_public) const fn is_corruption(self) -> bool {
        matches!(self.class, FailureClass::Corruption)
    }

    pub(in crate::cli::successor_public) const fn stage(self) -> &'static str {
        match self.stage {
            FailureStage::Open => "open",
            FailureStage::Query => "query",
            FailureStage::Explain => "explain",
            FailureStage::Append => "append",
            FailureStage::Revalidate => "revalidate",
        }
    }

    pub(in crate::cli::successor_public) const fn class(self) -> &'static str {
        match self.class {
            FailureClass::LockTimeout => "lock-timeout",
            FailureClass::PathBoundary => "path-boundary",
            FailureClass::Corruption => "corruption",
            FailureClass::Binding => "binding",
            FailureClass::Limit => "resource-limit",
            FailureClass::Io => "io",
            FailureClass::Changed => "concurrent-change",
            FailureClass::Unavailable => "unavailable",
        }
    }

    pub(in crate::cli::successor_public) const fn diagnostic_code(self) -> &'static str {
        match self.class {
            FailureClass::LockTimeout => "observe-local-read:lock-timeout",
            FailureClass::PathBoundary => "observe-local-read:path-boundary",
            FailureClass::Corruption => "observe-local-read:corruption",
            FailureClass::Binding => "observe-local-read:binding-mismatch",
            FailureClass::Limit => "observe-local-read:resource-limit",
            FailureClass::Io => "observe-local-read:io-failure",
            FailureClass::Changed => "observe-local-read:concurrent-change",
            FailureClass::Unavailable => "observe-local-read:unavailable",
        }
    }

    pub(in crate::cli::successor_public) const fn surface(self) -> &'static str {
        match self.stage {
            FailureStage::Open => "HCT-OBSERVE local store open",
            FailureStage::Query => "HCT-OBSERVE local store query",
            FailureStage::Explain => "HCT-OBSERVE local store explanation",
            FailureStage::Append => "HCT-OBSERVE local store append",
            FailureStage::Revalidate => "HCT-OBSERVE local store revalidation",
        }
    }

    pub(in crate::cli::successor_public) const fn summary(self) -> &'static str {
        match self.class {
            FailureClass::LockTimeout => {
                "The local store lock deadline expired before one stable read completed."
            }
            FailureClass::PathBoundary => {
                "The confined local store path did not resolve to one accepted regular file."
            }
            FailureClass::Corruption => {
                "The local store failed bounded row or integrity validation."
            }
            FailureClass::Binding => {
                "The local store binding did not match the current context, candidate, or source."
            }
            FailureClass::Limit => "The local store exceeded a declared read or output bound.",
            FailureClass::Io => "The local store could not complete one confined read operation.",
            FailureClass::Changed => {
                "The local store identity or bytes changed during the read operation."
            }
            FailureClass::Unavailable => "The local store could not be safely interpreted.",
        }
    }

    pub(in crate::cli::successor_public) const fn repair(self) -> &'static str {
        match self.class {
            FailureClass::LockTimeout => {
                "Retry after the current local writer finishes; investigate a holder that exceeds the declared deadline."
            }
            FailureClass::PathBoundary => {
                "Restore a confined single-link regular store and rerun the current-candidate query."
            }
            FailureClass::Corruption => {
                "Preserve the store for review; use only explicit truncated-tail recovery or a separately verified replacement."
            }
            FailureClass::Binding => {
                "Recompute current context and select only the separately candidate-bound local store."
            }
            FailureClass::Limit => {
                "Narrow the local query or reduce retained rows through an explicit lifecycle operation."
            }
            FailureClass::Io => {
                "Repair local permissions or storage availability, then rerun the bounded read."
            }
            FailureClass::Changed => {
                "Wait for concurrent local mutation to finish, rebuild current context, and rerun."
            }
            FailureClass::Unavailable => {
                "Repair the confined local store boundary and rerun the current-candidate read."
            }
        }
    }

    pub(in crate::cli::successor_public) fn value(self) -> Value {
        json!({
            "schema_version": "LocalStoreReadFailure-v1",
            "stage": self.stage(),
            "class": self.class(),
            "diagnostic_code": self.diagnostic_code(),
            "summary": self.summary(),
            "repair": self.repair(),
            "claim_effect": "none"
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureStage {
    Open,
    Query,
    Explain,
    Append,
    Revalidate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureClass {
    LockTimeout,
    PathBoundary,
    Corruption,
    Binding,
    Limit,
    Io,
    Changed,
    Unavailable,
}
