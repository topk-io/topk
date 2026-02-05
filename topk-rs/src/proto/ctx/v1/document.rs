use std::fmt;

use crate::proto::ctx::v1::DocumentKind;

use crate::error::Error;

impl DocumentKind {
    pub fn from_extension(extension: &str) -> Result<Self, Error> {
        if extension.eq_ignore_ascii_case("pdf") {
            Ok(Self::Pdf)
        } else if extension.eq_ignore_ascii_case("md") {
            Ok(Self::Markdown)
        } else {
            Err(Error::Input(anyhow::anyhow!(
                "Invalid document extension: {extension}"
            )))
        }
    }
}

impl fmt::Display for DocumentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => write!(f, "unspecified"),
            Self::Pdf => write!(f, "pdf"),
            Self::Markdown => write!(f, "markdown"),
        }
    }
}
