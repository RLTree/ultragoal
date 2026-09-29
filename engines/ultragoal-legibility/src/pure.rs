// Exact project-reviewed native-library surfaces. This is not a prefix permission.
pub fn permitted(symbol: &str) -> bool {
    if matches!(
        symbol,
        "syn::Expr::Closure" | "syn::Local" | "syn::Pat::Ident" | "syn::visit::visit_local"
    ) {
        return true;
    }
    if matches!(
        symbol,
        "serde_json::Value::as_object"
            | "syn::GenericArgument::Lifetime"
            | "syn::Type::Array"
            | "syn::Type::Slice"
    ) {
        return true;
    }
    if crate::pure_syntax::permitted(symbol) {
        return true;
    }
    if matches!(symbol, "libc::LOCK_SH" | "libc::SIG_ERR") {
        return true;
    }
    if matches!(
        symbol,
        "libc::ELOOP"
            | "libc::ESRCH"
            | "libc::EINTR"
            | "libc::F_GETFL"
            | "libc::F_SETFL"
            | "libc::LOCK_EX"
            | "libc::LOCK_NB"
            | "libc::LOCK_UN"
            | "libc::O_CLOEXEC"
            | "libc::O_NOFOLLOW"
            | "libc::O_NONBLOCK"
            | "libc::O_RDONLY"
            | "libc::O_DIRECTORY"
            | "libc::POLLOUT"
            | "libc::PROC_PIDTBSDINFO"
            | "libc::SIGTERM"
            | "libc::SIGKILL"
            | "libc::SIGINT"
            | "libc::SIG_IGN"
            | "libc::SIG_DFL"
            | "libc::DIR"
            | "libc::pollfd"
            | "libc::proc_bsdinfo"
            | "libc::sighandler_t"
            | "serde_json::Value::as_array"
            | "serde_json::Value::as_bool"
            | "serde_json::Value::as_i64"
            | "serde_json::Value::as_str"
            | "serde_json::Value::as_u64"
            | "serde_json::Value::is_array"
            | "serde_json::Value::is_null"
            | "serde_json::Value::is_object"
            | "serde::Serialize::serialize"
            | "clap::error::ErrorKind::DisplayHelp"
            | "clap::error::ErrorKind::DisplayVersion"
            | "proc_macro2::TokenStream"
            | "proc_macro2::TokenTree"
            | "proc_macro2::TokenTree::Ident"
            | "proc_macro2::TokenTree::Group"
            | "proc_macro2::TokenTree::Punct"
            | "proc_macro2::TokenTree::Literal"
            | "quote::quote"
    ) {
        return true;
    }
    matches!(
        symbol,
        "anyhow::Result"
            | "anyhow::Error"
            | "anyhow::Context"
            | "anyhow::anyhow"
            | "anyhow::bail"
            | "anyhow::ensure"
            | "anyhow::Ok"
            | "serde::Serialize"
            | "serde::Deserialize"
            | "serde::Serializer"
            | "serde::Deserializer"
            | "serde::de::DeserializeOwned"
            | "serde::de::Error"
            | "serde::ser::Error"
            | "serde::ser::SerializeStruct"
            | "serde_json::json"
            | "serde_json::to_string"
            | "serde_json::to_string_pretty"
            | "serde_json::to_vec"
            | "serde_json::to_vec_pretty"
            | "serde_json::to_value"
            | "serde_json::Value"
            | "serde_json::Map"
            | "serde_json::Number"
            | "serde_json::Error"
            | "serde_json::Result"
            | "serde_json::Value::Null"
            | "serde_json::Value::Bool"
            | "serde_json::Value::Number"
            | "serde_json::Value::String"
            | "serde_json::Value::Array"
            | "serde_json::Value::Object"
            | "serde_json::Map::new"
            | "serde_json::Map::with_capacity"
            | "serde_json::Number::from"
            | "serde_json::Number::from_f64"
            | "sha2::Digest"
            | "sha2::Sha256"
            | "sha2::Sha256::digest"
            | "sha2::Sha256::new"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_pure_emitters_do_not_authorize_parsing_or_processes() {
        for allowed in [
            "anyhow::Context",
            "serde::Deserialize",
            "serde_json::to_value",
            "serde_json::Value::Object",
            "sha2::Sha256::digest",
        ] {
            assert!(permitted(allowed));
        }
        for denied in [
            "serde_json::from_str",
            "serde_json::from_slice",
            "serde_json::from_value",
            "libc::fork",
            "std::env::var",
            "std::fs::read",
            "serde_json::*",
            "unknown::serialize",
        ] {
            assert!(!permitted(denied), "{denied}");
        }
    }
}
