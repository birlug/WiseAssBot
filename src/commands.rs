use crate::bot::Bot;

use telegram_types::bot::types::Message;
use worker::*;

pub fn share(bot: &Bot, msg: &Message) -> Result<Response> {
    if let Some(user) = &msg.from {
        if bot.config.bot.admin_users_id.contains(&user.id) {
            // TODO: share the message on the linked social media
            return bot.pin(msg);
        }
    }
    Response::empty()
}

pub fn report(bot: &Bot, msg: &Message) -> Result<Response> {
    let id = bot.config.bot.report_chat_id;
    let reporter = msg.from.as_ref();
    let reportee = msg.reply_to_message.as_ref().map(|x| x.from.clone());
    let message = msg.reply_to_message.as_ref();

    let report = format!(
        include_str!("./response/report"),
        reporter, reportee, message
    );

    bot.send(id, &report)
}
