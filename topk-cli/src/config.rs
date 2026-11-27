//! The CLI's configuration: which API to talk to, the login session, and the
//! access tokens minted with it.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{ensure, Context, Error, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::auth::{OAuthArgs, OAuthConfig, Session};
use crate::client::AccessToken;
use crate::host::{Host, HostArgs};
use crate::ProjectId;

pub use crate::storage::{lock, read, read_toml, write_secret_file, write_toml};

const CONFIG_FILE: &str = "config.toml";
const DEFAULT_PROFILE: &str = "default";

#[derive(clap::Args)]
pub struct Args {
    /// Use a named profile
    #[arg(
        long,
        env = "TOPK_PROFILE",
        global = true,
        help_heading = "Global options"
    )]
    pub profile: Option<ProfileName>,

    #[command(flatten)]
    pub host: HostArgs,

    #[command(flatten)]
    pub oauth: OAuthArgs,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ProfileName(String);

impl Default for ProfileName {
    fn default() -> Self {
        Self(DEFAULT_PROFILE.to_owned())
    }
}

impl ProfileName {
    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join("profiles").join(&self.0)
    }

    pub fn label(&self) -> String {
        match *self == Self::default() {
            true => String::new(),
            false => format!(" ({} profile)", self.0.cyan().bold()),
        }
    }
}

impl FromStr for ProfileName {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self> {
        ensure!(
            name.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit())
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_'),
            "invalid profile name {name:?}: use lowercase letters, digits, `-` and `_`"
        );
        Ok(Self(name.to_owned()))
    }
}

impl TryFrom<String> for ProfileName {
    type Error = Error;

    fn try_from(name: String) -> Result<Self> {
        name.parse()
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Profile {
    #[serde(flatten)]
    pub host: Host,
    #[serde(flatten)]
    oauth: Option<OAuthConfig>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Profiles {
    selected: Option<ProfileName>,
    #[serde(default)]
    profiles: BTreeMap<ProfileName, Profile>,
}

impl Profiles {
    fn load(dir: &Path) -> Result<Self> {
        Ok(read_toml(&dir.join(CONFIG_FILE))?.unwrap_or_default())
    }

    /// The selected profile, or the default one when none is selected.
    pub fn selected(&self) -> ProfileName {
        self.selected.clone().unwrap_or_default()
    }

    pub fn get(&self, name: &ProfileName) -> Option<&Profile> {
        self.profiles.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = (&ProfileName, &Profile)> {
        self.profiles.iter()
    }

    fn remove(&mut self, name: &ProfileName) {
        self.profiles.remove(name);
        if self.selected.as_ref() == Some(name) {
            self.selected = None;
        }
    }

    fn is_empty(&self) -> bool {
        self.selected.is_none() && self.profiles.is_empty()
    }
}

#[derive(Clone)]
pub struct Config {
    host: Host,
    oauth: OAuthConfig,
    dir: PathBuf,
    active_profile: ProfileName,
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
            dir,
            active_profile: ProfileName::default(),
        }
    }

    pub async fn load(args: &Args, dir: PathBuf) -> Result<Self> {
        let profiles = Profiles::load(&dir)?;
        let active_profile = match &args.profile {
            Some(name) => name.clone(),
            None => profiles.selected(),
        };
        let profile = profiles.get(&active_profile);
        // The host a profile is logged in to applies when none is given.
        let host = match (&args.host.host, profile) {
            (None, Some(profile)) => Host {
                host: profile.host.host.clone(),
                https: args.host.https.unwrap_or(profile.host.https),
            },
            _ => args.host.clone().into(),
        };
        // The login settings saved with a profile apply to its host only.
        let logged_in = profile.filter(|profile| profile.host == host);
        let oauth = match args
            .oauth
            .clone()
            .config()
            .or_else(|| logged_in?.oauth.clone())
        {
            Some(oauth) => oauth,
            None => match OAuthConfig::resolve(&host).await {
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
            dir,
            active_profile,
        })
    }

    pub fn save_profile(&self) -> Result<()> {
        let mut profiles = self.profiles()?;
        let profile = Profile {
            host: self.host.clone(),
            oauth: match self.oauth == OAuthConfig::default() {
                true => None,
                false => Some(self.oauth.clone()),
            },
        };
        profiles
            .profiles
            .insert(self.active_profile.clone(), profile);
        write_toml(&self.config_file(), &profiles)
    }

    pub fn select_profile(&self, name: &ProfileName) -> Result<()> {
        let mut profiles = self.profiles()?;
        ensure!(
            *name == ProfileName::default() || profiles.profiles.contains_key(name),
            "profile `{name}` not found; create it with `topk login --profile {name}`"
        );
        profiles.selected = (*name != ProfileName::default()).then(|| name.clone());
        write_toml(&self.config_file(), &profiles)
    }

    pub async fn logout(&self) -> Result<()> {
        self.session().await?.delete()?;
        self.access_tokens().clear()?;
        let mut profiles = self.profiles()?;
        profiles.remove(&self.active_profile);
        // An empty config is not kept.
        match profiles.is_empty() {
            true => remove(&self.config_file()),
            false => write_toml(&self.config_file(), &profiles),
        }
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

    pub fn dir() -> Result<PathBuf> {
        std::env::var_os("TOPK_CONFIG_DIR")
            .map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("topk")))
            .context("no config directory")
    }

    pub fn import_dir(&self) -> PathBuf {
        self.profile_dir().join("import")
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

    pub fn active_profile(&self) -> &ProfileName {
        &self.active_profile
    }

    pub fn profiles(&self) -> Result<Profiles> {
        Profiles::load(&self.dir)
    }

    fn profile_dir(&self) -> PathBuf {
        self.active_profile.dir(&self.dir)
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

    fn config_file(&self) -> PathBuf {
        self.dir.join(CONFIG_FILE)
    }

    fn defaults_file(&self) -> PathBuf {
        self.tenant_dir()
            .join("defaults")
            .join(format!("{}.toml", self.host.key()))
    }

    /// Everything scoped to this issuer's session lives here.
    fn tenant_dir(&self) -> PathBuf {
        self.profile_dir()
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

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("removing {}", path.display())),
    }
}
