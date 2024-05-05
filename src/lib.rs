use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use telegram_types::bot::methods::{
    ChatTarget, DeleteWebhook, GetMe, Method as TMethod, SendMessage, SetWebhook, TelegramResult,
    UpdateTypes,
};
use telegram_types::bot::types::{Message, Update, UpdateContent, User};
use worker::*;

#[derive(Clone, Debug, Serialize)]
pub struct WebhookReply<T: TMethod> {
    pub method: String,
    #[serde(flatten)]
    pub content: T,
}

impl<T: TMethod> From<T> for WebhookReply<T> {
    fn from(method: T) -> WebhookReply<T> {
        WebhookReply {
            method: <T>::NAME.to_string(),
            content: method,
        }
    }
}

#[derive(Deserialize)]
struct Config {
    allowed_users_id: Vec<i64>,
}

struct Bot {
    pub token: String,
    pub config: Config,
}

impl Bot {
    fn new(token: String, config: String) -> Result<Self> {
        let config: Config =
            toml::from_str(&config).map_err(|e| Error::RustError(e.to_string()))?;

        Ok(Self { token, config })
    }

    async fn send_json_request<T: TMethod>(&self, _: T, method: Method) -> Result<Response> {
        let mut request_builder = RequestInit::new();
        request_builder.with_method(method);

        Fetch::Request(Request::new_with_init(
            &T::url(&self.token),
            &request_builder,
        )?)
        .send()
        .await
    }

    async fn get_me(&self) -> Result<User> {
        let mut result = self.send_json_request(GetMe, Method::Get).await?;
        result
            .json::<TelegramResult<User>>()
            .await?
            .into_result()
            .map_err(|e| Error::RustError(e.description))
    }

    async fn set_webhook(&self, url: String) -> Result<()> {
        let payload = DeleteWebhook;
        self.send_json_request(payload, Method::Post).await?;

        let mut payload = SetWebhook::new(url);
        // only allow message types
        payload.allowed_updates = Some(Cow::from(&[UpdateTypes::Message]));
        self.send_json_request(payload, Method::Post).await?;

        Ok(())
    }

    fn reply(&self, msg: &Message, text: &str) -> Result<Response> {
        Response::from_json(&WebhookReply::from(
            SendMessage::new(ChatTarget::Id(msg.chat.id), text).reply(msg.message_id),
        ))
    }

    fn process(&self, msg: &Message) -> Result<Response> {
        let text = format!("{:?}", msg);
        self.reply(msg, &text)
    }
}

const TOKEN: &str = "TOKEN";
const CONFIG: &str = "CONFIG";
const BUCKET: &str = "WiseAss";

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let kv = env.kv(BUCKET)?;

    let token = env
        .secret(TOKEN)
        // should panic in case no token is provided
        .expect("TOKEN env variable can't be empty")
        .to_string();
    let config = kv
        .get(CONFIG)
        .text()
        .await?
        .expect("config is not provided");

    let bot = Bot::new(token, config).expect("could not initialize the bot");

    // webhook
    let router = Router::with_data(bot).get_async("/", |req, ctx| async move {
        let bot = ctx.data;
        bot.set_webhook(req.url()?.to_string()).await?;
        Response::from_json(&bot.get_me().await?)
    });

    // message handler
    let router = router.post_async("/updates", |mut req, ctx| async move {
        let update = req.json::<Update>().await?;

        if let Some(UpdateContent::Message(m)) = update.content {
            let bot = ctx.data;
            if let Some(sender) = m.from.clone() {
                // if bot.config.allowed_users_id.contains(&sender.id.0) {
                return bot.process(&m);
                // }
            }
        }

        Response::empty()
    });

    router.run(req, env).await
}
