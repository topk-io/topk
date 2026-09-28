use anyhow::{Context, Result};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::postgres::{PgConnectOptions, PgConnection, PgQueryResult, PgRow, PgSslMode};
use sqlx::{Connection, Either};

use crate::client::{AccessTokenInterceptor, ManagementClient};
use crate::config::Config;
use crate::endpoint::{Credentials, DataEndpoint};

const PORT: u16 = 5432;
const USER: &str = "topk";
const DATABASE: &str = "topk";
const APPLICATION_NAME: &str = "topk-cli";

/// A connection to the regional PostgreSQL wire protocol endpoint.
pub struct SqlClient {
    connection: PgConnection,
}

impl SqlClient {
    pub async fn connect(config: &Config, endpoint: &DataEndpoint) -> Result<Self> {
        let region = endpoint.region.as_deref().context(
            "--region is required (or set TOPK_REGION). \
             List available regions at https://docs.topk.io/regions",
        )?;
        let host = format!("{region}.sql.{}", config.host().host);
        let password = match endpoint.credentials()? {
            Credentials::ApiKey(api_key) => api_key,
            Credentials::Project(project_id) => {
                AccessTokenInterceptor::new(
                    ManagementClient::new(config.clone())?,
                    config.clone(),
                    project_id,
                )
                .token()
                .await?
                .token
            }
        };
        let options = PgConnectOptions::new_without_pgpass()
            .host(&host)
            .port(PORT)
            .username(USER)
            .password(&password)
            .database(DATABASE)
            .application_name(APPLICATION_NAME)
            .ssl_mode(match config.host().https {
                // Authenticate the endpoint before sending the API key or access token.
                true => PgSslMode::VerifyFull,
                // Disable SSL for non https connections.
                false => PgSslMode::Disable,
            });
        PgConnection::connect_with(&options)
            .await
            .map(|connection| Self { connection })
            .with_context(|| format!("connecting to {host}:{PORT}"))
    }

    /// Run one or more statements; each statement's rows precede its result.
    pub fn execute<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxStream<'a, Result<Either<PgQueryResult, PgRow>>> {
        sqlx::raw_sql(sql)
            .fetch_many(&mut self.connection)
            .map(|item| Ok(item?))
            .boxed()
    }
}
