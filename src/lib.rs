mod bot;
mod commands;
mod config;

use std::net::IpAddr;

use bot::Bot;
use include_dir::{include_dir, Dir};
use telegram_types::bot::types::{Update, UpdateContent};
use worker::*;

const TOKEN: &str = "TOKEN";
const CONFIG: &str = "CONFIG";
const BUCKET: &str = "BUCKET";

static RESPONSE_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/response");

#[event(fetch)]
async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
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

    let mut bot = Bot::new(token, config).expect("could not initialize the bot");

    // commands
    RESPONSE_DIR.files().for_each(|f| {
        let k = f.path().to_str().unwrap(); // safe to unwrap
        let r = f.contents_utf8().unwrap(); // safe to unwrap
        bot.commands
            .insert(format!("!{}", k), Box::new(|b, m| b.reply(m, r)));
    });
    bot.commands
        .insert("!report".to_string(), Box::new(commands::report));

    // message handler
    let router = Router::with_data(bot).post_async("/updates", |mut req, ctx| async move {
        let update = req.json::<Update>().await?;

        if let Some(UpdateContent::Message(m)) = update.content {
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
                return bot.process(&m).await;
            }
        }

        Response::empty()
    });

    router.run(req, env).await
}
