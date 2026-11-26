use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{ensure, Context, Error, Result};
use colored::Colorize;
use serde::Serialize;

use crate::host::Host;
use crate::storage::{config_dir, read, read_toml, write_secret_file, write_toml};

const DEFAULT: &str = "default";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ProfileName(String);

impl Default for ProfileName {
    fn default() -> Self {
        Self(DEFAULT.to_owned())
    }
}

impl ProfileName {
    /// Names the profile in a message, unless it is the default one.
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

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A separate login with its own host, session and defaults.
#[derive(Clone)]
pub struct Profile {
    root: PathBuf,
    name: ProfileName,
}

impl Profile {
    pub fn new(root: PathBuf, name: ProfileName) -> Self {
        Self { root, name }
    }

    pub fn name(&self) -> &ProfileName {
        &self.name
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join("profiles").join(&self.name.0)
    }

    /// The host the profile is logged in to.
    pub fn host(&self) -> Result<Option<Host>> {
        read_toml(&self.host_file())
    }

    /// A profile exists once it has been logged in to.
    pub fn exists(&self) -> Result<bool> {
        Ok(self.host()?.is_some())
    }

    pub fn save(&self, host: &Host) -> Result<()> {
        write_toml(&self.host_file(), host)
    }

    /// Log the profile out, keeping its default project and region.
    pub fn forget(&self) -> Result<()> {
        match std::fs::remove_file(self.host_file()) {
            Err(e) if e.kind() != ErrorKind::NotFound => {
                Err(e).with_context(|| format!("forgetting profile `{}`", self.name))
            }
            _ => Ok(()),
        }
    }

    fn host_file(&self) -> PathBuf {
        self.dir().join("profile.toml")
    }
}

pub struct Profiles {
    root: PathBuf,
}

impl Profiles {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Profiles in the user's config directory.
    pub fn load() -> Result<Self> {
        let profiles = Self::new(config_dir()?);
        profiles.remove_legacy_config()?;
        Ok(profiles)
    }

    pub fn get(&self, name: ProfileName) -> Profile {
        Profile::new(self.root.clone(), name)
    }

    /// The selected profile, or the default one when none is selected or
    /// the selected one has since been logged out.
    pub fn selected(&self) -> Result<ProfileName> {
        let Some(name) = read(&self.selection_file())? else {
            return Ok(ProfileName::default());
        };
        let name: ProfileName = name.trim().parse().context("invalid saved profile")?;
        Ok(match self.get(name.clone()).exists()? {
            true => name,
            false => ProfileName::default(),
        })
    }

    pub fn select(&self, name: &ProfileName) -> Result<()> {
        if *name == ProfileName::default() {
            return match std::fs::remove_file(self.selection_file()) {
                Err(e) if e.kind() != ErrorKind::NotFound => Err(e).context("clearing profile"),
                _ => Ok(()),
            };
        }
        ensure!(
            self.get(name.clone()).exists()?,
            "profile `{name}` not found; create it with `topk login --profile {name}`"
        );
        write_secret_file(&self.selection_file(), &name.0)
    }

    /// The profiles that have been logged in to, with their hosts, in order.
    pub fn list(&self) -> Result<Vec<(ProfileName, Host)>> {
        let mut names = match std::fs::read_dir(self.root.join("profiles")) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
                .collect::<Vec<ProfileName>>(),
            Err(e) if e.kind() == ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e).context("listing profiles"),
        };
        names.sort();
        let mut profiles = Vec::new();
        for name in names {
            if let Some(host) = self.get(name.clone()).host()? {
                profiles.push((name, host));
            }
        }
        Ok(profiles)
    }

    fn selection_file(&self) -> PathBuf {
        self.root.join("selected_profile")
    }

    fn remove_legacy_config(&self) -> Result<()> {
        match std::fs::remove_file(self.root.join("config.toml")) {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(e).context("removing config.toml"),
            _ => Ok(()),
        }
    }
}
