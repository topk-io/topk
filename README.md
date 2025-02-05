# TopK SDK

This repository contains TopK SDKs for different languages. Python is the only SDK that is currently supported, with Rust coming soon.

## Development

1. Install [Earthly](https://earthly.dev/get-earthly)
2. Install [Python](https://www.python.org/downloads/)
3. Install [Rust](https://www.rust-lang.org/tools/install)
4. Install [maturin](https://github.com/pyo3/maturin)
5. Run tests in development mode
   ```bash
   earthly +test
   ```

## Release

You can release a new version by creating [a new GitHub release](https://github.com/fafolabs/topk-sdk/releases) or creating a new tag:

```bash
git tag -a v1.1.5 -m "Release 1.1.5"
git push origin v1.1.5
```
