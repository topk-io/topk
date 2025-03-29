VERSION 0.8

test-py:
    FROM rust:slim

    # install dependencies
    RUN apt-get update && apt-get install -y protobuf-compiler python3.11-venv

    # setup maturin
    RUN cargo install maturin

    WORKDIR /sdk

    # setup python
    RUN python3 -m venv .venv \
        && . .venv/bin/activate \
        && pip install --upgrade pip \
        && pip install pytest pytest-xdist

    # copy source code
    COPY . .

    # build
    WORKDIR /sdk/topk-py
    RUN --mount=type=cache,target=/sdk/topk-py/target \
        --mount=type=cache,target=/root/.cargo \
        --mount=type=cache,target=/usr/local/cargo/registry \
        --mount=type=cache,target=/usr/local/cargo/git \
        maturin develop

    # region
    ARG region=dev
    ENV TOPK_REGION=$region

    # setup dev environment
    IF [ "$region" = "dev" ]
        # forward traffic to dev cluster running on host
        LET host_ip=$(getent hosts host.docker.internal | awk '{ print $1 }')
        HOST dev.api.ddb $host_ip
        ENV TOPK_HOST=ddb
    END

    # test
    RUN --no-cache --secret TOPK_API_KEY \
        . /sdk/.venv/bin/activate \
        && TOPK_API_KEY=$TOPK_API_KEY pytest -n auto
