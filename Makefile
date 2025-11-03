.PHONY: docs generate-docs

build:
	cd topk-py && maturin develop
	cd topk-js && yarn dev

docs: build
	cd topk-py && python3 docgen/main.py topk_sdk ../docs/sdk/topk-py
	cd topk-js && yarn docs

bench:
	$(eval tag=$(shell python3 -c "import secrets; print(secrets.token_urlsafe(6))"))
	depot build --platform linux/amd64,linux/arm64 -f Dockerfile.bench -t ttl.sh/topk-bench:$(tag) --push .
	echo $(tag) > .topk-bench-tag
