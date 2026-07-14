#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum AuthorityKind {
    Environment,
    JsonMap,
    JsonValue,
    Process,
    RawPath,
    RawPathBuffer,
    RawString,
    StructuredInput,
    StringValueMap,
    TomlValue,
}

impl AuthorityKind {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::JsonMap => "serde_json_map",
            Self::JsonValue => "serde_json_value",
            Self::Process => "process",
            Self::RawPath => "path",
            Self::RawPathBuffer => "path_buf",
            Self::RawString => "string",
            Self::StructuredInput => "structured_input",
            Self::StringValueMap => "string_value_map",
            Self::TomlValue => "toml_value",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct AuthorityUse {
    pub(crate) function: Option<String>,
    pub(crate) kind: AuthorityKind,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct IdentifierDeclaration {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct FunctionShape {
    pub(crate) name: String,
    pub(crate) returns_closed_result: bool,
}
