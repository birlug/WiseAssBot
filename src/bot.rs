use crate::config::Config;

use std::collections::HashMap;

use serde::Serialize;
use telegram_types::bot::{
    methods::{ChatTarget, Method, SendMessage},
    types::{ChatId, Message},
};
use worker::*;

type FnCmd = dyn Fn(&Bot, &Message) -> Result<Response>;

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

pub struct Bot {
    _token: String,
    pub commands: HashMap<String, Box<FnCmd>>,
    pub config: Config,
}

impl Bot {
    pub fn new(_token: String, config: String) -> Result<Self> {
        let config: Config =
            toml::from_str(&config).map_err(|e| Error::RustError(e.to_string()))?;

        Ok(Self {
            _token,
            config,
            commands: HashMap::new(),
        })
    }

    pub fn reply(&self, msg: &Message, text: &str) -> Result<Response> {
        let message_id = if let Some(replied) = &msg.reply_to_message {
            replied.message_id
        } else {
            msg.message_id
        };

        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(msg.chat.id), text).reply(message_id),
        ))
    }

    pub fn send(&self, chat_id: ChatId, text: &str) -> Result<Response> {
        Response::from_json(&WebhookReply::from(SendMessage::new(
            ChatTarget::Id(chat_id),
            text,
        )))
    }

    pub async fn process(&self, msg: &Message) -> Result<Response> {
        if let Some(command) = msg
            .text
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| t.starts_with("!"))
            .and_then(|t| self.commands.get(t))
        {
            return command(self, msg);
        }

        Response::empty()
    }
}
