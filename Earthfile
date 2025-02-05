VERSION 0.8

test:
    LOCALLY
    RUN cargo install maturin

    WORKDIR topk-py

    RUN python3 -m venv .venv \
        && . .venv/bin/activate \
        && pip install pytest \
        && maturin develop \
        && pytest
