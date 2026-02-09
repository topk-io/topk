VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

test:
    ARG region=emulator
    BUILD +test-rs --region=$region
    BUILD +test-py --region=$region
    BUILD +test-js --region=$region

test-rs:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-rs

    DO rust+CARGO --args="nextest archive -p topk-rs --archive-file e2e.tar.zst" # compile tests

    ARG region=emulator
    ARG host
    DO +SETUP_ENV --region=$region --host=$host

    # test
    ENV FORCE_COLOR=1
    ARG args="--no-fail-fast -j 16"
    # TODO: remove filter once ask/handle tests are ready
    ARG filter=not test(/ask/) and not test(/handle/)
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY cargo nextest run --archive-file e2e.tar.zst $args -E "$filter"


test-py:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler python3-venv

    # setup maturin
    RUN cargo install maturin@1.9.0 --locked

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

    ARG region=emulator
    DO +SETUP_ENV --region=$region

    # test
    ARG args=""
    RUN --no-cache --secret TOPK_API_KEY \
        . /venv/bin/activate \
        && TOPK_API_KEY=$TOPK_API_KEY pytest -n auto --tb=long --durations=50 --color=yes -vv $args

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

    ARG region=emulator
    DO +SETUP_ENV --region=$region
    # test
    ARG args=""
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY yarn test --colors $args

#

test-runner:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    RUN cargo install cargo-nextest --locked

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    COPY --keep-ts . .

    WORKDIR /sdk/topk-rs

    ENV RUSTFLAGS="-C target-cpu=generic"
    ENV FORCE_COLOR=1
    DO rust+CARGO --args="nextest archive --release --archive-file test-runner.tar.zst"

    ENTRYPOINT ["cargo", "nextest", "run", "--archive-file", "test-runner.tar.zst"]

    ARG registry
    ARG tag=latest
    SAVE IMAGE --push $registry/topk-test-runner:$tag

#

SETUP_ENV:
    FUNCTION

    # region
    ARG host
    ARG region=emulator
    ENV TOPK_REGION=$region

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
