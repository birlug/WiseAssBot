use cidr::IpCidr;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub routes: RouteConfig,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub admin_users_id: Vec<i64>,
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
        let config = r#"
        [bot]
        admin_users_id = [
            7357
        ]
        mastodon_api_key = ""

        [routes]
        allowed_ip = [
            "91.108.56.0/22",
            "91.108.4.0/22",
            "91.108.8.0/22",
            "91.108.16.0/22",
            "91.108.12.0/22",
            "149.154.160.0/20",
            "91.105.192.0/23",
            "91.108.20.0/22",
            "185.76.151.0/24",
            "2001:b28:f23d::/48",
            "2001:b28:f23f::/48",
            "2001:67c:4e8::/48",
            "2001:b28:f23c::/48",
            "2a0a:f280::/32",
        ]"#;

        let result: Config = toml::from_str(config).unwrap();
        assert_eq!(result.bot.admin_users_id, [7357]);
    }
}
