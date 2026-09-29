use quote::ToTokens;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Item, ItemMacro, Token, braced};

pub(super) fn has_serde(attrs: &[Attribute], key: &str) -> bool {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("serde"))
        .any(|a| {
            let mut found = false;
            let _ = a.parse_nested_meta(|meta| {
                found |= meta.path.is_ident(key);
                if meta.input.peek(Token![=]) {
                    let _: syn::Expr = meta.value()?.parse()?;
                } else if meta.input.peek(syn::token::Paren) {
                    let inner;
                    syn::parenthesized!(inner in meta.input);
                    let _: syn::punctuated::Punctuated<syn::Meta, Token![,]> =
                        inner.parse_terminated(syn::Meta::parse, Token![,])?;
                }
                Ok(())
            });
            found
        })
}

pub(super) fn deserialize(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("derive"))
        .any(|a| {
            a.parse_args_with(syn::punctuated::Punctuated::<syn::Path, Token![,]>::parse_terminated)
                .is_ok_and(|paths| {
                    paths
                        .iter()
                        .any(|p| p.segments.last().is_some_and(|s| s.ident == "Deserialize"))
                })
        })
}

// Recognize exactly the recovered template. A same-named arbitrary macro is not proof.
pub(super) fn object_contract(items: &[Item]) -> Option<bool> {
    let expected: ItemMacro = syn::parse_str(r#"
        macro_rules! object { ($name:ident { $($(#[$attr:meta])* $field:ident : $ty:ty),* $(,)? }) => {
            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            #[serde(deny_unknown_fields)]
            pub struct $name { $($(#[$attr])* pub $field: $ty),* }
        }; }
    "#).expect("fixed object contract parses");
    let definitions = items
        .iter()
        .filter_map(|item| match item {
            Item::Macro(item) if item.ident.as_ref().is_some_and(|i| i == "object") => Some(
                item.mac.path.is_ident("macro_rules")
                    && item.mac.tokens.to_token_stream().to_string()
                        == expected.mac.tokens.to_token_stream().to_string(),
            ),
            _ => None,
        })
        .collect::<Vec<_>>();
    if definitions.is_empty() {
        None
    } else {
        Some(definitions.iter().all(|known| *known))
    }
}

pub(super) struct Object {
    pub(super) name: Ident,
    pub(super) fields: Vec<syn::Field>,
}

impl Parse for Object {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        let fields;
        braced!(fields in input);
        let fields = fields
            .parse_terminated(syn::Field::parse_named, Token![,])?
            .into_iter()
            .collect();
        if !input.is_empty() {
            return Err(input.error("unexpected object! tokens"));
        }
        Ok(Self { name, fields })
    }
}

pub(super) fn test_only(item: &Item) -> bool {
    let attrs = match item {
        Item::Fn(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Const(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        _ => return false,
    };
    attrs.iter().filter(|a| a.path().is_ident("cfg")).any(|a| {
        a.parse_args::<syn::Meta>()
            .is_ok_and(|meta| false_for_production(&meta))
    })
}

fn false_for_production(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(p) => p.is_ident("test"),
        syn::Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let parsed = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, Token![,]>::parse_terminated,
            );
            parsed.is_ok_and(|items| {
                if list.path.is_ident("all") {
                    items.iter().any(false_for_production)
                } else {
                    !items.is_empty() && items.iter().all(false_for_production)
                }
            })
        }
        _ => false,
    }
}
