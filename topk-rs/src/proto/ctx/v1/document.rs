use crate::proto::ctx::v1::DocumentKind;

use crate::error::Error;

impl DocumentKind {
    pub fn from_extension(extension: &str) -> Result<Self, Error> {
        match extension {
            "pdf" => Ok(Self::Pdf),
            "md" => Ok(Self::Markdown),
            _ => Err(Error::Input(anyhow::anyhow!(
                "Invalid document extension: {extension}"
            ))),
        }
    }
}
