/// Mature Rust syntax compatibility only. No type, macro expansion or runtime claim.
pub fn rust(source: &str) -> Result<(), String> {
    syn::parse_file(source).map(|_| ()).map_err(|error| error.to_string())
}
