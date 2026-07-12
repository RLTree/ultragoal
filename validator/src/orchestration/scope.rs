use super::OrchestrationError;
use super::model::{EffectGrant, bounded, validate_identifier};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CanonicalPath(String);

impl<'de> Deserialize<'de> for CanonicalPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(|_| serde::de::Error::custom("invalid canonical path"))
    }
}

impl CanonicalPath {
    pub fn parse(value: &str) -> Result<Self, OrchestrationError> {
        if value.is_empty()
            || value.len() > 512
            || !value.is_ascii()
            || value.starts_with('/')
            || value.ends_with('/')
            || value.contains('\0')
            || value.contains('\\')
            || value.chars().any(char::is_control)
            || value.split('/').any(|part| {
                part.is_empty() || matches!(part, "." | ".." | "~") || part.contains(':')
            })
        {
            return Err(OrchestrationError::InvalidPath);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn overlaps(&self, other: &Self) -> bool {
        path_is_within_ascii_alias(self, other) || path_is_within_ascii_alias(other, self)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedScope {
    pub paths: BTreeSet<CanonicalPath>,
    pub semantic_symbols: BTreeSet<String>,
    pub generated_outputs: BTreeSet<CanonicalPath>,
    pub fixtures: BTreeSet<CanonicalPath>,
    pub effects: BTreeSet<EffectGrant>,
}

impl OwnedScope {
    pub fn validate(&self) -> Result<(), OrchestrationError> {
        bounded(&self.paths)?;
        bounded(&self.semantic_symbols)?;
        bounded(&self.generated_outputs)?;
        bounded(&self.fixtures)?;
        bounded(&self.effects)?;
        for symbol in &self.semantic_symbols {
            validate_identifier(symbol)?;
        }
        for effect in &self.effects {
            effect.validate()?;
        }
        if has_path_overlap(&self.paths)
            || has_path_overlap(&self.generated_outputs)
            || has_path_overlap(&self.fixtures)
            || has_effect_overlap(&self.effects)
            || any_path_overlap(&self.paths, &self.generated_outputs)
            || any_path_overlap(&self.paths, &self.fixtures)
            || any_path_overlap(&self.generated_outputs, &self.fixtures)
        {
            return Err(OrchestrationError::DuplicateOutput);
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
            && self.semantic_symbols.is_empty()
            && self.generated_outputs.is_empty()
            && self.fixtures.is_empty()
            && self.effects.is_empty()
    }

    pub fn conflicts(&self, other: &Self) -> bool {
        any_path_overlap(&self.paths, &other.paths)
            || any_path_overlap(&self.paths, &other.generated_outputs)
            || any_path_overlap(&self.paths, &other.fixtures)
            || any_path_overlap(&self.generated_outputs, &other.paths)
            || any_symbol_overlap(&self.semantic_symbols, &other.semantic_symbols)
            || any_path_overlap(&self.generated_outputs, &other.generated_outputs)
            || any_path_overlap(&self.generated_outputs, &other.fixtures)
            || any_path_overlap(&self.fixtures, &other.paths)
            || any_path_overlap(&self.fixtures, &other.generated_outputs)
            || any_path_overlap(&self.fixtures, &other.fixtures)
            || any_effect_overlap(&self.effects, &other.effects)
    }

    pub(crate) fn is_subset_of(&self, other: &Self) -> bool {
        self.paths.iter().all(|path| other.contains_path(path))
            && self.semantic_symbols.iter().all(|symbol| {
                other
                    .semantic_symbols
                    .iter()
                    .any(|allowed| symbol_in_prefix(symbol, allowed))
            })
            && self
                .generated_outputs
                .iter()
                .all(|path| other.contains_generated(path))
            && self
                .fixtures
                .iter()
                .all(|path| other.contains_fixture(path))
            && self
                .effects
                .iter()
                .all(|effect| other.effects.contains(effect))
    }

    pub(crate) fn contains_path(&self, path: &CanonicalPath) -> bool {
        self.paths
            .iter()
            .any(|allowed| path_is_within(path, allowed))
    }

    pub(crate) fn contains_generated(&self, path: &CanonicalPath) -> bool {
        self.generated_outputs
            .iter()
            .any(|allowed| path_is_within(path, allowed))
    }

    pub(crate) fn contains_fixture(&self, path: &CanonicalPath) -> bool {
        self.fixtures
            .iter()
            .any(|allowed| path_is_within(path, allowed))
    }

    pub(crate) fn contains_any_path(&self, path: &CanonicalPath) -> bool {
        self.contains_path(path) || self.contains_generated(path) || self.contains_fixture(path)
    }

    pub(crate) fn overlaps_read_paths(&self, reads: &BTreeSet<CanonicalPath>) -> bool {
        self.paths
            .iter()
            .chain(self.generated_outputs.iter())
            .chain(self.fixtures.iter())
            .any(|owned| reads.iter().any(|read| owned.overlaps(read)))
    }
}

pub(crate) fn path_is_within(path: &CanonicalPath, root: &CanonicalPath) -> bool {
    path == root
        || path
            .as_str()
            .strip_prefix(root.as_str())
            .is_some_and(|rest| rest.starts_with('/'))
}

// Conflict and root-protection checks must conservatively model common
// case-insensitive filesystems. Authorization deliberately uses the exact-case
// `path_is_within` check above so an alias never broadens an allowlist.
fn path_is_within_ascii_alias(path: &CanonicalPath, root: &CanonicalPath) -> bool {
    let root_len = root.as_str().len();
    path.as_str()
        .get(..root_len)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(root.as_str()))
        && (path.as_str().len() == root_len
            || path.as_str().as_bytes().get(root_len) == Some(&b'/'))
}

pub(crate) fn symbol_in_prefix(symbol: &str, prefix: &str) -> bool {
    symbol == prefix
        || symbol
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with("::"))
}

pub(crate) fn effect_contains(allowed: &EffectGrant, requested: &EffectGrant) -> bool {
    allowed.class == requested.class
        && (allowed.target == requested.target
            || requested
                .target
                .strip_prefix(&allowed.target)
                .is_some_and(|rest| rest.starts_with('/')))
}

fn has_path_overlap(paths: &BTreeSet<CanonicalPath>) -> bool {
    let values: Vec<_> = paths.iter().collect();
    values
        .iter()
        .enumerate()
        .any(|(index, left)| values[index + 1..].iter().any(|right| left.overlaps(right)))
}

fn any_path_overlap(left: &BTreeSet<CanonicalPath>, right: &BTreeSet<CanonicalPath>) -> bool {
    left.iter()
        .any(|first| right.iter().any(|second| first.overlaps(second)))
}

fn any_symbol_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> bool {
    left.iter().any(|first| {
        right
            .iter()
            .any(|second| symbol_in_prefix(first, second) || symbol_in_prefix(second, first))
    })
}

fn any_effect_overlap(left: &BTreeSet<EffectGrant>, right: &BTreeSet<EffectGrant>) -> bool {
    left.iter()
        .any(|first| right.iter().any(|second| effect_overlaps(first, second)))
}

fn has_effect_overlap(effects: &BTreeSet<EffectGrant>) -> bool {
    let values: Vec<_> = effects.iter().collect();
    values.iter().enumerate().any(|(index, first)| {
        values[index + 1..]
            .iter()
            .any(|second| effect_overlaps(first, second))
    })
}

fn effect_overlaps(first: &EffectGrant, second: &EffectGrant) -> bool {
    first.class == second.class
        && (effect_target_is_within_ascii_alias(&first.target, &second.target)
            || effect_target_is_within_ascii_alias(&second.target, &first.target))
}

fn effect_target_is_within_ascii_alias(target: &str, root: &str) -> bool {
    let root_len = root.len();
    target
        .get(..root_len)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(root))
        && (target.len() == root_len || target.as_bytes().get(root_len) == Some(&b'/'))
}
