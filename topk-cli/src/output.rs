use colored::Colorize;
use serde::Serialize;
use topk_rs::Error;

use crate::progress::Spinner;

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy)]
pub struct Output {
    pub format: OutputFormat,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn print_json<T: Serialize>(&self, value: &T) -> Result<(), Error> {
        println!(
            "{}",
            serde_json::to_string(value).map_err(|e| Error::MalformedResponse(e.to_string()))?
        );
        Ok(())
    }

    pub fn spinner(&self, msg: impl Into<String>) -> Spinner {
        match self.format {
            OutputFormat::Text => Spinner::with_elapsed(msg),
            OutputFormat::Json => Spinner::disabled(),
        }
    }

    pub fn success(&self, msg: &str) {
        match self.format {
            OutputFormat::Text => {
                eprintln!("{} {msg}", "✓".green());
            }
            OutputFormat::Json => {}
        }
    }

    pub fn meta(&self, msg: &str) {
        match self.format {
            OutputFormat::Text => {
                eprintln!("{msg}");
            }
            OutputFormat::Json => {}
        }
    }

    pub fn error(&self, e: &Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!("{payload}");
            }
            OutputFormat::Text => eprintln!("{} {:#}", "error:".red().bold(), e),
        }
    }
}
