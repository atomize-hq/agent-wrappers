use crate::{AgentWrapperError, AgentWrapperRunRequest};

use super::super::session_selectors::{
    parse_session_fork_v1, SessionSelectorV1, EXT_SESSION_FORK_V1,
};

pub(super) fn extract_fork_selector_v1(
    request: &AgentWrapperRunRequest,
) -> Result<Option<SessionSelectorV1>, AgentWrapperError> {
    request
        .extensions
        .get(EXT_SESSION_FORK_V1)
        .map(parse_session_fork_v1)
        .transpose()
}

