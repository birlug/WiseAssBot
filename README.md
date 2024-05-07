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
