use topk_rs::{Client, ClientConfig};

pub fn make_client(api_key: &str, region: &str, host: &str, https: bool) -> Client {
    Client::new(
        ClientConfig::new(api_key, region)
            .with_host(host)
            .with_https(https),
    )
}

pub fn make_global_client(api_key: &str, host: &str, https: bool) -> Client {
    make_client(api_key, "global", host, https)
}
