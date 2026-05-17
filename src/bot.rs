use crate::challenge::Quiz;
use crate::config::{Action, Config};
use crate::telegram::{self, ForwardMessage, PinChatMessage, WebhookReply};

use std::collections::HashMap;

use rust_persian_tools::{arabic_chars::HasArabic, digits::DigitsEn2Fa, persian_chars::HasPersian};
use telegram_types::bot::{
    methods::{
        AnswerCallbackQuery, ApproveJoinRequest, ChatTarget, DeclineJoinRequest, DeleteMessage,
        GetChatMember, ReplyMarkup, RestrictChatMember, SendMessage, TelegramResult,
    },
    types::{
        ChatId, ChatMemberStatus, ChatPermissions, InlineKeyboardButton,
        InlineKeyboardButtonPressed, InlineKeyboardMarkup, Message, MessageId, ParseMode, Update,
        UpdateContent, User, UserId,
    },
};
use worker::*;

const JOIN_PREFIX: &str = "_JOIN_";
const REPORT_PREFIX: &str = "_REPORT_";
type FnCmd = dyn Fn(&Bot, &Message) -> Result<Response>;

#[derive(serde::Serialize)]
struct ReportEntry {
    chat_id: i64,
    reported_user_id: i64,
    reported_by_user_id: i64,
    join_message_id: i64,
    reported_at: u64,
}

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

        let report_key = InlineKeyboardButton {
            text: "Report".to_string(),
            pressed: InlineKeyboardButtonPressed::CallbackData(report_callback_data(user.id)),
        };

        let response: TelegramResult<Message> = telegram::send_json_request(
            &self._token,
            SendMessage::new(ChatTarget::Id(chat_id), message)
                .parse_mode(ParseMode::Markdown)
                .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup {
                    inline_keyboard: vec![keys, vec![report_key]],
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

    async fn answer_callback(&self, callback_query_id: &str, text: &str, show_alert: bool) {
        let _ = telegram::send_json_request(
            &self._token,
            AnswerCallbackQuery::new(callback_query_id.to_string())
                .text(text.to_string())
                .show_alert(show_alert),
        )
        .await;
    }

    async fn report_join_request(
        &self,
        callback_query_id: &str,
        reporter_id: UserId,
        msg: &Message,
        reported_user_id: UserId,
    ) -> Result<Response> {
        let response = telegram::send_json_request(
            &self._token,
            GetChatMember {
                chat_id: ChatTarget::Id(msg.chat.id),
                user_id: reporter_id,
            },
        )
        .await?
        .json::<TelegramResult<telegram_types::bot::types::ChatMember>>()
        .await?;

        let Some(member) = response.result else {
            self.answer_callback(callback_query_id, "Unable to verify admin status.", true)
                .await;
            return Response::empty();
        };

        if !is_admin_status(&member.status) {
            self.answer_callback(
                callback_query_id,
                "Only group admins can report users.",
                true,
            )
            .await;
            return Response::empty();
        }

        let reported_at = Date::now().as_millis() / 1000;
        let entry = ReportEntry {
            chat_id: msg.chat.id.0,
            reported_user_id: reported_user_id.0,
            reported_by_user_id: reporter_id.0,
            join_message_id: msg.message_id.0,
            reported_at,
        };
        let key = report_key(msg.chat.id, reported_user_id, reporter_id, reported_at);
        let value = serde_json::to_string(&entry).map_err(|e| Error::RustError(e.to_string()))?;
        self.kv.put(&key, value)?.execute().await?;

        self.answer_callback(callback_query_id, "Reported user recorded.", false)
            .await;
        Response::empty()
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
                // hotfix: ignore persian names for now
                // as targeted spam users mostly have a name containing persian chars
                if r.from.first_name.has_persian(true) || r.from.first_name.has_arabic() {
                    return Response::empty();
                }
                return self.chat_join_request(&r.from, r.chat.id).await;
            }
            Some(UpdateContent::CallbackQuery(q)) => {
                // ignore callbacks without an associated message
                if let Some(msg) = &q.message {
                    if let Some(reported_user_id) =
                        q.data.as_deref().and_then(extract_reported_user_id)
                    {
                        return self
                            .report_join_request(&q.id, q.from.id, msg, reported_user_id)
                            .await;
                    }

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

fn report_callback_data(user_id: UserId) -> String {
    format!("{}{}", REPORT_PREFIX, user_id.0)
}

fn extract_reported_user_id(data: &str) -> Option<UserId> {
    data.strip_prefix(REPORT_PREFIX)
        .and_then(|user_id| user_id.parse::<i64>().ok())
        .map(UserId)
}

fn report_key(
    chat_id: ChatId,
    reported_user_id: UserId,
    reporter_id: UserId,
    reported_at: u64,
) -> String {
    format!(
        "{}{}:{}:{}:{}",
        REPORT_PREFIX, chat_id.0, reported_user_id.0, reported_at, reporter_id.0
    )
}

fn is_admin_status(status: &ChatMemberStatus) -> bool {
    matches!(
        status,
        ChatMemberStatus::Creator | ChatMemberStatus::Administrator
    )
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

    #[test]
    fn test_report_callback_data() {
        let data = report_callback_data(UserId(123));
        assert_eq!(extract_reported_user_id(&data), Some(UserId(123)));
        assert_eq!(extract_reported_user_id("_JOIN_123"), None);
    }

    #[test]
    fn test_report_key() {
        let key = report_key(ChatId(-100), UserId(123), UserId(456), 789);
        assert_eq!(key, "_REPORT_-100:123:789:456");
    }

    #[test]
    fn test_is_admin_status() {
        assert!(is_admin_status(&ChatMemberStatus::Creator));
        assert!(is_admin_status(&ChatMemberStatus::Administrator));
        assert!(!is_admin_status(&ChatMemberStatus::Member));
    }
}
