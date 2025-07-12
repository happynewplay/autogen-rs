use crate::agent_id::AgentId;
use crate::cancellation_token::CancellationToken;
use crate::topic::TopicId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageContext {
    pub sender: Option<AgentId>,
    pub topic_id: Option<TopicId>,
    pub is_rpc: bool,
    pub cancellation_token: CancellationToken,
    pub message_id: String,
}