use aws_config::Region;
use aws_sdk_s3::{config::Credentials, Client, Config};
use serde::Deserialize;

use crate::config::LoadConfig;

#[derive(Debug, Deserialize)]
pub struct S3Settings {
    /// AWS access key ID   
    pub aws_access_key_id: String,
    /// AWS secret access key
    pub aws_secret_access_key: String,
    /// AWS region
    pub aws_region: String,
}

pub fn new_client() -> anyhow::Result<Client> {
    let settings = S3Settings::load_config()?;

    let creds = Credentials::new(
        settings.aws_access_key_id,
        settings.aws_secret_access_key,
        None,
        None,
        "topk-bench",
    );

    let endpoint_url = format!("https://s3.{}.amazonaws.com", settings.aws_region);

    let mut builder = Config::builder()
        .region(Region::new(settings.aws_region))
        .credentials_provider(creds)
        .endpoint_url(endpoint_url);

    // Disable the following warning: This checksum is a part-level checksum which can't be validated by the Rust SDK. Disable checksum validation for this request to fix this warning. more_info="See https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity.html#large-object-checksums for more information."
    builder.set_request_checksum_calculation(None);

    Ok(Client::from_conf(builder.build()))
}
