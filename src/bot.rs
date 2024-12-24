use crate::challenge::Quiz;
use crate::config::{Action, Config};
use crate::telegram::{self, ForwardMessage, PinChatMessage, WebhookReply};

use std::collections::HashMap;

use rust_persian_tools::digits::DigitsEn2Fa;
use telegram_types::bot::{
    methods::{
        ApproveJoinRequest, ChatTarget, DeclineJoinRequest, DeleteMessage, ReplyMarkup,
        RestrictChatMember, SendMessage, TelegramResult,
    },
    types::{
        ChatId, ChatPermissions, InlineKeyboardButton, InlineKeyboardButtonPressed,
        InlineKeyboardMarkup, Message, MessageId, ParseMode, Update, UpdateContent, User, UserId,
    },
};
use worker::*;

const JOIN_PREFIX: &str = "_JOIN_";
type FnCmd = dyn Fn(&Bot, &Message) -> Result<Response>;

pub struct Bot {
    _token: String,
    kv: kv::KvStore,
    pub commands: HashMap<String, Box<FnCmd>>,
    pub config: Config,
}

impl Bot {
    pub fn new(_token: String, config: String, kv: kv::KvStore) -> Result<Self> {
        let config: Config =
            toml::from_str(&config).map_err(|e| Error::RustError(e.to_string()))?;

        Ok(Self {
            _token,
            kv,
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

    pub async fn remove_expired_join_requests(&self) -> Result<()> {
        const TTL_LIMIT: u64 = 7 * 60;

        let keys = self
            .kv
            .list()
            .prefix(JOIN_PREFIX.to_string())
            .execute()
            .await?
            .keys;

        for key in keys {
            // TODO: join requests without expiration date are invalid
            if let Some(ttl) = key.expiration {
                let now = Date::now().as_millis() / 1000;
                if ttl - now < TTL_LIMIT {
                    let (chat_id, message_id) = extract_key_details(&key.name);
                    let _ = telegram::send_json_request(
                        &self._token,
                        DeleteMessage {
                            chat_id: ChatTarget::Id(chat_id),
                            message_id,
                        },
                    )
                    .await;
                    // the key will be removed automatically after being expired
                }
            }
        }

        Ok(())
    }

    async fn chat_join_request(&self, user: &User, chat_id: ChatId) -> Result<Response> {
        let user_mention = format!("[{}](tg://user?id={})", user.first_name, user.id.0);

        let quiz = Quiz::new();
        let message = format!(include_str!("./response/join"), user_mention, quiz.encode());

        let keys = quiz
            .choices()
            .iter()
            .map(|x| InlineKeyboardButton {
                text: x.clone(),
                pressed: InlineKeyboardButtonPressed::CallbackData(x.clone()),
            })
            .collect::<Vec<InlineKeyboardButton>>();

        let response: TelegramResult<Message> = telegram::send_json_request(
            &self._token,
            SendMessage::new(ChatTarget::Id(chat_id), message)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup {
                    inline_keyboard: vec![keys],
                })),
        )
        .await?
        .json()
        .await?;

        let message_id = response
            .result
            .ok_or("response result empty".to_string())
            .map_err(|e| Error::RustError(e))?
            .message_id;
        let _ = self
            .kv
            .put(
                &format!("{}{}:{}", JOIN_PREFIX, chat_id.0, message_id.0),
                user.id.0,
            )?
            .expiration_ttl(10 * 60) // FIXME: configurable expiration ttl
            .execute()
            .await?;

        Response::empty()
    }

    async fn restrict_user(&self, user: &User, chat_id: ChatId) {
        let _ = telegram::send_json_request(
            &self._token,
            RestrictChatMember {
                chat_id: ChatTarget::Id(chat_id),
                user_id: user.id,
                permissions: ChatPermissions {
                    can_send_messages: false,
                },
            },
        )
        .await;
    }

    pub async fn process(&self, update: &Update) -> Result<Response> {
        match &update.content {
            Some(UpdateContent::Message(m)) => {
                if !self.config.bot.allowed_chats_id.contains(&m.chat.id) {
                    // report unallowed chats
                    return self.forward(&m, self.config.bot.report_chat_id);
                }
                // rules
                for rule in &self.config.bot.rules {
                    for word in &rule.contains {
                        if m.text
                            .as_ref()
                            .map(|t| t.contains(word))
                            .unwrap_or_default()
                        {
                            match rule.action {
                                Action::Block => {
                                    if let Some(u) = &m.from {
                                        self.restrict_user(&u, m.chat.id).await;
                                    }
                                }
                            }
                            return self.forward(&m, self.config.bot.report_chat_id);
                        }
                    }
                }
                // easter egg: appreciate powers of two!
                if m.message_id.0 & (m.message_id.0 - 1) == 0 {
                    let reply = format!(
                        include_str!("./response/easter-egg"),
                        m.message_id.0.digits_en_to_fa()
                    );
                    return self.reply(m, &reply);
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
                return self.chat_join_request(&r.from, r.chat.id).await;
            }
            Some(UpdateContent::CallbackQuery(q)) => {
                // ignore callbacks without an associated message
                if let Some(msg) = &q.message {
                    let key = format!("{}{}:{}", JOIN_PREFIX, msg.chat.id.0, msg.message_id.0);

                    let assigned_user = self.kv.get(&key).text().await?.unwrap_or_default();
                    let answered_user = q.from.id.0.to_string();

                    if assigned_user == answered_user {
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
                            self.kv.delete(&key).await?; // TODO: remove stale keys within an interval

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

fn extract_question(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    // currently the last line contains the question
    lines[lines.len() - 1].to_string()
}

fn extract_key_details(text: &str) -> (ChatId, MessageId) {
    let mut chat_id = 0;
    let mut message_id = 0;

    let info = text.strip_prefix(JOIN_PREFIX).unwrap(); // safe to unwrap
    let info = info
        .split(':')
        .map(|x| x.parse().unwrap_or_default())
        .collect::<Vec<i64>>();

    if info.len() == 2 {
        chat_id = info[0];
        message_id = info[1];
    }

    (ChatId(chat_id), MessageId(message_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_question() {
        let question = extract_question("first line\nsecond line\nthird line");
        assert_eq!(question, "third line");
    }

    #[test]
    fn test_extract_key_details() {
        let (chat_id, message_id) = extract_key_details(&format!("{}{}:{}", JOIN_PREFIX, 123, 456));
        assert_eq!(chat_id.0, 123);
        assert_eq!(message_id.0, 456);

        let (chat_id, message_id) = extract_key_details(&format!("{}{}-", JOIN_PREFIX, 123));
        assert_eq!(chat_id.0, 0);
        assert_eq!(message_id.0, 0);
    }
}
