SHELL=/bin/bash

include .env

.PHONY: help
help: ## display help section
	@ cat $(MAKEFILE_LIST) | grep -e "^[a-zA-Z_\-]*: *.*## *" | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: deploy
deploy: prepare ## deploy to cf workers
	@ npx wrangler deploy --var BUCKET:$(KV_BINDING)

.PHONY: config-upload
config-upload: ## upload config to kv storage. "name" arg should be provided
	@ echo $(TOKEN) | npx wrangler secret put TOKEN
	@ npx wrangler kv:key put CONFIG --path=$(name) --namespace-id=$(KV_ID)

.PHONY: prepare
prepare: ## prepare developing environment
	@ sed -i -e "s/KV_BINDING/$(KV_BINDING)/" -e "s/KV_ID/$(KV_ID)/" wrangler.toml
