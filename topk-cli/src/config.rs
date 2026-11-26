//! The CLI's configuration: which API to talk to, the login session, and the
//! access tokens minted with it.

use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::auth::{OAuthArgs, OAuthConfig, Session};
use crate::client::AccessToken;
use crate::host::{Host, HostArgs, DEFAULT_HOST};
use crate::profile::{Profile, ProfileName, Profiles};
use crate::ProjectId;

pub use crate::storage::{config_dir, lock, read, read_toml, write_secret_file, write_toml};

#[derive(clap::Args)]
pub struct Args {
    /// Use a named profile
    #[arg(long, env = "TOPK_PROFILE", global = true, hide = true)]
    pub profile: Option<ProfileName>,

    #[command(flatten)]
    pub host: HostArgs,

    #[command(flatten)]
    pub oauth: OAuthArgs,
}

#[derive(Clone)]
pub struct Config {
    host: Host,
    oauth: OAuthConfig,
    profile: Profile,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Defaults {
    pub project_id: Option<ProjectId>,
    pub region: Option<String>,
}

impl Config {
    pub fn new(host: Host, oauth: OAuthConfig, dir: PathBuf) -> Self {
        Self {
            host,
            oauth,
            profile: Profile::new(dir, ProfileName::default()),
        }
    }

    pub async fn load(args: &Args, profiles: &Profiles) -> Result<Self> {
        let profile = profiles.get(match &args.profile {
            Some(name) => name.clone(),
            None => profiles.selected()?,
        });
        // The host a profile is logged in to applies when none is given.
        let host = match (&args.host.host, profile.host()?) {
            (None, Some(saved)) => Host {
                https: args.host.https.unwrap_or(saved.https),
                ..saved
            },
            _ => args.host.clone().into(),
        };
        let oauth = match args.oauth.clone().config() {
            Some(oauth) => oauth,
            None => match Self::resolve_oauth(&host, &profile.dir()).await {
                Ok(oauth) => oauth,
                Err(e) => {
                    warn!("cannot load login settings for {}: {e:#}", host.host);
                    OAuthConfig::default()
                }
            },
        };
        Ok(Self {
            host,
            oauth,
            profile,
        })
    }

    pub fn save_profile(&self) -> Result<()> {
        self.profile.save(&self.host)
    }

    async fn resolve_oauth(host: &Host, dir: &Path) -> Result<OAuthConfig> {
        let file = oauth_file(dir, host);
        match read_toml(&file)? {
            Some(cached) => Ok(cached),
            None => {
                let config = OAuthConfig::resolve(host).await?;
                match host.host.as_str() {
                    // Using the default host, don't cache the oauth config.
                    DEFAULT_HOST => {}
                    _ => write_toml(&file, &config)?,
                }
                Ok(config)
            }
        }
    }

    pub async fn logout(&self) -> Result<()> {
        self.session().await?.delete()?;
        self.access_tokens().clear()?;
        remove(&oauth_file(&self.profile.dir(), &self.host))?;
        self.profile.forget()
    }

    pub async fn session(&self) -> Result<LockedSession<'_>> {
        Ok(LockedSession {
            config: self,
            _lock: lock(self.session_lock_file()).await?,
        })
    }

    pub fn access_tokens(&self) -> AccessTokens {
        AccessTokens {
            dir: self.tenant_dir().join("projects"),
        }
    }

    pub fn import_dir(&self) -> PathBuf {
        self.profile.dir().join("import")
    }

    pub fn host(&self) -> &str {
        &self.host.host
    }

    pub fn hostname(&self) -> &str {
        self.host
            .host
            .rsplit_once(':')
            .map_or(&self.host.host, |(name, _)| name)
    }

    pub fn port(&self) -> Option<u16> {
        self.host
            .host
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse().ok())
    }

    pub fn https(&self) -> bool {
        self.host.https
    }

    pub fn api_url(&self) -> String {
        self.host.api_url()
    }

    pub fn oauth(&self) -> &OAuthConfig {
        &self.oauth
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn defaults(&self) -> Result<Defaults> {
        Ok(read_toml(&self.defaults_file())
            .context("invalid saved defaults")?
            .unwrap_or_default())
    }

    pub fn set_project_id(&self, project_id: Option<ProjectId>) -> Result<()> {
        write_toml(
            &self.defaults_file(),
            &Defaults {
                project_id,
                ..self.defaults()?
            },
        )
    }

    pub fn set_region(&self, region: Option<String>) -> Result<()> {
        write_toml(
            &self.defaults_file(),
            &Defaults {
                region,
                ..self.defaults()?
            },
        )
    }

    fn defaults_file(&self) -> PathBuf {
        self.tenant_dir()
            .join("defaults")
            .join(format!("{}.toml", self.host.key()))
    }

    /// Everything scoped to this issuer's session lives here.
    fn tenant_dir(&self) -> PathBuf {
        self.profile
            .dir()
            .join("tenants")
            .join(self.oauth.issuer_key())
    }

    fn credentials_file(&self) -> PathBuf {
        self.tenant_dir().join("credentials.toml")
    }

    fn session_lock_file(&self) -> PathBuf {
        self.tenant_dir().join("session.lock")
    }
}

pub struct LockedSession<'a> {
    config: &'a Config,
    _lock: File,
}

impl LockedSession<'_> {
    pub fn load(&self) -> Result<Option<Session>> {
        let credentials: Option<Credentials> =
            read_toml(&self.config.credentials_file()).context("stored credentials are corrupt")?;
        credentials
            .map(|credentials| credentials.session(&self.config.oauth))
            .transpose()
    }

    pub fn save(&self, session: Session) -> Result<()> {
        write_toml(
            &self.config.credentials_file(),
            &Credentials::new(&self.config.oauth, session),
        )
    }

    pub fn delete(&self) -> Result<()> {
        remove(&self.config.credentials_file())
    }
}

/// The access tokens minted with this session, cached per project.
pub struct AccessTokens {
    dir: PathBuf,
}

impl AccessTokens {
    /// The project's cached token; a missing or corrupt file reads as `None`.
    pub fn load(&self, project_id: &str) -> Result<Option<AccessToken>> {
        Ok(read(&self.token_file(project_id))?.and_then(|raw| toml::from_str(&raw).ok()))
    }

    /// Wait for the project's token lock.
    pub async fn lock(&self, project_id: &str) -> Result<File> {
        lock(self.lock_file(project_id)).await
    }

    pub fn save(&self, project_id: &str, token: &AccessToken) -> Result<()> {
        write_toml(&self.token_file(project_id), token)
    }

    pub fn clear(&self) -> Result<()> {
        match std::fs::remove_dir_all(self.tokens_dir()) {
            Err(error) if error.kind() != ErrorKind::NotFound => {
                Err(error).context("clearing access token cache")
            }
            _ => Ok(()),
        }
    }

    fn tokens_dir(&self) -> PathBuf {
        self.dir.join("tokens")
    }

    fn token_file(&self, project_id: &str) -> PathBuf {
        self.tokens_dir().join(format!("{project_id}.toml"))
    }

    fn lock_file(&self, project_id: &str) -> PathBuf {
        self.dir.join("locks").join(format!("{project_id}.lock"))
    }
}

#[derive(Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    audience: String,
    #[serde(flatten)]
    session: Session,
}

impl Credentials {
    fn new(oauth: &OAuthConfig, session: Session) -> Self {
        Self {
            client_id: oauth.client_id.clone(),
            audience: oauth.audience.clone(),
            session,
        }
    }

    fn session(self, oauth: &OAuthConfig) -> Result<Session> {
        ensure!(
            self.client_id == oauth.client_id && self.audience == oauth.audience,
            "authentication configuration mismatch"
        );
        Ok(self.session)
    }
}

fn oauth_file(dir: &Path, host: &Host) -> PathBuf {
    dir.join("hosts").join(format!("{}.toml", host.key()))
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}
