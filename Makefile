.PHONY: docs generate-docs

build:
	cd topk-py && maturin develop
	cd topk-js && yarn dev
	@echo "Documentation generation complete!"

docs: build
	cd topk-py && python3 docgen/main.py topk_sdk ../docs/sdk/topk-py
	cd topk-js && yarn docs
