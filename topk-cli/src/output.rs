use std::io::{IsTerminal, Write};

use dialoguer::{console::Term, Confirm};
use serde::Serialize;

use crate::util::Spinner;

pub const GREEN: &str = "\x1b[32m";
pub const RED: &str = "\x1b[31m";
pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const BLUE: &str = "\x1b[34m";

#[derive(Debug, Clone, Copy, clap::ValueEnum, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

pub trait RenderForHuman: Serialize {
    fn render(&self) -> impl Into<String>;
}

#[derive(Clone, Copy)]
pub struct Output {
    format: OutputFormat,
}

impl Output {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    pub fn print<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        match self.format {
            OutputFormat::Json => self.print_json(value),
            OutputFormat::Text => self.print_text(value),
        }
    }

    pub fn print_text<T: RenderForHuman>(&self, value: &T) -> Result<(), serde_json::Error> {
        clear_progress();
        let rendered: String = value.render().into();
        if !rendered.is_empty() {
            println!("{rendered}");
        }
        Ok(())
    }

    pub fn print_json<T: Serialize>(&self, value: &T) -> Result<(), serde_json::Error> {
        println!("{}", serde_json::to_string(value)?);
        Ok(())
    }

    pub fn print_json_line<T: Serialize>(&self, value: &T) -> Result<(), serde_json::Error> {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        serde_json::to_writer(&mut lock, value)?;
        writeln!(&mut lock).map_err(serde_json::Error::io)?;
        Ok(())
    }

    pub fn spinner(&self, msg: impl Into<String>) -> Spinner {
        match self.format {
            OutputFormat::Text => Spinner::with_elapsed(msg),
            OutputFormat::Json => Spinner::disabled(),
        }
    }

    pub fn success(&self, msg: &str) {
        if matches!(self.format, OutputFormat::Text) {
            eprintln!("{GREEN}✓{RESET} {msg}");
        }
    }

    pub fn is_human(&self) -> bool {
        matches!(self.format, OutputFormat::Text)
    }

    pub fn can_render_human_stderr(&self) -> bool {
        self.is_human() && std::io::stderr().is_terminal()
    }

    pub fn confirm(&self, prompt: &str) -> std::io::Result<bool> {
        if !matches!(self.format, OutputFormat::Text) || !std::io::stdout().is_terminal() {
            return Ok(false);
        }

        #[cfg(unix)]
        {
            let tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")?;
            let term = Term::read_write_pair(tty.try_clone()?, tty);
            return Confirm::new()
                .with_prompt(prompt)
                .default(false)
                .wait_for_newline(true)
                .interact_on(&term)
                .map_err(std::io::Error::other);
        }

        #[cfg(not(unix))]
        {
            if std::io::stdin().is_terminal() {
                return Confirm::new()
                    .with_prompt(prompt)
                    .default(false)
                    .wait_for_newline(true)
                    .interact()
                    .map_err(std::io::Error::other);
            }

            Ok(false)
        }
    }

    pub fn confirm_or_yes(&self, prompt: &str, yes: bool) -> std::io::Result<bool> {
        if yes {
            Ok(true)
        } else {
            self.confirm(prompt)
        }
    }

    pub fn error(&self, e: &anyhow::Error) {
        match self.format {
            OutputFormat::Json => {
                let payload = serde_json::json!({ "error": format!("{:#}", e) });
                eprintln!("{payload}");
            }
            OutputFormat::Text => eprintln!("{BOLD}{RED}error:{RESET} {:#}", e),
        }
    }
}

fn clear_progress() {
    if std::io::stderr().is_terminal() {
        eprint!("\r\x1b[2K");
        let _ = std::io::stderr().flush();
    }
}

#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use tempfile::tempdir;

    fn cmd() -> Command {
        Command::cargo_bin("topk").unwrap()
    }

    #[test]
    fn missing_api_key_error() {
        let config_home = tempdir().unwrap();
        let home = tempdir().unwrap();

        let out = cmd()
            .env_remove("TOPK_API_KEY")
            .env("XDG_CONFIG_HOME", config_home.path())
            .env("HOME", home.path())
            .args(["-o", "json", "dataset", "list"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let stderr = String::from_utf8_lossy(&out.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(stderr.trim()).expect("stderr should be valid JSON");
        let error = parsed["error"].as_str().unwrap();
        assert!(error.contains("API key not set"));
        assert!(error.contains("TOPK_API_KEY"));
    }

    #[test]
    fn completions_zsh() {
        let out = cmd().args(["completions", "zsh"]).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("#compdef topk"));
    }

    #[test]
    fn completions_bash() {
        let out = cmd().args(["completions", "bash"]).output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("topk"));
    }
}
