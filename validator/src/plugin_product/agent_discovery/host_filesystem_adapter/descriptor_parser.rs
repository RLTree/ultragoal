use crate::plugin_product::agent_discovery::model::ProjectAgentDescriptor;

pub(super) struct AgentDescriptorParserRequest<'a> {
    text: &'a str,
}

pub(super) struct AgentDescriptorParserResponse {
    descriptor: ProjectAgentDescriptor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AgentDescriptorParserError {
    Invalid,
}

pub(super) fn parse(text: &str) -> Result<ProjectAgentDescriptor, AgentDescriptorParserError> {
    execute(AgentDescriptorParserRequest { text }).map(|response| response.descriptor)
}

fn execute(
    request: AgentDescriptorParserRequest<'_>,
) -> Result<AgentDescriptorParserResponse, AgentDescriptorParserError> {
    toml::from_str(request.text)
        .map(|descriptor| AgentDescriptorParserResponse { descriptor })
        .map_err(|_| AgentDescriptorParserError::Invalid)
}
