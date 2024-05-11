use crate::bot::*;

use telegram_types::bot::types::Message;
use worker::*;

pub fn report(bot: &Bot, msg: &Message) -> Result<Response> {
    let id = bot.config.bot.report_chat_id;

    let reporter = msg.from.as_ref().map(|x| x.id.0).unwrap_or_default();

    bot.send(id, &format!("{}", reporter))
}
