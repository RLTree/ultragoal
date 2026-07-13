#[path = "build_support/repository_fit_template_sources.rs"]
mod repository_fit_template_sources;

use std::env;
use std::path::PathBuf;

fn main() {
    let crate_root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let repository_root = crate_root.parent().expect("repository root");
    let manifest = repository_root.join("plugin-manifest-draft.json");
    let templates = repository_root.join("templates");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));

    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rerun-if-changed={}", templates.display());
    repository_fit_template_sources::generate(&manifest, &templates, &output).unwrap_or_else(
        |failure| panic!("repository-fit template source validation failed: {failure}"),
    );
}
