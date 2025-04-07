use napi_derive::napi;

#[napi(object)]
pub struct ClientConfig {
    pub api_key: String,
    pub region: String,
    pub host: Option<String>,
    pub https: Option<bool>,
}
