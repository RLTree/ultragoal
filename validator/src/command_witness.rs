use crate::cli::successor::{Group, OptionSpec, ValueKind, catalog, effect_name};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CompiledCommandGroup {
    pub name: String,
    pub digest_sha256: String,
    pub route_count: usize,
}

#[derive(Serialize)]
struct RouteIdentity<'a> {
    subcommand: Option<&'a str>,
    effect: &'static str,
    purpose: &'a str,
    options: Vec<OptionIdentity>,
}

#[derive(Serialize)]
struct OptionIdentity {
    name: &'static str,
    kind: &'static str,
    required: bool,
}

fn option_identity(option: OptionSpec) -> OptionIdentity {
    OptionIdentity {
        name: option.name.as_str(),
        kind: match option.kind {
            ValueKind::Flag => "flag",
            ValueKind::Identifier => "identifier",
            ValueKind::RepositoryTarget => "repository-target",
            ValueKind::RelativePath => "relative-path",
            ValueKind::HostPath => "host-path",
        },
        required: option.required,
    }
}

pub(crate) fn compiled_groups() -> Result<Vec<CompiledCommandGroup>, &'static str> {
    let mut routes = BTreeMap::<String, Vec<RouteIdentity<'_>>>::new();
    let mut identities = BTreeSet::new();
    for descriptor in catalog() {
        let group = descriptor.command.group().as_str();
        let identity = (group, descriptor.subcommand);
        if !identities.insert(identity)
            || descriptor.purpose.trim().is_empty()
            || descriptor.purpose.len() > 512
        {
            return Err("compiled successor command catalog is invalid");
        }
        let mut option_names = BTreeSet::new();
        if descriptor
            .options
            .iter()
            .any(|option| !option_names.insert(option.name.as_str()))
        {
            return Err("compiled successor command options are duplicated");
        }
        routes
            .entry(group.to_owned())
            .or_default()
            .push(RouteIdentity {
                subcommand: descriptor.subcommand,
                effect: effect_name(descriptor.effect),
                purpose: descriptor.purpose,
                options: descriptor
                    .options
                    .iter()
                    .copied()
                    .map(option_identity)
                    .collect(),
            });
    }
    let expected = Group::ALL
        .iter()
        .map(|group| group.as_str())
        .collect::<BTreeSet<_>>();
    if routes.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err("compiled successor command groups are incomplete");
    }
    routes
        .into_iter()
        .map(|(name, mut rows)| {
            rows.sort_by(|left, right| left.subcommand.cmp(&right.subcommand));
            let route_count = rows.len();
            let bytes = serde_json::to_vec(&rows)
                .map_err(|_| "compiled successor command catalog cannot serialize")?;
            Ok(CompiledCommandGroup {
                name,
                digest_sha256: format!("{:x}", Sha256::digest(bytes)),
                route_count,
            })
        })
        .collect()
}
