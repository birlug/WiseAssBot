use cidr::IpCidr;
use serde::Deserialize;
use telegram_types::bot::types::{ChatId, UserId};

#[derive(Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub routes: RouteConfig,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub admin_users_id: Vec<UserId>,
    pub report_chat_id: ChatId,
    pub allowed_chats_id: Vec<ChatId>,
}

#[derive(Deserialize)]
pub struct RouteConfig {
    pub allowed_ip: Vec<IpCidr>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = include_str!("../config-example.toml");
        let result: Config = toml::from_str(config).unwrap();

        assert_eq!(result.bot.admin_users_id, [UserId(7357)]);
        assert_eq!(result.bot.report_chat_id, ChatId(7357));
    }
}
