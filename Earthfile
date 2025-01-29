VERSION 0.8

py:
    ARG --required version
    BUILD +py-macos --version $version
    BUILD +py-linux --version $version

py-macos:
    ARG --required version
    ARG mode=test

    WAIT
        BUILD +py-macos-3.13
        BUILD +py-macos-3.12
        BUILD +py-macos-3.11
        BUILD +py-macos-3.10
        BUILD +py-macos-3.9
    END

    FROM +py-builder

    WORKDIR /wheels
    COPY topk-py/wheels/topk_sdk-${version}-cp313-cp313-macosx_11_0_arm64.whl .
    COPY topk-py/wheels/topk_sdk-${version}-cp312-cp312-macosx_11_0_arm64.whl .
    COPY topk-py/wheels/topk_sdk-${version}-cp311-cp311-macosx_11_0_arm64.whl .
    COPY topk-py/wheels/topk_sdk-${version}-cp310-cp310-macosx_11_0_arm64.whl .
    COPY topk-py/wheels/topk_sdk-${version}-cp39-cp39-macosx_11_0_arm64.whl .

    IF $mode == "test"
        RUN --push --secret test_pypi_token MATURIN_PYPI_TOKEN=$test_pypi_token maturin upload -r testpypi $(ls topk_sdk-*.whl)
    ELSE
        RUN --push --secret pypi_token MATURIN_PYPI_TOKEN=$pypi_token maturin upload -r pypi $(ls topk_sdk-*.whl)
    END

py-linux:
    ARG --required version
    ARG mode=test

    WAIT
        BUILD +py-linux-3.13
        BUILD +py-linux-3.12
        BUILD +py-linux-3.11
        BUILD +py-linux-3.10
        BUILD +py-linux-3.9
    END

    FROM +py-builder

    WORKDIR /wheels
    COPY +py-linux-3.13/topk_sdk-${version}-cp313-cp313-manylinux_2_34_aarch64.whl .
    COPY +py-linux-3.12/topk_sdk-${version}-cp312-cp312-manylinux_2_34_aarch64.whl .
    COPY +py-linux-3.11/topk_sdk-${version}-cp311-cp311-manylinux_2_34_aarch64.whl .
    COPY +py-linux-3.10/topk_sdk-${version}-cp310-cp310-manylinux_2_34_aarch64.whl .
    COPY +py-linux-3.9/topk_sdk-${version}-cp39-cp39-manylinux_2_34_aarch64.whl .

    IF $mode == "test"
        RUN --push --secret test_pypi_token MATURIN_PYPI_TOKEN=$test_pypi_token maturin upload -r testpypi $(ls topk_sdk-*.whl)
    ELSE
        RUN --push --secret pypi_token MATURIN_PYPI_TOKEN=$pypi_token maturin upload -r pypi $(ls topk_sdk-*.whl)
    END

py-linux-3.13:
    DO +PYTHON_SDK_LINUX --python 3.13

py-linux-3.12:
    DO +PYTHON_SDK_LINUX --python 3.12

py-linux-3.11:
    DO +PYTHON_SDK_LINUX --python 3.11

py-linux-3.10:
    DO +PYTHON_SDK_LINUX --python 3.10

py-linux-3.9:
    DO +PYTHON_SDK_LINUX --python 3.9

py-macos-3.13:
    DO +PYTHON_SDK_DARWIN --python 3.13

py-macos-3.12:
    DO +PYTHON_SDK_DARWIN --python 3.12

py-macos-3.11:
    DO +PYTHON_SDK_DARWIN --python 3.11

py-macos-3.10:
    DO +PYTHON_SDK_DARWIN --python 3.10

py-macos-3.9:
    DO +PYTHON_SDK_DARWIN --python 3.9

# helpers

py-init:
    LOCALLY
    WORKDIR topk-py
    RUN uv venv
    RUN uv pip install maturin pytest

py-test:
    ARG args=""
    LOCALLY
    WORKDIR topk-py
    RUN uv run maturin develop --uv
    RUN TOPK_API_KEY=$(ddb-ctl cps auth create-test-project) .venv/bin/python -m pytest $args


py-builder:
    FROM python:3.13-slim

    # install build deps
    DO +APT_INSTALL --pkgs "curl build-essential protobuf-compiler"

    # install rust
    RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
    ENV PATH="/root/.cargo/bin:${PATH}"

    # install uv
    RUN curl -LsSf https://astral.sh/uv/install.sh | sh

    # install python deps
    RUN python -m venv .venv && . .venv/bin/activate
    RUN pip install maturin


PYTHON_SDK_DARWIN:
    FUNCTION
    ARG --required python
    LOCALLY
    WORKDIR topk-py
    RUN uvx maturin build --release --interpreter $python --out wheels

PYTHON_SDK_LINUX:
    FUNCTION
    ARG --required python
    FROM +py-builder

    COPY . /sdk
    WORKDIR /sdk/topk-py

    # build wheels (TODO: compiler cache is not saved)
    RUN maturin build --release --interpreter $python --out .
    SAVE ARTIFACT $(ls topk_sdk-*.whl)

APT_INSTALL:
    FUNCTION

    ARG --required pkgs

    RUN apt-get update \
        && apt-get install -y $pkgs \
        && apt-get clean \
        && rm -rf /var/lib/apt/lists/*
