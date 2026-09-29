use quote::ToTokens;
use std::collections::BTreeMap;
use syn::visit::Visit;

pub struct Trait {
    pub key: String,
    pub scope: String,
    pub private: bool,
    pub supers: Vec<String>,
}
pub struct Implementation {
    pub scope: String,
    pub target: String,
    pub ty: syn::Type,
    pub generic: bool,
}
#[derive(Default)]
pub struct Index {
    pub traits: Vec<Trait>,
    pub implementations: Vec<Implementation>,
    pub opaque_impl_packages: Vec<String>,
    pub imports: BTreeMap<String, Vec<String>>,
    pub private_modules: Vec<String>,
}

pub fn scope(path: &str) -> Option<String> {
    let (owner, relative) = if let Some((owner, p)) = path.rsplit_once("/src/") {
        (owner, p)
    } else {
        ("project", path.strip_prefix("src/")?)
    };
    let mut segments: Vec<_> = relative.strip_suffix(".rs")?.split('/').collect();
    if segments
        .last()
        .is_some_and(|s| matches!(*s, "mod" | "main" | "lib"))
    {
        segments.pop();
    }
    Some(
        std::iter::once(owner)
            .chain(segments)
            .collect::<Vec<_>>()
            .join("::"),
    )
}

impl Index {
    pub fn collect(
        files: &BTreeMap<String, Vec<u8>>,
        paths: impl Iterator<Item = String>,
    ) -> Result<Index, syn::Error> {
        let mut index = Index::default();
        for path in paths {
            let Some(scope) = scope(&path) else {
                continue;
            };
            let Some(bytes) = files.get(&path) else {
                continue;
            };
            let Ok(source) = std::str::from_utf8(bytes) else {
                continue;
            };
            let file = syn::parse_file(source)?;
            let mut visitor = Collector {
                scope,
                private: false,
                index: &mut index,
            };
            visitor.visit_file(&file);
        }
        for marker in &mut index.traits {
            marker.private |= index
                .private_modules
                .iter()
                .any(|module| marker.key.starts_with(&format!("{module}::")));
        }
        Ok(index)
    }

    pub fn resolve<'a>(&'a self, current: &str, path: &str) -> Option<&'a Trait> {
        let full = super::imports::absolute(current, path)?;
        super::imports::resolve(self, &full, &mut Vec::new())
    }
}

struct Collector<'a> {
    scope: String,
    private: bool,
    index: &'a mut Index,
}
impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if crate::syntax::production_item(item) {
            syn::visit::visit_item(self, item);
        }
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let old_scope = self.scope.clone();
        let old_private = self.private;
        self.scope.push_str(&format!("::{}", item.ident));
        self.private |= !matches!(item.vis, syn::Visibility::Public(_));
        if !matches!(item.vis, syn::Visibility::Public(_)) {
            self.index.private_modules.push(self.scope.clone());
        }
        syn::visit::visit_item_mod(self, item);
        self.scope = old_scope;
        self.private = old_private;
    }
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        super::imports::collect(&item.tree, &self.scope, &[], &mut self.index.imports);
    }
    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.index.traits.push(Trait {
            key: format!("{}::{}", self.scope, item.ident),
            scope: self.scope.clone(),
            private: self.private || !matches!(item.vis, syn::Visibility::Public(_)),
            supers: item
                .supertraits
                .iter()
                .filter_map(|b| match b {
                    syn::TypeParamBound::Trait(t) => {
                        Some(t.path.to_token_stream().to_string().replace(' ', ""))
                    }
                    _ => None,
                })
                .collect(),
        });
    }
    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if let Some((target, _)) = &item.trait_ {
            self.index.implementations.push(Implementation {
                scope: self.scope.clone(),
                target: target.to_token_stream().to_string().replace(' ', ""),
                ty: (*item.self_ty).clone(),
                generic: !item.generics.params.is_empty(),
            });
        }
    }
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        fn emits_impl(tokens: proc_macro2::TokenStream) -> bool {
            tokens.into_iter().any(|t| match t {
                proc_macro2::TokenTree::Ident(i) => i == "impl" || i == "trait",
                proc_macro2::TokenTree::Group(g) => emits_impl(g.stream()),
                _ => false,
            })
        }
        if emits_impl(item.mac.tokens.clone()) {
            self.index
                .opaque_impl_packages
                .push(self.scope.split("::").next().unwrap().into());
        }
    }
}
