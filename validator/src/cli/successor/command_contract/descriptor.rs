use super::{OptionSpec, SuccessorCommand};
use crate::context::EffectClass;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandDescriptor {
    pub command: SuccessorCommand,
    pub subcommand: Option<&'static str>,
    pub effect: EffectClass,
    pub purpose: &'static str,
    pub options: &'static [OptionSpec],
}
