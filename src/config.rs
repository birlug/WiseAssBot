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
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Deserialize)]
pub struct Rule {
    pub action: Action,
    pub contains: Vec<String>,
}

#[derive(Deserialize)]
pub struct RouteConfig {
    pub allowed_ip: Vec<IpCidr>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Block,
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
        assert_eq!(result.bot.rules.len(), 1);
        assert_eq!(result.bot.rules[0].action, Action::Block);
        assert_eq!(result.bot.rules[0].contains, vec!["bad word"]);
    }
}
