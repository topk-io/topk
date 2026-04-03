use std::io::{self, IsTerminal, Write};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub fn parse_kv(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("expected key=value, got '{}'", s))
}

pub fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprint!("{}", prompt);
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Yes"))
}

pub struct FileProgress {
    pub overall: Option<ProgressBar>,
    pub current: Option<ProgressBar>,
    multi: Option<MultiProgress>,
}

impl FileProgress {
    pub fn new(total: u64) -> Self {
        if !io::stderr().is_terminal() {
            return Self { overall: None, current: None, multi: None };
        }
        let multi = MultiProgress::new();
        let overall = multi.add(ProgressBar::new(total));
        overall.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>4}/{len:4} files",
            )
            .unwrap()
            .progress_chars("##-"),
        );
        let current = multi.add(ProgressBar::new_spinner());
        current.set_style(ProgressStyle::with_template("{spinner:.cyan} {msg}").unwrap());
        current.enable_steady_tick(std::time::Duration::from_millis(100));
        Self { overall: Some(overall), current: Some(current), multi: Some(multi) }
    }

    pub fn finish(self) {
        if let Some(pb) = self.current { pb.finish_and_clear(); }
        if let Some(pb) = self.overall { pb.finish_and_clear(); }
        if let Some(m) = self.multi { let _ = m.clear(); }
    }
}

pub struct Spinner {
    pub bar: Option<ProgressBar>,
    multi: Option<MultiProgress>,
}

impl Spinner {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg}")
    }

    pub fn with_elapsed(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg} [{elapsed}]")
    }

    pub fn disabled() -> Self {
        Self { bar: None, multi: None }
    }

    fn create(msg: impl Into<String>, template: &str) -> Self {
        if !io::stderr().is_terminal() {
            return Self { bar: None, multi: None };
        }
        let multi = MultiProgress::new();
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(ProgressStyle::with_template(template).unwrap());
        bar.set_message(msg.into());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self { bar: Some(bar), multi: Some(multi) }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        if let Some(pb) = &self.bar {
            pb.set_message(msg.into());
        }
    }

    /// Print a line above the spinner without disrupting it.
    pub fn println(&self, msg: impl AsRef<str>) {
        match &self.multi {
            Some(m) => { let _ = m.println(msg.as_ref()); }
            None => eprintln!("{}", msg.as_ref()),
        }
    }

    pub fn finish(self) {
        if let Some(pb) = self.bar { pb.finish_and_clear(); }
        if let Some(m) = self.multi { let _ = m.clear(); }
    }
}
