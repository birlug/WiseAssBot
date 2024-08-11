use crate::challenge::Quiz;
use crate::config::Config;
use crate::telegram::{self, ForwardMessage, PinChatMessage, WebhookReply};

use std::collections::HashMap;

use telegram_types::bot::{
    methods::{
        ApproveJoinRequest, ChatTarget, DeclineJoinRequest, DeleteMessage, ReplyMarkup, SendMessage,
    },
    types::{
        ChatId, InlineKeyboardButton, InlineKeyboardButtonPressed, InlineKeyboardMarkup, Message,
        MessageEntity, ParseMode, Update, UpdateContent, User, UserId,
    },
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

    pub fn forward(&self, msg: &Message, chat_id: ChatId) -> Result<Response> {
        let from_chat_id = msg.chat.id;
        let message_id = msg.message_id;

        Response::from_json(&WebhookReply::from(ForwardMessage {
            chat_id,
            from_chat_id,
            message_id,
        }))
    }

    pub fn approve_join_request(&self, chat_id: ChatId, user_id: UserId) -> Result<Response> {
        Response::from_json(&WebhookReply::from(ApproveJoinRequest {
            chat_id: ChatTarget::Id(chat_id),
            user_id,
        }))
    }

    pub fn decline_join_request(&self, chat_id: ChatId, user_id: UserId) -> Result<Response> {
        Response::from_json(&WebhookReply::from(DeclineJoinRequest {
            chat_id: ChatTarget::Id(chat_id),
            user_id,
        }))
    }

    fn chat_join_request(&self, user: &User, chat_id: ChatId) -> Result<Response> {
        let user_mention = format!("[{}](tg://user?id={})", user.first_name, user.id.0);

        let quiz = Quiz::new();
        let message = format!(include_str!("./response/join"), user_mention, quiz.encode(),);

        let keys = quiz
            .choices()
            .iter()
            .map(|x| InlineKeyboardButton {
                text: x.clone(),
                pressed: InlineKeyboardButtonPressed::CallbackData(x.clone()),
            })
            .collect::<Vec<InlineKeyboardButton>>();

        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(chat_id), message)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup {
                    inline_keyboard: vec![keys],
                })),
        ))
    }

    pub async fn process(&self, update: &Update) -> Result<Response> {
        match &update.content {
            Some(UpdateContent::Message(m)) => {
                if !self.config.bot.allowed_chats_id.contains(&m.chat.id) {
                    // report unallowed chats
                    return self.forward(&m, self.config.bot.report_chat_id);
                }
                if let Some(command) = m
                    .text
                    .as_ref()
                    .map(|t| t.trim())
                    .filter(|t| t.starts_with("!"))
                    .and_then(|t| self.commands.get(t))
                {
                    return command(self, &m);
                }
            }
            Some(UpdateContent::ChatJoinRequest(r)) => {
                if !self.config.bot.allowed_chats_id.contains(&r.chat.id) {
                    return Response::empty();
                }
                return self.chat_join_request(&r.from, r.chat.id);
            }
            Some(UpdateContent::CallbackQuery(q)) => {
                // ignore callbacks without an associated message
                if let Some(msg) = &q.message {
                    if extract_tg_id(&msg.entities) == q.from.id {
                        if let Some(text) = &msg.text {
                            let quiz = Quiz::from_str(&extract_question(&text));
                            let answer = &quiz.answer().to_string();

                            let _ = telegram::send_json_request(
                                &self._token,
                                DeleteMessage {
                                    chat_id: ChatTarget::Id(msg.chat.id),
                                    message_id: msg.message_id,
                                },
                            )
                            .await;

                            return if q.data.as_ref().map(|x| x == answer).unwrap_or_default() {
                                self.approve_join_request(msg.chat.id, q.from.id)
                            } else {
                                self.decline_join_request(msg.chat.id, q.from.id)
                            };
                        }
                    }
                }
            }
            _ => {}
        }

        Response::empty()
    }
}

fn extract_tg_id(entities: &Vec<MessageEntity>) -> UserId {
    if !entities.is_empty() {
        if let Some(user) = &entities[0].user {
            return user.id;
        }
    }
    // this case should never happen
    UserId(0)
}

fn extract_question(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    // currently the last line contains the question
    lines[lines.len() - 1].to_string()
}
