mod bot;

use bot::Bot;
use worker::*;
use telegram_types::bot::types::{Update, UpdateContent};

const TOKEN: &str = "TOKEN";
const CONFIG: &str = "CONFIG";
const BUCKET: &str = "BUCKET";

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let bucket = env.var(BUCKET)
        .expect("BUCKET env variable can't be empty");
    let kv = env.kv(&bucket.to_string())?;

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
        let ip = req.headers().get("cf-connecting-ip");
        Response::ok(format!("ip: {:?}", ip))
    });

    // message handler
    let router = router.post_async("/updates", |mut req, ctx| async move {
        let update = req.json::<Update>().await?;

        if let Some(UpdateContent::Message(m)) = update.content {
            // reject other ips rather than telegram_types

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
