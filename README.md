# TopK SDK

This repository contains TopK SDKs for different languages. Python is the only SDK that is currently supported, with Rust coming soon.

## Development

1. Install [Earthly](https://earthly.dev/get-earthly)
2. Install [Python](https://www.python.org/downloads/)
3. Install [Rust](https://www.rust-lang.org/tools/install)
4. Initialize python dev environment
    ```bash
    earthly +py-init
    ```
5. Run python tests
    ```bash
    earthly +py-test
    ```
