mod bot;
mod challenge;
mod commands;
mod config;
mod telegram;

use std::net::IpAddr;

use bot::Bot;
use include_dir::{include_dir, Dir};
use telegram_types::bot::types::Update;
use worker::*;

const TOKEN: &str = "TOKEN";
const CONFIG: &str = "CONFIG";
const BUCKET: &str = "BUCKET";

static RESPONSE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/response");

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let mut bot = init_bot(&env).await.expect("could not initialize the bot");

    // commands
    let ignored_commands = ["report", "join", "easter-egg"];
    RESPONSE_DIR.files().for_each(|f| {
        let k = f.path().to_str().unwrap(); // safe to unwrap
        let r = f.contents_utf8().unwrap(); // safe to unwrap
        if !ignored_commands.contains(&k) {
            bot.commands
                .insert(format!("!{}", k), Box::new(|b, m| b.reply(m, r)));
        }
    });
    bot.commands
        .insert("!report".to_string(), Box::new(commands::report));
    bot.commands
        .insert("!share".to_string(), Box::new(commands::share));

    // message handler
    let router = Router::with_data(bot).post_async("/updates", |mut req, ctx| async move {
        let bot = ctx.data;

        // reject ip addresses other than telegram
        let ip = req
            .headers()
            .get("cf-connecting-ip")?
            .map(|ip| ip.parse::<IpAddr>())
            .unwrap() // safe to unwrap
            .map_err(|e| Error::RustError(e.to_string()))?;
        if bot
            .config
            .routes
            .allowed_ip
            .iter()
            .any(|cidr| cidr.contains(&ip))
        {
            let update = req.json::<Update>().await?;
            return bot.process(&update).await;
        }

        Response::empty()
    });

    router.run(req, env).await
}

#[event(scheduled)]
async fn remove_join_messages(_ev: ScheduledEvent, env: Env, _: ScheduleContext) {
    if let Ok(bot) = init_bot(&env).await {
        let _ = bot.remove_expired_join_requests().await;
    }
}

async fn init_bot(env: &Env) -> Result<Bot> {
    let bucket = env.var(BUCKET).expect("BUCKET env variable can't be empty");
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

    Bot::new(token, config, kv)
}
