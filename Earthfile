VERSION 0.8
IMPORT github.com/earthly/lib/rust:3.0.1 AS rust

# shared base: nextest as a prebuilt binary. `cargo install cargo-nextest`
# compiles it from source in every test target, ~2 min each, six times a run.
rust-base:
    FROM rust:slim
    ARG nextest_version=0.9.143
    RUN apt-get update && apt-get install -y curl && \
        arch=$(uname -m) && \
        curl -fsSL https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${nextest_version}/cargo-nextest-${nextest_version}-${arch}-unknown-linux-gnu.tar.gz \
        | tar -xz -C /usr/local/cargo/bin cargo-nextest

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
    FROM +rust-base

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
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
    FROM +rust-base

    # docker for WITH DOCKER; g++/cmake for bundled duckdb
    RUN apt-get update && apt-get install -y protobuf-compiler jq curl docker.io pkg-config libssl-dev g++ cmake
    ARG compose_version=v5.5.0
    RUN mkdir -p /usr/local/lib/docker/cli-plugins && \
        curl -fsSL https://github.com/docker/compose/releases/download/${compose_version}/docker-compose-linux-$(uname -m) \
        -o /usr/local/lib/docker/cli-plugins/docker-compose && \
        chmod +x /usr/local/lib/docker/cli-plugins/docker-compose

    # sccache for rustc and the bundled duckdb C++ (cc-rs honors CC/CXX wrappers);
    # the ARGs come from CI, locally they are empty and sccache stays off.
    ARG sccache_version=0.17.0
    RUN arch=$(uname -m) && \
        curl -fsSL https://github.com/mozilla/sccache/releases/download/v${sccache_version}/sccache-v${sccache_version}-${arch}-unknown-linux-musl.tar.gz \
        | tar -xz -C /usr/local/bin --strip-components=1 --wildcards '*/sccache'
    # v2 cache service only: v1 is gone and sccache prefers it when both are set.
    ARG ACTIONS_RESULTS_URL=""
    ARG ACTIONS_RUNTIME_TOKEN=""
    IF [ -n "$ACTIONS_RESULTS_URL" ]
        ENV ACTIONS_RESULTS_URL=$ACTIONS_RESULTS_URL
        ENV ACTIONS_RUNTIME_TOKEN=$ACTIONS_RUNTIME_TOKEN
        ENV ACTIONS_CACHE_SERVICE_V2=on
        ENV SCCACHE_GHA_ENABLED=true
        ENV RUSTC_WRAPPER=sccache
        ENV CC="sccache cc"
        ENV CXX="sccache c++"
    END
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-cli

    ARG EARTHLY_GIT_HASH
    # a real layer, not rust+CARGO's cache mount: WITH DOCKER below cannot read that.
    RUN --mount=type=cache,target=/root/.cargo/registry \
        --mount=type=cache,target=/root/.cargo/git \
        cargo nextest run -p topk-cli --no-run && sccache --show-stats

    ARG --required region
    ARG --required host
    DO +SETUP_ENV --region=$region --host=$host

    # test — CARGO_BIN_EXE_topk must be set explicitly since tests live in
    # #[cfg(test)] modules (not integration tests) and Cargo won't set it automatically
    ENV FORCE_COLOR=1
    ENV CARGO_BIN_EXE_topk=/sdk/topk-cli/target/debug/topk
    ARG args=""
    # `--compose` does not wait on healthchecks; `up --wait` does.
    WITH DOCKER --compose docker-compose.import.yaml
        RUN --no-cache --secret TOPK_API_KEY \
            (docker compose -f docker-compose.import.yaml up -d --wait || \
             (docker compose -f docker-compose.import.yaml ps; \
              docker inspect --format '{{json .State.Health}}' default-mysql-1; \
              docker compose -f docker-compose.import.yaml logs --tail 50; exit 1)) && \
            TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run -p topk-cli --no-fail-fast $args
    END

test-sql:
    FROM +rust-base

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
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
    # SETUP_ENV is shared by every test target; only these tests speak pgwire
    # to a remote cluster, and only they should refuse a plaintext fallback.
    ENV PGSSLMODE=require

    # test
    ARG args=""
    ENV FORCE_COLOR=1
    RUN --no-cache --secret TOPK_API_KEY \
        TOPK_API_KEY=$TOPK_API_KEY topk-test-sandbox cargo nextest run --archive-file sql.tar.zst --no-fail-fast -j 16 $args

#

test-es:
    FROM +rust-base

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler
    COPY +test-sandbox/topk-test-sandbox /usr/local/bin/topk-test-sandbox

    DO rust+INIT --keep_fingerprints=true
    WORKDIR /sdk

    # copy source code
    COPY --keep-ts . .

    WORKDIR /sdk/topk-es

    ARG EARTHLY_GIT_HASH
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
    FROM +rust-base

    RUN apt-get update && apt-get install -y protobuf-compiler

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
        HOST emulator.es.ddb $host
        HOST emulator.sql.ddb $host
        ENV TOPK_HOST=ddb
        ENV TOPK_HTTPS=false

        # emulator gateway is plaintext: es on :9200, pgwire on :5432
        ENV ES_URL=http://emulator.es.ddb:9200
        ENV PGHOST=emulator.sql.ddb
        ENV PGSSLMODE=disable
    ELSE
        ENV ES_URL=https://${region}.es.${host}
        ENV PGHOST=${region}.sql.${host}
    END
