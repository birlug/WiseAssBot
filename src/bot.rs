use crate::config::Config;
use crate::telegram::{PinChatMessage, WebhookReply};

use std::collections::HashMap;

use telegram_types::bot::{
    methods::{ChatTarget, SendMessage},
    types::{ChatId, Message, MessageId, ParseMode},
};
use worker::*;

type FnCmd = dyn Fn(&Bot, &Message) -> Result<Response>;

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
        let message_id = msg
            .reply_to_message
            .as_ref()
            .map(|x| x.message_id)
            .unwrap_or(msg.message_id);

        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(msg.chat.id), text)
                .parse_mode(ParseMode::Markdown)
                .reply(message_id),
        ))
    }

    pub fn send(&self, chat_id: ChatId, text: &str) -> Result<Response> {
        Response::from_json(&WebhookReply::from(SendMessage::new(
            ChatTarget::Id(chat_id),
            text,
        )))
    }

    pub fn pin(&self, msg: &Message) -> Result<Response> {
        let chat_id = msg.chat.id;
        let message_id = msg
            .reply_to_message
            .as_ref()
            .map(|x| x.message_id)
            .unwrap_or(msg.message_id);

        Response::from_json(&WebhookReply::from(PinChatMessage {
            chat_id,
            message_id,
        }))
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
