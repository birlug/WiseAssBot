use crate::config::Config;

use std::collections::HashMap;

use serde::Serialize;
use telegram_types::bot::{
    methods::{ChatTarget, Method, SendMessage},
    types::Message,
};
use worker::*;

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

#[derive(Debug)]
struct Command {
    key: String,
    response: String,
}

pub struct Bot {
    _token: String,
    commands: HashMap<String, Command>,
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

    pub fn add_command(&mut self, key: &str, response: &str) {
        let cmd = Command {
            key: key.to_string(),
            response: response.to_string(),
        };
        self.commands.insert(key.to_string(), cmd);
    }

    pub fn reply(&self, msg: &Message, text: &str) -> Result<Response> {
        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(msg.chat.id), text).reply(msg.message_id),
        ))
    }

    pub fn reply_to_parent(&self, msg: &Message, text: &str) -> Result<Response> {
        let mut msg = Message::from(msg.clone());

        if let Some(replied) = &msg.reply_to_message {
            msg.message_id = replied.message_id;
        }

        self.reply(&msg, text)
    }

    pub fn process(&self, msg: &Message) -> Result<Response> {
        if let Some(command) = msg
            .text
            .as_ref()
            .map(|t| t.trim())
            .filter(|t| t.starts_with("!"))
            .and_then(|t| self.commands.get(t))
        {
            return self.reply_to_parent(msg, &command.response);
        }

        self.reply(msg, "unknown")
        // Err(Error::RustError("command not found".to_string()))
    }
}
