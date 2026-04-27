use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MimeType {
    ApplicationPdf,
    TextMarkdown,
    TextHtml,
    ImagePng,
    ImageJpeg,
    ImageGif,
    ImageWebp,
    ImageTiff,
    ImageBmp,
    Other(String),
}

impl MimeType {
    pub fn is_supported(&self) -> bool {
        !matches!(self, MimeType::Other(_))
    }

    pub fn to_ext(&self) -> &str {
        match self {
            MimeType::ImagePng => "png",
            MimeType::ImageJpeg => "jpg",
            MimeType::ImageGif => "gif",
            MimeType::ImageWebp => "webp",
            MimeType::ImageTiff => "tiff",
            MimeType::ImageBmp => "bmp",
            _ => "bin",
        }
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MimeType::ApplicationPdf => "application/pdf",
            MimeType::TextMarkdown => "text/markdown",
            MimeType::TextHtml => "text/html",
            MimeType::ImagePng => "image/png",
            MimeType::ImageJpeg => "image/jpeg",
            MimeType::ImageGif => "image/gif",
            MimeType::ImageWebp => "image/webp",
            MimeType::ImageTiff => "image/tiff",
            MimeType::ImageBmp => "image/bmp",
            MimeType::Other(s) => s,
        };
        f.write_str(s)
    }
}

impl From<&str> for MimeType {
    fn from(s: &str) -> Self {
        match s {
            "application/pdf" => MimeType::ApplicationPdf,
            "text/markdown" => MimeType::TextMarkdown,
            "text/html" => MimeType::TextHtml,
            "image/png" => MimeType::ImagePng,
            "image/jpeg" => MimeType::ImageJpeg,
            "image/gif" => MimeType::ImageGif,
            "image/webp" => MimeType::ImageWebp,
            "image/tiff" => MimeType::ImageTiff,
            "image/bmp" => MimeType::ImageBmp,
            other => MimeType::Other(other.to_string()),
        }
    }
}

impl From<String> for MimeType {
    fn from(s: String) -> Self {
        MimeType::from(s.as_str())
    }
}
