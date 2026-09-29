use super::imports;
use std::collections::BTreeMap;
use syn::{GenericArgument, GenericParam, ItemType, PathArguments, Type};

pub(super) fn names(alias: &ItemType) -> Result<Vec<String>, String> {
    if alias.generics.where_clause.is_some() {
        return Err("Generic type alias where clause unsupported".into());
    }
    alias
        .generics
        .params
        .iter()
        .map(|parameter| match parameter {
            GenericParam::Type(p) if p.bounds.is_empty() && p.default.is_none() => {
                Ok(p.ident.to_string())
            }
            _ => Err("Generic type alias requires plain type parameters".into()),
        })
        .collect()
}

pub(super) fn shadow(alias: &ItemType) -> Result<imports::Aliases, String> {
    Ok(names(alias)?
        .into_iter()
        .map(|n| (n.clone(), vec![n]))
        .collect())
}

pub(super) fn arguments(
    alias: &ItemType,
    path: &syn::Path,
) -> Result<BTreeMap<String, Type>, String> {
    let names = names(alias)?;
    let mut supplied = Vec::new();
    for (index, segment) in path.segments.iter().enumerate() {
        match &segment.arguments {
            PathArguments::None => (),
            PathArguments::AngleBracketed(args) if index + 1 == path.segments.len() => {
                for arg in &args.args {
                    let GenericArgument::Type(ty) = arg else {
                        return Err("Generic type alias requires type arguments".into());
                    };
                    supplied.push(ty.clone());
                }
            }
            _ => return Err("Generic type alias has unsupported arguments".into()),
        }
    }
    if names.len() != supplied.len() {
        return Err(format!(
            "Generic type alias arity: expected {}, got {}",
            names.len(),
            supplied.len()
        ));
    }
    Ok(names.into_iter().zip(supplied).collect())
}

pub(super) fn substitute(ty: &Type, values: &BTreeMap<String, Type>) -> Result<Type, String> {
    let mut output = ty.clone();
    match &mut output {
        Type::Path(p) if p.qself.is_none() => {
            if p.path.segments.len() > 1
                && values.contains_key(&p.path.segments[0].ident.to_string())
            {
                return Err("Generic type alias associated type unsupported".into());
            }
            if p.path.segments.len() == 1 && p.path.leading_colon.is_none() {
                let segment = &p.path.segments[0];
                if let Some(value) = values.get(&segment.ident.to_string()) {
                    if !matches!(segment.arguments, PathArguments::None) {
                        return Err("Generic type alias parameter has arguments".into());
                    }
                    return Ok(value.clone());
                }
            }
            for segment in &mut p.path.segments {
                match &mut segment.arguments {
                    PathArguments::None => (),
                    PathArguments::AngleBracketed(args) => {
                        for arg in &mut args.args {
                            let GenericArgument::Type(inner) = arg else {
                                return Err("Generic type alias RHS has non-type arguments".into());
                            };
                            *inner = substitute(inner, values)?;
                        }
                    }
                    _ => return Err("Generic type alias RHS has callable arguments".into()),
                }
            }
        }
        Type::Tuple(tuple) => {
            for inner in &mut tuple.elems {
                *inner = substitute(inner, values)?;
            }
        }
        Type::Reference(reference) => *reference.elem = substitute(&reference.elem, values)?,
        Type::Paren(paren) => *paren.elem = substitute(&paren.elem, values)?,
        _ => return Err("Generic type alias RHS shape unsupported".into()),
    }
    Ok(output)
}

pub(super) fn default_error(path: &syn::Path, scopes: &[imports::Aliases]) -> Option<&'static str> {
    let resolved = imports::resolve(path, scopes);
    match resolved.as_slice() {
        [path] if path == "anyhow::Result" => Some("anyhow::Error"),
        [path] if path == "std::io::Result" => Some("std::io::Error"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{analyze, analyze_with_full_context, module_type_aliases};
    #[test]
    fn generic_alias_substitutes_actual_payload_and_rejects_wrong_arity() {
        let source = "type ProcessResult<T> = Result<T, BoundaryError>; fn good()->ProcessResult<Payload>{todo!()} fn raw()->ProcessResult<serde_json::Value>{todo!()} fn wrong()->ProcessResult<Payload, Error>{todo!()}";
        let report = analyze("x.rs", source).unwrap();
        assert_eq!(
            report
                .functions
                .iter()
                .map(|f| f.returns_closed_result)
                .collect::<Vec<_>>(),
            [true, false, false]
        );
        assert!(report.limitations.iter().any(|l| l.contains("arity")));
    }
    #[test]
    fn generic_exports_preserve_parameters_and_imported_default_error() {
        let aliases =
            module_type_aliases("type ProcessResult<T> = std::io::Result<T>;", &[], &[]).unwrap();
        assert!(aliases[0].1.contains("type"));
        let qualified = vec![("super::ProcessResult".into(), aliases[0].1.clone())];
        let report = analyze_with_full_context("x.rs", "use super::ProcessResult as Outcome; fn good()->Outcome<Payload>{todo!()} fn raw()->Outcome<serde_json::Value>{todo!()}", false, &[], &qualified).unwrap();
        assert!(report.functions[0].returns_closed_result);
        assert!(!report.functions[1].returns_closed_result);
        assert_eq!(
            report.functions[0].resolved_error.as_deref(),
            Some("std::io::Error")
        );
    }
    #[test]
    fn io_default_error_requires_exact_import() {
        for (source, expected) in [
            (
                "use std::io::Result as Outcome; fn read()->Outcome<Payload>{todo!()}",
                true,
            ),
            (
                "use other::Result; fn read()->Result<Payload>{todo!()}",
                false,
            ),
            (
                "use std::io::Result; fn read()->Result<serde_json::Value>{todo!()}",
                false,
            ),
        ] {
            assert_eq!(
                analyze("x.rs", source).unwrap().functions[0].returns_closed_result,
                expected
            );
        }
    }
}
