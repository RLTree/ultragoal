#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Report {
    pub(crate) functions: Vec<Function>,
    pub(crate) dependencies: Vec<DependencyUse>,
    pub(crate) authorities: Vec<AuthorityUse>,
    pub(crate) types: Vec<TypeDeclaration>,
    pub(crate) modules: Vec<String>,
    pub(crate) limitations: Vec<String>,
    pub(crate) failures: Vec<String>,
    pub(crate) includes: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) return_type: String,
    pub(crate) returns_closed_result: bool,
    pub(crate) resolved_error: Option<String>,
    pub(crate) input_types: Vec<String>,
    pub(crate) output_identifiers: Vec<String>,
    pub(crate) validation_calls: Vec<String>,
    pub(crate) calls: Vec<String>,
    pub(crate) direct_calls: Vec<String>,
    pub(crate) authority_kinds: Vec<String>,
    pub(crate) generic: GenericBounds,
    pub(crate) self_type: Option<String>,
    pub(crate) constructions: Vec<String>,
    pub(crate) conditions: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct GenericBounds {
    pub(crate) parameters: Vec<String>,
    pub(crate) response_parameters: Vec<String>,
    pub(crate) response: String,
    pub(crate) constraints: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DependencyUse {
    pub(crate) crate_name: String,
    pub(crate) symbol: String,
    pub(crate) function: Option<String>,
    pub(crate) owner: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct AuthorityUse {
    pub(crate) function: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeDeclaration {
    pub(crate) name: String,
    pub(crate) closed: bool,
    pub(crate) source: String,
    pub(crate) fields: Vec<(String, String)>,
    pub(crate) variants: Vec<String>,
    pub(crate) derived_default: bool,
    pub(crate) aliased_path: Option<String>,
}
