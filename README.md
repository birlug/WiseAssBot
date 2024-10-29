<p align="center"><img src="./logo.png" width="250"></p>

# WiseAssBot
WiseAssBot (AKA MrKnowItAll) is a serverless telegram bot designed to manage the group with more ease.

## Quick Start
1. [Create an API token](https://developers.cloudflare.com/fundamentals/api/get-started/create-token/) from the cloudflare dashboard.
2. Create a `.env` file based on `.env.example` and fill the values based on your tokens

| Variable            | Description                                      |
|---------------------|--------------------------------------------------|
| CLOUDFLARE_API_TOKEN | The API key retrieved from Cloudflare dashboard |
| TOKEN               | Telegram bot token                               |
| KV_BINDING          | KV namespace binding                             |
| KV_ID               | KV namespace ID                                  |

Read more about KV bindings [here](https://developers.cloudflare.com/kv/reference/kv-bindings/).

3. Prepare your developing environment by:
```sh
$ make prepare
```

4. Upload the config on KV storage

**NOTE: Skip this step if you uploaded your config before**
```sh
$ make config-upload
```

5. Deploy
```sh
$ make deploy
```

## Run Locally
To run the bot locally, first:
1. you need to write a local configuration file by using:
```sh
$ make dev-config name=config-example.toml
 ```
2.  to run wiseass locally:
```sh
$ make dev
```
3. install [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-local-tunnel/) and run the following command to expose your local wiseass instance to the internet.

```sh
$ cloudflared tunnel --url http://localhost:8780
```

4. set a telegram webhook:
 ```
 https://api.telegram.org/bot{YOUR_BOT_TOKEN}/setWebhook?url=https://{CLOUDFLARED_EXPOSED_BASE_URL}/updates 
```
