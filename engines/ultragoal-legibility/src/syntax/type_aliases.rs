use super::{alias_parameters, classify, declarations, imports};
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::{GenericArgument, Item, PathArguments, ReturnType, Type};

pub(super) type Definitions = BTreeMap<String, syn::ItemType>;

pub(super) fn inherited(names: &[String]) -> imports::Aliases {
    names
        .iter()
        .map(|name| (name.clone(), vec!["anyhow::Result".into()]))
        .collect()
}

pub(super) fn collect(items: &[Item]) -> Definitions {
    items
        .iter()
        .filter(|item| !declarations::test_only(item))
        .filter_map(|item| match item {
            Item::Type(item) => Some((item.ident.to_string(), item.clone())),
            _ => None,
        })
        .collect()
}

pub(super) fn context(
    names: &[String],
    pairs: &[(String, String)],
) -> Result<(imports::Aliases, Definitions), String> {
    let mut scope = inherited(names);
    let mut definitions = Definitions::new();
    for (name, source) in pairs {
        let declaration = if let Ok(item) = syn::parse_str::<syn::ItemType>(source) {
            item
        } else {
            let ty: Type =
                syn::parse_str(source).map_err(|e| format!("Inherited alias {name}: {e}"))?;
            syn::parse2::<syn::ItemType>(quote!(type Imported = #ty;)).map_err(|e| e.to_string())?
        };
        scope.insert(name.clone(), vec![name.clone()]);
        definitions.insert(name.clone(), declaration);
    }
    Ok((scope, definitions))
}

pub(super) fn module_aliases(
    source: &str,
    names: &[String],
    pairs: &[(String, String)],
) -> Result<Vec<(String, String)>, String> {
    let file = syn::parse_file(source).map_err(|e| e.to_string())?;
    let (outer, inherited) = context(names, pairs)?;
    let (local, _) = imports::collect(&file.items);
    let definitions = [inherited, collect(&file.items)];
    let scopes = [outer, local];
    let mut out = Vec::new();
    let names = scopes
        .iter()
        .flat_map(|s| s.keys())
        .filter(|n| !n.contains("::") && *n != "*")
        .collect::<std::collections::BTreeSet<_>>();
    for name in names {
        let path: syn::Path = syn::parse_str(name).map_err(|e| e.to_string())?;
        let resolved = imports::resolve(&path, &scopes);
        if !resolved
            .iter()
            .any(|target| definitions.iter().any(|d| d.contains_key(target)))
        {
            continue;
        }
        if resolved.len() == 1
            && let Some(index) = scopes.iter().rposition(|s| s.contains_key(&resolved[0]))
            && let Some(alias) = definitions[index].get(&resolved[0])
            && !alias.generics.params.is_empty()
        {
            let mut exported = alias.clone();
            let mut local_scopes = scopes[..=index].to_vec();
            let Ok(shadow) = alias_parameters::shadow(alias) else {
                continue;
            };
            local_scopes.push(shadow);
            let mut local_definitions = definitions[..=index].to_vec();
            local_definitions.push(Definitions::new());
            *exported.ty = expand(
                &alias.ty,
                &local_scopes,
                &local_definitions,
                &mut vec![format!("{index}:{}", resolved[0])],
            )?;
            out.push((name.clone(), exported.to_token_stream().to_string()));
            continue;
        }
        let source: Type = syn::parse_str(name).map_err(|e| e.to_string())?;
        let ty = expand(&source, &scopes, &definitions, &mut vec![])?;
        out.push((name.clone(), ty.to_token_stream().to_string()));
    }
    Ok(out)
}

pub(super) fn output_paths(output: &ReturnType, scopes: &[imports::Aliases]) -> Vec<String> {
    match output {
        ReturnType::Type(_, ty) => classify::type_facts(ty, scopes).0,
        _ => Vec::new(),
    }
}

pub(super) fn expand_output(
    output: &ReturnType,
    scopes: &[imports::Aliases],
    definitions: &[Definitions],
) -> Result<ReturnType, String> {
    match output {
        ReturnType::Type(arrow, ty) => Ok(ReturnType::Type(
            *arrow,
            Box::new(expand(ty, scopes, definitions, &mut Vec::new())?),
        )),
        ReturnType::Default => Ok(ReturnType::Default),
    }
}

fn expand(
    ty: &Type,
    scopes: &[imports::Aliases],
    definitions: &[Definitions],
    seen: &mut Vec<String>,
) -> Result<Type, String> {
    let Type::Path(path) = ty else {
        return Ok(ty.clone());
    };
    if path.qself.is_some() {
        return Ok(ty.clone());
    }
    let resolved = imports::resolve(&path.path, scopes);
    if resolved.len() == 1 {
        let name = &resolved[0];
        if let Some(index) = scopes.iter().rposition(|scope| scope.contains_key(name))
            && let Some(alias) = definitions.get(index).and_then(|scope| scope.get(name))
        {
            let key = format!("{index}:{name}");
            if seen.contains(&key) {
                return Err(format!("Cyclic type alias: {name}"));
            }
            let mut values = alias_parameters::arguments(alias, &path.path)?;
            for value in values.values_mut() {
                *value = expand(value, scopes, definitions, seen)?;
            }
            seen.push(key);
            let mut local_scopes = scopes[..=index].to_vec();
            local_scopes.push(alias_parameters::shadow(alias)?);
            let mut local_definitions = definitions[..=index].to_vec();
            local_definitions.push(Definitions::new());
            let expanded = expand(&alias.ty, &local_scopes, &local_definitions, seen);
            seen.pop();
            return expanded.and_then(|ty| {
                if values.is_empty() {
                    Ok(ty)
                } else {
                    alias_parameters::substitute(&ty, &values)
                }
            });
        }
    }
    let mut result = path.clone();
    if resolved.len() == 1
        && let Ok(mut canonical) = syn::parse_str::<syn::Path>(&resolved[0])
    {
        if let (Some(source), Some(target)) =
            (path.path.segments.last(), canonical.segments.last_mut())
        {
            target.arguments = source.arguments.clone();
        }
        result.path = canonical;
    }
    for segment in &mut result.path.segments {
        if let PathArguments::AngleBracketed(args) = &mut segment.arguments {
            for arg in &mut args.args {
                if let GenericArgument::Type(inner) = arg {
                    *inner = expand(inner, scopes, definitions, seen)?;
                }
            }
        }
    }
    Ok(Type::Path(result))
}
