use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeErrorId {
    WrongCommand,
    EffectMismatch,
    UnexpectedArguments,
    ContextUnavailable,
}

impl RuntimeErrorId {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::WrongCommand => "successor_runtime_wrong_command",
            Self::EffectMismatch => "successor_runtime_effect_mismatch",
            Self::UnexpectedArguments => "successor_runtime_unexpected_arguments",
            Self::ContextUnavailable => "successor_runtime_context_unavailable",
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::WrongCommand => "successor runtime command unsupported",
            Self::EffectMismatch => "successor runtime effect mismatch",
            Self::UnexpectedArguments => "successor runtime arguments unsupported",
            Self::ContextUnavailable => "successor runtime context unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeError {
    id: RuntimeErrorId,
}

impl RuntimeError {
    pub(crate) const fn new(id: RuntimeErrorId) -> Self {
        Self { id }
    }

    pub(crate) const fn id(self) -> RuntimeErrorId {
        self.id
    }

    pub(crate) const fn stable_id(self) -> &'static str {
        self.id.as_str()
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id.message())
    }
}

impl std::error::Error for RuntimeError {}
