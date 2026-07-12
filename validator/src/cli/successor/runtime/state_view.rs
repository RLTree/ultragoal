#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StateProjection<'a> {
    Summary,
    Findings,
    Claims,
    Diagnose(Option<&'a str>),
    Next,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StateDisposition {
    NoAction,
    Action,
    AuthorityRequest,
    NoLegalRoute,
}

pub(crate) trait StateView {
    fn context_id(&self) -> &str;
    fn state_id(&self) -> &str;
    fn finding_count(&self) -> usize;
    fn disposition(&self) -> StateDisposition;
    fn project(&self, projection: StateProjection<'_>) -> Result<Option<Vec<u8>>, String>;
}
