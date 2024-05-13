use crate::bot::Bot;

use telegram_types::bot::types::Message;
use worker::*;

pub fn share(bot: &Bot, msg: &Message) -> Result<Response> {
    bot.pin(msg)
}

pub fn report(bot: &Bot, msg: &Message) -> Result<Response> {
    let id = bot.config.bot.report_chat_id;

    let reporter = msg.from.as_ref().map(|x| x.id.0).unwrap_or_default();
    let reportee = if let Some(m) = &msg.reply_to_message {
        m.from.as_ref().map(|x| x.id.0).unwrap_or_default()
    } else {
        0
    };

    bot.send(id, &format!("{}", reporter))
}
