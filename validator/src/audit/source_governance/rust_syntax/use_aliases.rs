use super::classification::authority_for_segments;
use super::model::AuthorityKind;
use std::collections::{BTreeMap, BTreeSet};
use syn::UseTree;
use syn::visit::Visit;

pub(super) fn collect(file: &syn::File) -> BTreeMap<String, AuthorityKind> {
    let mut collector = AliasCollector::default();
    collector.visit_file(file);
    collector.aliases
}

#[derive(Default)]
struct AliasCollector {
    aliases: BTreeMap<String, AuthorityKind>,
}

impl<'ast> Visit<'ast> for AliasCollector {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        collect_tree(&node.tree, Vec::new(), &mut self.aliases);
    }
}

fn collect_tree(
    tree: &UseTree,
    mut prefix: Vec<String>,
    aliases: &mut BTreeMap<String, AuthorityKind>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_tree(&path.tree, prefix, aliases);
        }
        UseTree::Name(name) => {
            prefix.push(name.ident.to_string());
            if let Some(kind) = authority_for_segments(&prefix) {
                aliases.insert(name.ident.to_string(), kind);
            }
        }
        UseTree::Rename(rename) => {
            prefix.push(rename.ident.to_string());
            if let Some(kind) = authority_for_segments(&prefix) {
                aliases.insert(rename.rename.to_string(), kind);
            }
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_tree(item, prefix.clone(), aliases);
            }
        }
        UseTree::Glob(_) => {}
    }
}

pub(super) fn collect_roots(tree: &UseTree, roots: &mut BTreeSet<String>) {
    match tree {
        UseTree::Path(path) => {
            roots.insert(path.ident.to_string());
        }
        UseTree::Name(name) => {
            roots.insert(name.ident.to_string());
        }
        UseTree::Rename(rename) => {
            roots.insert(rename.ident.to_string());
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_roots(item, roots);
            }
        }
        UseTree::Glob(_) => {}
    }
}
