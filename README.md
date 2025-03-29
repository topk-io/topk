# TopK SDK

This repository contains TopK SDKs for different languages. Python is the only SDK that is currently supported, with Rust coming soon.

## Running tests

1. Install [Earthly](https://earthly.dev/get-earthly)
1. Run `python` tests
   ```bash
   earthly --secret TOPK_API_KEY +test-py
   ```

## Python development

1. Install [Python](https://www.python.org/downloads/)
1. Install [maturin](https://github.com/pyo3/maturin)
1. Install [pytest](https://github.com/pytest-dev/pytest)
1. `cd topk-py`
1. Build the sdk
   ```bash
   maturin develop
   ```
1. Run tests
   ```bash
   pytest
   ```

## Release

You can release a new version by creating [a new GitHub release](https://github.com/fafolabs/topk-sdk/releases) or creating a new tag:

```bash
git tag -a v1.1.5 -m "Release 1.1.5"
git push origin v1.1.5
```
