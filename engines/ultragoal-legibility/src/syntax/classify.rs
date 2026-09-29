use super::alias_parameters::default_error;
use super::closed_types::{closed_domain, closed_error};
use super::imports::{self, Aliases};
use std::collections::BTreeSet;
use syn::visit::Visit;

pub(super) fn authority(path: &str) -> Option<&'static str> {
    let parts = path.split("::").collect::<Vec<_>>();
    if path.starts_with("std::fs::")
        && parts
            .last()
            .is_some_and(|p| p.chars().next().is_some_and(char::is_lowercase))
    {
        return Some("filesystem");
    }
    if path.starts_with("libc::") {
        if matches!(
            parts.last(),
            Some(
                &"open"
                    | &"openat"
                    | &"close"
                    | &"opendir"
                    | &"fdopendir"
                    | &"readdir"
                    | &"closedir"
                    | &"fcntl"
                    | &"flock"
                    | &"read"
                    | &"write"
                    | &"poll"
                    | &"dup"
                    | &"dup2"
                    | &"pipe"
            )
        ) {
            return Some("filesystem");
        }
        if matches!(
            parts.last(),
            Some(
                &"kill"
                    | &"signal"
                    | &"fork"
                    | &"execve"
                    | &"waitpid"
                    | &"proc_pidinfo"
                    | &"proc_listchildpids"
                    | &"proc_listpids"
            )
        ) {
            return Some("process");
        }
    }
    if parts.windows(2).any(|pair| pair == ["process", "Command"]) {
        return Some("process");
    }
    if path == "std::env"
        || parts.windows(2).any(|pair| {
            pair[0] == "env"
                && matches!(
                    pair[1],
                    "var"
                        | "var_os"
                        | "vars"
                        | "vars_os"
                        | "args"
                        | "args_os"
                        | "current_dir"
                        | "temp_dir"
                )
        })
    {
        return Some("environment");
    }
    if parts.len() == 2
        && matches!(parts[0], "serde_json" | "toml" | "serde_saphyr")
        && matches!(
            parts[1],
            "from_str" | "from_slice" | "from_value" | "from_reader"
        )
    {
        return Some("structured_input");
    }
    if matches!(
        parts.first(),
        Some(&"serde_json" | &"serde" | &"toml" | &"serde_saphyr")
    ) && (parts.last() == Some(&"deserialize")
        || (parts.iter().any(|p| matches!(*p, "Deserializer" | "de"))
            && parts
                .last()
                .is_some_and(|p| matches!(*p, "from_str" | "from_slice" | "from_reader"))))
    {
        return Some("structured_input");
    }
    match path {
        "String" | "std::string::String" => Some("string"),
        "Path" | "std::path::Path" => Some("path"),
        "PathBuf" | "std::path::PathBuf" => Some("path_buf"),
        "serde_json::Value" => Some("serde_json_value"),
        "serde_json::Map" => Some("serde_json_map"),
        "toml::Value" => Some("toml_value"),
        _ => None,
    }
}

pub(super) fn closed_result(output: &syn::ReturnType, scopes: &[Aliases]) -> bool {
    let syn::ReturnType::Type(_, output) = output else {
        return false;
    };
    let syn::Type::Path(path) = output.as_ref() else {
        return false;
    };
    let Some(last) = path.path.segments.last() else {
        return false;
    };
    let default_error = default_error(&path.path, scopes);
    if last.ident != "Result" && default_error.is_none() {
        return false;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return false;
    };
    let types = args
        .args
        .iter()
        .filter_map(|arg| match arg {
            syn::GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect::<Vec<_>>();
    (types.len() == 2 || default_error.is_some() && types.len() == 1)
        && types.first().is_some_and(|ty| closed_domain(ty, true))
        && types.get(1).is_none_or(|ty| closed_error(ty))
}

pub(super) fn resolved_error(output: &syn::ReturnType, scopes: &[Aliases]) -> Option<String> {
    let syn::ReturnType::Type(_, output) = output else {
        return None;
    };
    let syn::Type::Path(path) = output.as_ref() else {
        return None;
    };
    let error = default_error(&path.path, scopes)?;
    let last = path.path.segments.last()?;
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    if args.args.len() == 1 && matches!(args.args.first(), Some(syn::GenericArgument::Type(_))) {
        Some(error.into())
    } else {
        None
    }
}

pub(super) fn structured(kind: &str) -> bool {
    matches!(
        kind,
        "serde_json_value" | "serde_json_map" | "toml_value" | "string_value_map"
    )
}

pub(super) fn semantic(name: &str) -> bool {
    name.split('_').any(|s| {
        matches!(
            s,
            "parse" | "decode" | "read" | "load" | "ingest" | "capture"
        )
    })
}

pub(super) fn type_facts(ty: &syn::Type, scopes: &[Aliases]) -> (Vec<String>, Vec<String>) {
    let mut facts = TypeFacts {
        scopes,
        paths: BTreeSet::new(),
        kinds: BTreeSet::new(),
        map: false,
    };
    facts.visit_type(ty);
    if facts.map && facts.kinds.iter().any(|s| structured(s)) {
        facts.kinds.insert("string_value_map".into());
    }
    (
        facts.paths.into_iter().collect(),
        facts.kinds.into_iter().collect(),
    )
}

struct TypeFacts<'a> {
    scopes: &'a [Aliases],
    paths: BTreeSet<String>,
    kinds: BTreeSet<String>,
    map: bool,
}

impl<'ast> Visit<'ast> for TypeFacts<'_> {
    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        for path in imports::resolve(&ty.path, self.scopes) {
            self.map |= matches!(path.rsplit("::").next(), Some("HashMap" | "BTreeMap"));
            if let Some(kind) = authority(&path) {
                self.kinds.insert(kind.into());
            }
            self.paths.insert(path);
        }
        syn::visit::visit_type_path(self, ty);
    }
}
