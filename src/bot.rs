use std::borrow::Cow;

use worker::*;
use serde::{Deserialize, Serialize};
use telegram_types::bot::methods::{
    ChatTarget, DeleteWebhook, GetMe, Method as TMethod, SendMessage, SetWebhook, TelegramResult,
    UpdateTypes,
};
use telegram_types::bot::types::{Message, User};

#[derive(Clone, Debug, Serialize)]
pub struct WebhookReply<T: TMethod> {
    pub method: String,
    #[serde(flatten)]
    pub content: T,
}

impl<T: TMethod> From<T> for WebhookReply<T> {
    fn from(method: T) -> WebhookReply<T> {
        WebhookReply {
            method: <T>::NAME.to_string(),
            content: method,
        }
    }
}

#[derive(Deserialize)]
struct Config {
    allowed_users_id: Vec<i64>,
}

pub struct Bot {
    pub token: String,
    pub config: Config,
}

impl Bot {
    pub fn new(token: String, config: String) -> Result<Self> {
        let config: Config =
            toml::from_str(&config).map_err(|e| Error::RustError(e.to_string()))?;

        Ok(Self { token, config })
    }

    async fn send_json_request<T: TMethod>(&self, _: T, method: Method) -> Result<Response> {
        let mut request_builder = RequestInit::new();
        request_builder.with_method(method);

        Fetch::Request(Request::new_with_init(
            &T::url(&self.token),
            &request_builder,
        )?)
        .send()
        .await
    }

    pub async fn get_me(&self) -> Result<User> {
        let mut result = self.send_json_request(GetMe, Method::Get).await?;
        result
            .json::<TelegramResult<User>>()
            .await?
            .into_result()
            .map_err(|e| Error::RustError(e.description))
    }

    pub async fn set_webhook(&self, url: String) -> Result<()> {
        let payload = DeleteWebhook;
        self.send_json_request(payload, Method::Post).await?;

        let mut payload = SetWebhook::new(url);
        // only allow message types
        payload.allowed_updates = Some(Cow::from(&[UpdateTypes::Message]));
        self.send_json_request(payload, Method::Post).await?;

        Ok(())
    }

    pub fn reply(&self, msg: &Message, text: &str) -> Result<Response> {
        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(msg.chat.id), text).reply(msg.message_id),
        ))
    }

    pub fn process(&self, msg: &Message) -> Result<Response> {
        let text = format!("{:?}", msg);
        self.reply(msg, &text)
    }
}
