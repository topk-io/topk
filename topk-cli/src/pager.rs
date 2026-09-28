use std::io::{IsTerminal, Stdout, Write};
use std::process::{Child, Command, Stdio};

/// Routes terminal text output through a pager when it exceeds the screen height.
/// Short results and redirected output print directly.
///
/// Selects `$TOPK_PAGER`, then `$PAGER`, falling back to `less`. An empty command
/// or `cat` disables paging.
pub struct Pager(State);

#[derive(Debug, thiserror::Error)]
#[error("pager closed: {0}")]
struct Closed(#[source] std::io::Error);

impl Closed {
    fn wrap(error: std::io::Error) -> std::io::Error {
        match error.kind() {
            std::io::ErrorKind::BrokenPipe => std::io::Error::new(error.kind(), Self(error)),
            _ => error,
        }
    }
}

enum State {
    /// Held until it is known whether the output fits the screen.
    Screen {
        held: Vec<u8>,
        lines: usize,
        height: usize,
    },
    Pager(Child),
    Stdout(Stdout),
}

impl Pager {
    pub fn stdout() -> Self {
        let height = std::io::stdout()
            .is_terminal()
            .then(crossterm::terminal::size)
            .and_then(Result::ok)
            .map(|(_, rows)| rows as usize)
            .filter(|&rows| rows > 0);
        Self(match height {
            Some(height) => State::Screen {
                held: Vec::new(),
                lines: 0,
                height,
            },
            None => return Self::direct(),
        })
    }

    /// Straight to stdout, never paged; for output meant for other programs.
    pub fn direct() -> Self {
        Self(State::Stdout(std::io::stdout()))
    }

    /// Print what is held and wait for the user to quit the pager, if it started.
    pub fn finish(self) -> std::io::Result<()> {
        match self.0 {
            State::Screen { held, .. } => std::io::stdout().write_all(&held),
            State::Pager(mut child) => {
                drop(child.stdin.take());
                let status = child.wait()?;
                if !status.success() {
                    return Err(std::io::Error::other(format!("pager exited with {status}")));
                }
                Ok(())
            }
            State::Stdout(mut stdout) => stdout.flush(),
        }
    }

    /// Whether the error is the user quitting the pager before the output ended.
    pub fn quit(error: &anyhow::Error) -> bool {
        error
            .chain()
            .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
            .any(|error| error.get_ref().is_some_and(|inner| inner.is::<Closed>()))
    }
}

/// `None` when paging is off or the pager cannot start.
fn spawn() -> Option<Child> {
    let mut command = match std::env::var("TOPK_PAGER").or_else(|_| std::env::var("PAGER")) {
        Ok(pager) if pager.trim().is_empty() || pager.trim() == "cat" => return None,
        Ok(pager) if cfg!(windows) => {
            let mut command = Command::new("cmd");
            command.args(["/C", &pager]);
            command
        }
        Ok(pager) => {
            let mut command = Command::new("sh");
            command.args(["-c", &pager]);
            command
        }
        // Spawned directly, so a missing `less` falls back to stdout.
        Err(_) => Command::new("less"),
    };
    // Pass colours and leave the output on screen after quitting.
    if std::env::var_os("LESS").is_none() {
        command.env("LESS", "RX");
    }
    let child = command.stdin(Stdio::piped()).spawn().ok()?;
    ignore_sigpipe();
    Some(child)
}

/// Overrides `main`'s default SIGPIPE for the rest of the run, once a pager is up:
/// quitting the pager closes the pipe, and writes then fail with `BrokenPipe`
/// (see `Pager::quit`) instead of the signal killing the run.
fn ignore_sigpipe() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN)
    };
}

impl Write for Pager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match &mut self.0 {
            State::Screen {
                held,
                lines,
                height,
            } => {
                held.extend_from_slice(buf);
                *lines += buf.iter().filter(|&&b| b == b'\n').count();
                // One line stays free for the shell prompt.
                if *lines >= *height {
                    let held = std::mem::take(held);
                    self.0 = match spawn() {
                        Some(child) => State::Pager(child),
                        None => State::Stdout(std::io::stdout()),
                    };
                    self.write_all(&held)?;
                }
                Ok(buf.len())
            }
            State::Pager(child) => child
                .stdin
                .as_mut()
                .expect("pager stdin")
                .write(buf)
                .map_err(Closed::wrap),
            State::Stdout(stdout) => stdout.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.0 {
            State::Screen { .. } => Ok(()),
            State::Pager(child) => child
                .stdin
                .as_mut()
                .expect("pager stdin")
                .flush()
                .map_err(Closed::wrap),
            State::Stdout(stdout) => stdout.flush(),
        }
    }
}
