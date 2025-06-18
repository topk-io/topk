VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

test:
    ARG region=dev
    BUILD +test-rs --region=$region
    BUILD +test-py --region=$region
    BUILD +test-js --region=$region

test-rs:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY . .

    WORKDIR /sdk/topk-rs

    DO rust+CARGO --args="nextest archive -p topk-rs --archive-file e2e.tar.zst" # compile tests

    ARG region=dev
    DO +SETUP_ENV --region=$region

    # test
    ENV FORCE_COLOR=1
    ARG args="--no-fail-fast -j 16"
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY cargo nextest run --archive-file e2e.tar.zst $args


test-py:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler python3.11-venv

    # setup maturin
    RUN cargo install maturin

    # setup python
    RUN python3 -m venv /venv \
        && . /venv/bin/activate \
        && pip install --upgrade pip \
        && pip install pytest pytest-xdist

    # install pyright
    RUN . /venv/bin/activate && pip install pyright[nodejs]

    # source code
    WORKDIR /sdk
    COPY . .

    WORKDIR /sdk/topk-py

    # type check
    RUN . /venv/bin/activate && pyright

    # build
    RUN --mount=type=cache,target=target \
        --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        . /venv/bin/activate && maturin develop

    ARG region=dev
    DO +SETUP_ENV --region=$region

    # test
    RUN --no-cache --secret TOPK_API_KEY \
        . /venv/bin/activate \
        && TOPK_API_KEY=$TOPK_API_KEY pytest -n auto --tb=long --durations=50 --color=yes

test-js:
    FROM node:20-slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler curl build-essential

    # install Rust
    RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    ENV PATH="/root/.cargo/bin:${PATH}"

    # Ensure yarn bins are in the PATH
    ENV PATH="/sdk/topk-js/node_modules/.bin:${PATH}"

    # copy source code
    WORKDIR /sdk
    COPY . .

    # build
    WORKDIR /sdk/topk-js
    ENV YARN_CACHE_FOLDER=/root/.yarn
    RUN --mount=type=cache,target=/root/.yarn yarn install

    RUN --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        yarn build

    ARG region=dev
    DO +SETUP_ENV --region=$region
    # test
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY yarn test --colors

#

test-runner-image:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    COPY . .

    WORKDIR /sdk/topk-rs

    ENV RUSTFLAGS="-C target-cpu=generic"
    DO rust+CARGO --args="nextest archive --release --archive-file test-runner.tar.zst"

    CMD ["cargo", "nextest", "run", "--archive-file", "test-runner.tar.zst", "--no-fail-fast", "-j", "16"]

    ARG tag=latest
    SAVE IMAGE --push ghcr.io/topk-io/topk-test-runner:$tag

test-runner:
    ARG tag=latest
    BUILD --platform linux/amd64 +test-runner-image --tag $tag
    BUILD --platform linux/arm64 +test-runner-image --tag $tag

#

SETUP_ENV:
    FUNCTION

    # region
    ARG region=dev
    ENV TOPK_REGION=$region

    # setup dev environment
    IF [ "$region" = "dev" ]
        # forward traffic to dev cluster running on host
        LET host_ip=$(getent hosts host.docker.internal | awk '{ print $1 }')
        HOST dev.api.ddb $host_ip
        ENV TOPK_HOST=ddb
        ENV TOPK_HTTPS=false
    END
