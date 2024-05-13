use serde::Serialize;
use telegram_types::bot::{
    methods::Method,
    types::{ChatId, Message, MessageId},
};

#[derive(Clone, Debug, Serialize)]
pub struct WebhookReply<M: Method> {
    pub method: String,
    #[serde(flatten)]
    pub content: M,
}

impl<M: Method> From<M> for WebhookReply<M> {
    fn from(method: M) -> WebhookReply<M> {
        WebhookReply {
            method: <M>::NAME.to_string(),
            content: method,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct PinChatMessage {
    pub chat_id: ChatId,
    pub message_id: MessageId,
}

impl Method for PinChatMessage {
    const NAME: &'static str = "pinChatMessage";
    type Item = Message;
}

#[derive(Clone, Serialize)]
pub struct ForwardMessage {
    pub chat_id: ChatId,
    pub from_chat_id: ChatId,
    pub message_id: MessageId,
}

impl Method for ForwardMessage {
    const NAME: &'static str = "forwardMessage";
    type Item = Message;
}
