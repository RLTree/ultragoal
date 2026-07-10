use super::types::EffectClass;
use std::collections::BTreeSet;

pub(super) fn permitted_effects(selected: EffectClass) -> BTreeSet<EffectClass> {
    match selected {
        EffectClass::Read => BTreeSet::from([EffectClass::Read]),
        EffectClass::PlannedWrite => BTreeSet::from([EffectClass::Read, EffectClass::PlannedWrite]),
        EffectClass::WorkspaceWrite => BTreeSet::from([
            EffectClass::Read,
            EffectClass::PlannedWrite,
            EffectClass::WorkspaceWrite,
        ]),
        EffectClass::ExternalWrite => {
            BTreeSet::from([EffectClass::Read, EffectClass::ExternalWrite])
        }
        EffectClass::Destructive => BTreeSet::from([
            EffectClass::Read,
            EffectClass::PlannedWrite,
            EffectClass::WorkspaceWrite,
            EffectClass::Destructive,
        ]),
    }
}
