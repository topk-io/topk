use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{ensure, Error, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::host::Host;
use crate::storage::{read_toml, write_toml};

const DEFAULT: &str = "default";
const PROFILES_FILE: &str = "profiles.toml";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ProfileName(String);

impl Default for ProfileName {
    fn default() -> Self {
        Self(DEFAULT.to_owned())
    }
}

impl ProfileName {
    pub fn dir(&self, root: &Path) -> PathBuf {
        root.join("profiles").join(&self.0)
    }

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

/// The file every profile's host and the selected profile are saved in.
#[derive(Clone)]
pub struct SavedProfiles {
    root: PathBuf,
}

impl SavedProfiles {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn load(&self) -> Result<Profiles> {
        Ok(read_toml(&self.path())?.unwrap_or_default())
    }

    pub fn select(&self, name: &ProfileName) -> Result<()> {
        let mut profiles = self.load()?;
        ensure!(
            *name == ProfileName::default() || profiles.profiles.contains_key(name),
            "profile `{name}` not found; create it with `topk login --profile {name}`"
        );
        profiles.selected = (*name != ProfileName::default()).then(|| name.clone());
        self.write(&profiles)
    }

    pub fn save(&self, name: &ProfileName, host: &Host) -> Result<()> {
        let mut profiles = self.load()?;
        profiles.profiles.insert(name.clone(), host.clone());
        self.write(&profiles)
    }

    pub fn remove(&self, name: &ProfileName) -> Result<()> {
        let mut profiles = self.load()?;
        if profiles.profiles.remove(name).is_none() {
            return Ok(());
        }
        if profiles.selected.as_ref() == Some(name) {
            profiles.selected = None;
        }
        self.write(&profiles)
    }

    fn write(&self, profiles: &Profiles) -> Result<()> {
        write_toml(&self.path(), profiles)
    }

    fn path(&self) -> PathBuf {
        self.root.join(PROFILES_FILE)
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Profiles {
    selected: Option<ProfileName>,
    #[serde(default)]
    profiles: BTreeMap<ProfileName, Host>,
}

impl Profiles {
    /// The selected profile, or the default one when none is selected.
    pub fn selected(&self) -> ProfileName {
        self.selected.clone().unwrap_or_default()
    }

    pub fn host(&self, name: &ProfileName) -> Option<&Host> {
        self.profiles.get(name)
    }

    pub fn list(&self) -> impl Iterator<Item = (&ProfileName, &Host)> {
        self.profiles.iter()
    }
}
