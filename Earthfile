VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

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

    # copy source code
    WORKDIR /sdk
    COPY . .

    # build
    WORKDIR /sdk/topk-py
    RUN --mount=type=cache,target=target \
        --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        . /venv/bin/activate && maturin develop

    ARG region=dev
    DO +SETUP_ENV --region=$region

    # test
    RUN --no-cache --secret TOPK_API_KEY \
        . /venv/bin/activate \
        && TOPK_API_KEY=$TOPK_API_KEY pytest -n auto --tb=short

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
    RUN --mount=type=cache,target=/usr/local/share/.cache/yarn/v6 \
        yarn install

    RUN --mount=type=cache,target=target \
        --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        yarn build

    ARG region=dev
    DO +SETUP_ENV --region=$region
    # test
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY yarn test

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
