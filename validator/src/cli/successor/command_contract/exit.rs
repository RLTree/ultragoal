use crate::context::EffectClass;

pub const fn effect_name(effect: EffectClass) -> &'static str {
    match effect {
        EffectClass::Read => "read",
        EffectClass::PlannedWrite => "planned-write",
        EffectClass::WorkspaceWrite => "workspace-write",
        EffectClass::ExternalWrite => "external-write",
        EffectClass::Destructive => "destructive",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitClass {
    Success,
    ActionableFinding,
    InvalidInvocation,
    BlockedAuthority,
    UnsupportedCapability,
    InternalFailure,
}

impl ExitClass {
    pub const fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::ActionableFinding => 1,
            Self::InvalidInvocation => 2,
            Self::BlockedAuthority => 3,
            Self::UnsupportedCapability => 4,
            Self::InternalFailure => 70,
        }
    }
}
