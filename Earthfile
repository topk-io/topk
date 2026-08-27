VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

test:
    ARG --required region
    ARG --required host
    BUILD +test-rs --region=$region --host=$host
    BUILD +test-py --region=$region --host=$host
    BUILD +test-js --region=$region --host=$host
    BUILD +test-cli --region=$region --host=$host
    BUILD +test-sql --region=$region --host=$host
    BUILD +test-es --region=$region --host=$host

test-rs:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true

    # copy source code
    WORKDIR /sdk
    ARG EARTHLY_GIT_HASH
    COPY --keep-ts . .

    WORKDIR /sdk/topk-rs

    DO rust+CARGO --args="nextest archive -p topk-rs --archive-file e2e.tar.zst" # compile tests

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ARG args=""
    ENV FORCE_COLOR=1
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file e2e.tar.zst --no-fail-fast -j 16 $args


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
    ARG EARTHLY_GIT_HASH
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
    ARG EARTHLY_GIT_HASH
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
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true

    # copy source code
    WORKDIR /sdk
    ARG EARTHLY_GIT_HASH
    COPY --keep-ts . .

    WORKDIR /sdk/topk-cli

    RUN --mount=type=cache,target=/root/.cargo/registry \
        --mount=type=cache,target=/root/.cargo/git \
        cargo nextest run -p topk-cli --no-run

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ENV FORCE_COLOR=1
    ARG args=""
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run -p topk-cli --no-fail-fast $args

test-sql:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true

    # copy source code
    WORKDIR /sdk
    ARG EARTHLY_GIT_HASH
    COPY --keep-ts . .

    WORKDIR /sdk/topk-sql

    DO rust+CARGO --args="nextest archive -p topk-sql --archive-file sql.tar.zst" # compile tests

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ARG args=""
    ENV FORCE_COLOR=1
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file sql.tar.zst --no-fail-fast -j 16 $args

#

test-es:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true

    # copy source code
    WORKDIR /sdk
    ARG EARTHLY_GIT_HASH
    COPY --keep-ts . .

    WORKDIR /sdk/topk-es

    DO rust+CARGO --args="nextest archive -p topk-es --archive-file es.tar.zst" # compile tests

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ARG args=""
    ENV FORCE_COLOR=1
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file es.tar.zst --no-fail-fast -j 16 $args

#

test-runner-builder:
    FROM rust:slim

    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked

    DO rust+INIT --keep_fingerprints=true

    WORKDIR /sdk
    ARG EARTHLY_GIT_HASH
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
        LET domain=ddb
        HOST ${region}.api.${domain} $host
        HOST ${region}.es.${domain} $host
        HOST ${region}.sql.${domain} $host
        ENV TOPK_HOST=$domain
        ENV TOPK_HTTPS=false
    END
