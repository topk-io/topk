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
