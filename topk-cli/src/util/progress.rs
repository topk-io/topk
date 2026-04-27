use std::io::{self, IsTerminal};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct Spinner {
    pub(crate) progress_bar: Option<ProgressBar>,
    pub(crate) multi: Option<MultiProgress>,
}

impl Spinner {
    pub fn new(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg}")
    }

    pub fn with_elapsed(msg: impl Into<String>) -> Self {
        Self::create(msg, "{spinner:.cyan} {msg} [{elapsed}]")
    }

    pub fn disabled() -> Self {
        Self {
            progress_bar: None,
            multi: None,
        }
    }

    fn create(msg: impl Into<String>, template: &str) -> Self {
        if !io::stderr().is_terminal() {
            return Self::disabled();
        }
        let multi = MultiProgress::new();
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(ProgressStyle::with_template(template).expect("valid spinner template"));
        bar.set_message(msg.into());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self {
            progress_bar: Some(bar),
            multi: Some(multi),
        }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.set_message(msg.into());
        }
    }

    /// Print text above the spinner without disrupting it.
    pub fn print(&self, msg: impl AsRef<str>) {
        match &self.multi {
            Some(m) => {
                let _ = m.println(msg.as_ref());
            }
            None => eprintln!("{}", msg.as_ref()),
        }
    }

    pub fn finish(self) {
        if let Some(progress_bar) = self.progress_bar {
            progress_bar.finish_and_clear();
        }
        if let Some(m) = self.multi {
            let _ = m.clear();
        }
    }
}
