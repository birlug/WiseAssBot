use serde::Serialize;
use telegram_types::bot::{
    methods::Method,
    types::{ChatId, Message, MessageId},
};
use worker::{Error, Fetch, Headers, Request, RequestInit, Response, Result};

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

pub async fn send_json_request<T: Method>(token: &str, request: T) -> Result<Response> {
    let mut request_builder = RequestInit::new();

    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Accept", "application/json")?;

    let payload = serde_json::to_string(&request)
        .map_err(|_| Error::RustError("invalid json payload".to_string()))?;

    request_builder.with_body(Some(worker::wasm_bindgen::JsValue::from_str(&payload)));
    request_builder
        .with_headers(headers)
        .with_method(worker::Method::Post);

    Fetch::Request(Request::new_with_init(&T::url(token), &request_builder)?)
        .send()
        .await
}
