from topk_sdk import Client, AsyncClient, RetryConfig, BackoffConfig


def test_client_construction_no_retry_config():
    # No retry_config provided
    client = Client(api_key="test_key", region="us-east-1")
    assert isinstance(client, Client)

    async_client = AsyncClient(api_key="test_key", region="us-east-1")
    assert isinstance(async_client, AsyncClient)


def test_client_construction_explicit_none_retry_config():
    # Explicit retry_config=None
    client_none = Client(api_key="test_key", region="us-east-1", retry_config=None)
    assert isinstance(client_none, Client)

    async_client_none = AsyncClient(api_key="test_key", region="us-east-1", retry_config=None)
    assert isinstance(async_client_none, AsyncClient)


def test_client_construction_retry_config_as_dict():
    # retry_config as dict (with backoff as dict)
    retry_dict = {
        "max_retries": 5,
        "timeout": 30000,  # milliseconds
        "backoff": {
            "base": 2,
            "init_backoff": 100,
            "max_backoff": 5000,
        },
    }

    client_dict = Client(api_key="test_key", region="us-east-1", retry_config=retry_dict)
    assert isinstance(client_dict, Client)

    async_client_dict = AsyncClient(api_key="test_key", region="us-east-1", retry_config=retry_dict)
    assert isinstance(async_client_dict, AsyncClient)


def test_client_construction_retry_config_as_instance_with_backoff_instance():
    # retry_config as instance (with backoff as instance)
    backoff_instance = BackoffConfig(base=2, init_backoff=100, max_backoff=5000)
    retry_config_instance = RetryConfig(max_retries=5, timeout=30000, backoff=backoff_instance)

    client_instance = Client(api_key="test_key", region="us-east-1", retry_config=retry_config_instance)
    assert isinstance(client_instance, Client)

    async_client_instance = AsyncClient(api_key="test_key", region="us-east-1", retry_config=retry_config_instance)
    assert isinstance(async_client_instance, AsyncClient)


def test_client_construction_retry_config_as_instance_with_backoff_dict():
    # retry_config as instance (with backoff as dict)
    backoff_dict = {
        "base": 2,
        "init_backoff": 100,
        "max_backoff": 5000,
    }

    # This should fail with TypeError: argument 'backoff': 'dict' object cannot be converted to 'BackoffConfig'
    try:
        retry_config_dict = RetryConfig(max_retries=5, timeout=30000, backoff=backoff_dict)  # type: ignore
        assert False, "Expected RetryConfig construction to fail"
    except TypeError as e:
        assert "argument 'backoff': 'dict' object cannot be converted to 'BackoffConfig'" in str(e)
