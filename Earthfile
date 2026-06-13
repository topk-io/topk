VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

test:
    ARG --required region
    ARG --required host
    BUILD +test-rs --region=$region --host=$host
    BUILD +test-py --region=$region --host=$host
    BUILD +test-js --region=$region --host=$host
    BUILD +test-sql --region=$region --host=$host

test-rs:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-rs

    ARG EARTHLY_GIT_HASH
    DO rust+CARGO --args="nextest archive -p topk-rs --archive-file e2e.tar.zst" # compile tests

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ENV FORCE_COLOR=1
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file e2e.tar.zst --no-fail-fast -j 16


test-py:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler python3-venv
    RUN cargo install maturin@1.9.0 --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    # setup python
    RUN python3 -m venv /venv \
        && . /venv/bin/activate \
        && pip install --upgrade pip \
        && pip install pytest pytest-xdist pytest-asyncio patchelf

    # install pyright
    RUN . /venv/bin/activate && pip install pyright[nodejs]

    # install numpy
    RUN . /venv/bin/activate && pip install numpy

    # source code
    WORKDIR /sdk
    COPY . .

    WORKDIR /sdk/topk-py

    # type check
    RUN . /venv/bin/activate && pyright

    # build
    RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        . /venv/bin/activate && maturin develop

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ARG args=""
    RUN --no-cache --secret TOPK_API_KEY \
        . /venv/bin/activate \
        && TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox pytest -n auto --tb=long --durations=50 --color=yes -vv $args

test-js:
    FROM node:20-slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler curl build-essential

    # install Rust
    RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    ENV PATH="/root/.cargo/bin:${PATH}"

    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    # Ensure yarn bins are in the PATH
    ENV PATH="/sdk/topk-js/node_modules/.bin:${PATH}"

    # copy source code
    WORKDIR /sdk
    COPY . .

    # save contents of typescript index.d.ts file in an env variable before build
    ENV D_TS_FILE_CONTENTS=$(cat /sdk/topk-js/index.d.ts)

    # build
    WORKDIR /sdk/topk-js
    ENV YARN_CACHE_FOLDER=/root/.yarn
    RUN --mount=type=cache,target=/root/.yarn yarn install

    RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        yarn build && yarn typecheck

    # validate that the typescript definition index.d.ts file remains the same after the build
    RUN if [ "$D_TS_FILE_CONTENTS" != "$(cat /sdk/topk-js/index.d.ts)" ]; then \
        echo "❌ Typescript definition file changed after build" && \
        echo "Diff:"; \
        echo "$D_TS_FILE_CONTENTS" | diff - /sdk/topk-js/index.d.ts || true; \
        exit 1; \
    fi

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host
    # test
    ARG args=""
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox yarn test --colors $args

test-cli:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler jq
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-cli

    ARG EARTHLY_GIT_HASH
    DO rust+CARGO --args="test -p topk-cli --no-run" # compile tests (warms up registry cache)

    # build the binary as a real layer file so CARGO_BIN_EXE_topk can point to it
    # (DO rust+CARGO uses cache mounts that don't persist as regular files)
    RUN --mount=type=cache,target=/root/.cargo/registry \
        --mount=type=cache,target=/root/.cargo/git \
        cargo build -p topk-cli

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test — CARGO_BIN_EXE_topk must be set explicitly since tests live in
    # #[cfg(test)] modules (not integration tests) and Cargo won't set it automatically
    ENV FORCE_COLOR=1
    ENV CARGO_BIN_EXE_topk=/sdk/topk-cli/target/debug/topk
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo test -p topk-cli --lib --no-fail-fast

test-sql:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-sql

    ARG EARTHLY_GIT_HASH
    DO rust+CARGO --args="nextest archive -p topk-sql --archive-file sql.tar.zst" # compile tests

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ENV FORCE_COLOR=1
    ENV POSTGRES_HOST=${region}.sql.${host}
    ENV POSTGRES_PORT=5432
    ENV POSTGRES_SSL=require
    RUN --no-cache --secret TOPK_API_KEY \
        POSTGRES_PASSWORD=$TOPK_API_KEY TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file sql.tar.zst --no-fail-fast -j 16

#

test-runner-builder:
    FROM rust:slim

    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked

    WORKDIR /sdk
    DO rust+INIT --keep_fingerprints=true
    COPY --keep-ts . .

    WORKDIR /sdk/topk-rs
    ENV RUSTFLAGS="-C target-cpu=generic"
    ENV FORCE_COLOR=1
    DO rust+CARGO --args="nextest archive --release --archive-file test-runner.tar.zst"

    SAVE ARTIFACT test-runner.tar.zst
    SAVE ARTIFACT /usr/local/cargo/bin/cargo-nextest

test-runner:
    FROM rust:slim

    COPY +test-runner-builder/cargo-nextest /usr/local/bin/cargo-nextest
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox
    COPY +test-runner-builder/test-runner.tar.zst /test-runner.tar.zst

    COPY --dir . /sdk

    WORKDIR /sdk/topk-rs
    ENTRYPOINT ["topk-test-sandbox", "cargo-nextest", "nextest", "run", "--archive-file", "/test-runner.tar.zst", "--no-fail-fast", "-j", "16"]

    ARG --required registry
    ARG --required tag
    SAVE IMAGE --push $registry:$tag

test-sandbox:
    FROM oven/bun:latest

    COPY --dir utils/ /workspace

    WORKDIR /workspace
    RUN bun install --frozen-lockfile
    RUN bun build --compile --outfile topk-test-sandbox test-sandbox.ts
    SAVE ARTIFACT topk-test-sandbox

#

SETUP_ENV:
    FUNCTION

    # region
    ARG --required host
    ARG --required region
    ENV TOPK_REGION=$region
    ENV TOPK_HOST=$host

    # setup dev environment
    IF [ "$region" = "emulator" ]
        IF [ -z "$host" ]
            LET host=$(getent hosts host.docker.internal | awk '{ print $1 }')
        END

        # forward traffic to dev cluster running on host
        HOST emulator.api.ddb $host
        ENV TOPK_HOST=ddb
        ENV TOPK_HTTPS=false
    END
