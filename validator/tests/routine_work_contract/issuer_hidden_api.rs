use std::fs;
use std::path::Path;

pub(crate) fn tree_has_hidden_public_api(root: &Path) -> bool {
    let mut entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    entries.into_iter().any(|path| {
        if path.is_dir() {
            tree_has_hidden_public_api(&path)
        } else {
            path.extension().is_some_and(|value| value == "rs")
                && file_has_hidden_public_api(&fs::read_to_string(path).unwrap())
        }
    })
}

pub(crate) fn file_has_hidden_public_api(source: &str) -> bool {
    let file = syn::parse_file(source).expect("issuer visibility source parses");
    file.items.iter().any(item_has_hidden_public_api)
}

fn item_has_hidden_public_api(item: &syn::Item) -> bool {
    use syn::Item;
    let direct = match item {
        Item::Const(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Enum(value) => {
            public(&value.vis)
                && (hidden(&value.attrs)
                    || value.variants.iter().any(|variant| {
                        hidden(&variant.attrs)
                            || variant.fields.iter().any(|field| hidden(&field.attrs))
                    }))
        }
        Item::ExternCrate(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Fn(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Mod(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Static(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Struct(value) => {
            public(&value.vis)
                && (hidden(&value.attrs)
                    || value
                        .fields
                        .iter()
                        .any(|field| public(&field.vis) && hidden(&field.attrs)))
        }
        Item::Trait(value) => public(&value.vis) && hidden(&value.attrs),
        Item::TraitAlias(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Type(value) => public(&value.vis) && hidden(&value.attrs),
        Item::Union(value) => {
            public(&value.vis)
                && (hidden(&value.attrs)
                    || value
                        .fields
                        .named
                        .iter()
                        .any(|field| public(&field.vis) && hidden(&field.attrs)))
        }
        Item::Use(value) => public(&value.vis) && hidden(&value.attrs),
        _ => false,
    };
    if direct {
        return true;
    }
    match item {
        Item::Impl(value) => value.items.iter().any(|item| match item {
            syn::ImplItem::Const(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ImplItem::Fn(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ImplItem::Type(value) => public(&value.vis) && hidden(&value.attrs),
            _ => false,
        }),
        Item::Mod(value) => value
            .content
            .as_ref()
            .is_some_and(|(_, items)| items.iter().any(item_has_hidden_public_api)),
        Item::Trait(value) if public(&value.vis) => value.items.iter().any(|item| match item {
            syn::TraitItem::Const(value) => hidden(&value.attrs),
            syn::TraitItem::Fn(value) => hidden(&value.attrs),
            syn::TraitItem::Macro(value) => hidden(&value.attrs),
            syn::TraitItem::Type(value) => hidden(&value.attrs),
            _ => false,
        }),
        Item::ForeignMod(value) => value.items.iter().any(|item| match item {
            syn::ForeignItem::Fn(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ForeignItem::Static(value) => public(&value.vis) && hidden(&value.attrs),
            syn::ForeignItem::Type(value) => public(&value.vis) && hidden(&value.attrs),
            _ => false,
        }),
        // `thread_local!` is the only production item macro; its expanded public
        // surface is covered by the rustdoc inventory. Unknown item macros fail
        // this source law because their expansion cannot be inspected here.
        Item::Macro(value) => !value.mac.path.is_ident("thread_local"),
        _ => false,
    }
}

fn public(visibility: &syn::Visibility) -> bool {
    matches!(visibility, syn::Visibility::Public(_))
}

fn hidden(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| meta_hidden(&attribute.meta))
}

fn meta_hidden(meta: &syn::Meta) -> bool {
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if list.path.is_ident("doc") && list.tokens.to_string() == "hidden" {
        return true;
    }
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    parser
        .parse2(list.tokens.clone())
        .is_ok_and(|items| items.iter().skip(1).any(meta_hidden))
}
