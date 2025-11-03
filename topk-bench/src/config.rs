use config::ConfigError;
use serde::de::DeserializeOwned;
use tracing::error;

pub trait LoadConfig
where
    Self: Sized,
{
    fn load_config() -> Result<Self, ConfigError>;
}

impl<T: DeserializeOwned> LoadConfig for T {
    fn load_config() -> Result<Self, ConfigError> {
        let config = config::Config::builder()
            .add_source(config::Environment::default())
            .build();

        match config {
            Ok(config) => match config.try_deserialize() {
                Ok(config) => Ok(config),
                Err(e) => {
                    error!(?e, "Failed to deserialize config: {:#?}", e);
                    Err(e)
                }
            },
            Err(e) => {
                error!(?e, "Failed to build config");
                Err(e)
            }
        }
    }
}
