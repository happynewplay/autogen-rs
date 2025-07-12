use crate::message_handler_context::MessageHandlerContext;
use crate::topic::TopicId;

/// `DefaultTopicId` provides a sensible default for the `topic_id` and `source`
/// fields of a `TopicId`.
///
/// If created in the context of a message handler, the source will be set to
/// the `agent_id` of the message handler; otherwise, it will be set to "default".
pub struct DefaultTopicId;

impl DefaultTopicId {
    /// Creates a new `TopicId` with default values.
    ///
    /// # Arguments
    ///
    /// * `r#type` - Topic type to publish the message to. Defaults to "default".
    /// * `source` - Topic source to publish the message from. If `None`, the source
    ///              will be set to the `agent_id` of the message handler if in the
    ///              context of a message handler; otherwise, it will be set to "default".
    ///              Defaults to `None`.
    ///
    /// # Returns
    ///
    /// A new `TopicId`.
    pub fn new(r#type: Option<String>, source: Option<String>) -> TopicId {
        let source = source.unwrap_or_else(|| {
            MessageHandlerContext::agent_id()
                .map(|id| id.key)
                .unwrap_or_else(|| "default".to_string())
        });
        TopicId {
            r#type: r#type.unwrap_or_else(|| "default".to_string()),
            source,
        }
    }
}