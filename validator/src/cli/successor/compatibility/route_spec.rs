use super::super::super::super::command_contract::LegacyCommand;

pub(in super::super) struct RouteSpec {
    pub(in super::super) prefix: &'static [&'static str],
    pub(in super::super) required: &'static [&'static str],
    pub(in super::super) command: LegacyCommand,
}

impl RouteSpec {
    pub(super) fn matches(&self, tokens: &[&str]) -> bool {
        tokens.starts_with(self.prefix)
            && self
                .required
                .iter()
                .all(|required| tokens.contains(required))
    }
}

pub(super) fn first_match(tokens: &[&str], catalogs: &[&[RouteSpec]]) -> Option<LegacyCommand> {
    catalogs
        .iter()
        .flat_map(|catalog| catalog.iter())
        .find(|route| route.matches(tokens))
        .map(|route| route.command)
}
