use super::{
    classify,
    imports::{self, Aliases},
    model::GenericBounds,
};
use quote::ToTokens;

pub(super) fn collect(
    sig: &syn::Signature,
    effective: &syn::ReturnType,
    scopes: &[Aliases],
) -> GenericBounds {
    let mut facts = GenericBounds::default();
    for parameter in sig.generics.type_params() {
        let name = parameter.ident.to_string();
        facts.parameters.push(name.clone());
        bounds(&name, &parameter.bounds, scopes, &mut facts);
    }
    if let Some(clause) = &sig.generics.where_clause {
        for predicate in &clause.predicates {
            if let syn::WherePredicate::Type(predicate) = predicate {
                bounds(
                    &predicate
                        .bounded_ty
                        .to_token_stream()
                        .to_string()
                        .replace(' ', ""),
                    &predicate.bounds,
                    scopes,
                    &mut facts,
                );
            }
        }
    }
    if let syn::ReturnType::Type(_, ty) = effective {
        let mut response = ty.as_ref();
        if let syn::Type::Path(path) = response
            && let Some(last) = path.path.segments.last()
            && last.ident == "Result"
            && let syn::PathArguments::AngleBracketed(args) = &last.arguments
            && let Some(syn::GenericArgument::Type(ty)) = args.args.first()
        {
            response = ty;
        }
        facts.response = response.to_token_stream().to_string().replace(' ', "");
        let identifiers = classify::type_facts(response, scopes).0;
        facts.response_parameters = facts
            .parameters
            .iter()
            .filter(|p| identifiers.contains(p))
            .cloned()
            .collect();
    }
    facts
}

fn bounds(
    subject: &str,
    bounds: &syn::punctuated::Punctuated<syn::TypeParamBound, syn::Token![+]>,
    scopes: &[Aliases],
    facts: &mut GenericBounds,
) {
    for bound in bounds {
        if let syn::TypeParamBound::Trait(bound) = bound {
            for target in imports::resolve(&bound.path, scopes) {
                facts.constraints.push((subject.into(), target));
            }
        }
    }
}
