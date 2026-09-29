use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorityRequirement {
    None,
    Workspace,
    Root,
    External,
    HumanDestructive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityRequest {
    pub target: String,
    pub consequence: String,
    pub reversible: bool,
    pub accepted_loss_required: Option<String>,
}
