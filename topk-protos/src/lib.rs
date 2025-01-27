use std::str::FromStr;

pub mod utils;

mod control;
mod data;

mod macros;

pub mod v1 {
    pub use super::control::v1 as control;
    pub use super::data::v1 as data;
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct OrgId(u64);

impl OrgId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::LowerHex for OrgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Display for OrgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for OrgId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<OrgId> for u64 {
    fn from(value: OrgId) -> Self {
        value.0
    }
}

impl FromStr for OrgId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct ProjectId(u32);

impl ProjectId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

impl std::fmt::LowerHex for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl std::fmt::Display for ProjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for ProjectId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<ProjectId> for u32 {
    fn from(value: ProjectId) -> Self {
        value.0
    }
}

impl FromStr for ProjectId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}
