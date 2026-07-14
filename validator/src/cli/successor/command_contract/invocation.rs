use super::{OptionArgument, SuccessorCommand};
use crate::context::EffectClass;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputMode {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedInvocation {
    pub command: SuccessorCommand,
    pub effect: EffectClass,
    pub output_mode: OutputMode,
    pub arguments: Vec<OptionArgument>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpTarget {
    Root,
    Group(super::Group),
    Command(SuccessorCommand),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseOutcome {
    Invocation(ParsedInvocation),
    Help {
        target: HelpTarget,
        output_mode: OutputMode,
    },
    Version(OutputMode),
}
