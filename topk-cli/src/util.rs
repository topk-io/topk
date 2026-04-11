use std::{
    collections::HashSet,
    io::{self, IsTerminal, Write},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use bytesize::ByteSize;
use comfy_table::{
    presets, Attribute, Cell, Color, ColumnConstraint, ContentArrangement, Table, Width,
};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use terminal_size::{terminal_size, Width as TermWidth};

pub(crate) fn confirm(prompt: &str) -> std::io::Result<bool> {
    eprint!("{}", prompt);
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Yes"))
}

pub fn plural<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

struct UploadProgressState {
    rows: Vec<UploadProgressRow>,
    active: HashSet<usize>,
    completed: HashSet<usize>,
    failed_indices: HashSet<usize>,
    completion_order: Vec<usize>,
    failed: usize,
    final_message: Option<String>,
}

pub struct UploadProgress {
    state: Arc<Mutex<UploadProgressState>>,
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct UploadProgressHandle {
    state: Arc<Mutex<UploadProgressState>>,
}

#[derive(Clone)]
pub struct UploadProgressRow {
    pub path: String,
    pub size: u64,
    pub mime_type: String,
}

impl UploadProgress {
    pub fn new(rows: Vec<UploadProgressRow>) -> (Self, UploadProgressHandle) {
        let state = Arc::new(Mutex::new(UploadProgressState {
            rows,
            active: HashSet::new(),
            completed: HashSet::new(),
            failed_indices: HashSet::new(),
            completion_order: Vec::new(),
            failed: 0,
            final_message: None,
        }));

        let handle = UploadProgressHandle {
            state: state.clone(),
        };

        if !io::stderr().is_terminal() {
            return (
                Self {
                    state,
                    stop_tx: None,
                    join_handle: None,
                },
                handle,
            );
        }

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let thread_state = state.clone();
        let join_handle = thread::spawn(move || {
            let mut tick = 0usize;
            let mut last_lines = 0usize;

            loop {
                let stopped = stop_rx.recv_timeout(Duration::from_millis(100)).is_ok();
                let rendered = {
                    let state = thread_state.lock().expect("upload progress state lock");
                    render_upload_progress(&state, tick, stopped)
                };
                redraw_rendered_block(&rendered, &mut last_lines);
                if stopped {
                    break;
                }
                tick = tick.wrapping_add(1);
            }
        });

        (
            Self {
                state,
                stop_tx: Some(stop_tx),
                join_handle: Some(join_handle),
            },
            handle,
        )
    }

    pub fn finish(mut self, message: impl Into<String>) {
        if let Ok(mut state) = self.state.lock() {
            state.final_message = Some(message.into());
        }

        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
        if io::stderr().is_terminal() {
            eprintln!();
        }
    }
}

impl UploadProgressHandle {
    pub fn start(&self, index: usize) {
        if let Ok(mut state) = self.state.lock() {
            state.active.insert(index);
        }
    }

    pub fn finish(&self, index: usize, success: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.active.remove(&index);
            state.completed.insert(index);
            state.completion_order.push(index);
            if !success {
                state.failed_indices.insert(index);
                state.failed += 1;
            }
        }
    }
}

pub fn render_upload_preview(summary: &str, rows: &[UploadProgressRow]) -> String {
    render_upload_table(summary, rows, &HashSet::new(), &HashSet::new(), 0, true)
}

pub fn rendered_block_line_count(rendered: &str) -> usize {
    rendered.lines().count().max(1)
}

pub fn clear_rendered_block(lines: usize) {
    if !io::stderr().is_terminal() || lines == 0 {
        return;
    }

    eprint!("\r\x1b[2K");
    for _ in 0..lines {
        eprint!("\x1b[1A\r\x1b[2K");
    }
    let _ = io::stderr().flush();
}

fn render_upload_progress(state: &UploadProgressState, tick: usize, stopped: bool) -> String {
    let summary = state.final_message.clone().unwrap_or_else(|| {
        let mut line = format!(
            "Uploading {}/{} {}",
            state.completed.len(),
            state.rows.len(),
            plural(state.rows.len(), "file", "files")
        );
        if state.failed > 0 {
            line.push_str(&format!(" ({} failed)", state.failed));
        }
        line
    });

    if stopped {
        return render_upload_final_table(
            &summary,
            &state.rows,
            &state.completion_order,
            &state.failed_indices,
        );
    }

    render_upload_table(
        &summary,
        &state.rows,
        &state.active,
        &state.completed,
        tick,
        stopped,
    )
}

fn render_upload_final_table(
    summary: &str,
    rows: &[UploadProgressRow],
    completion_order: &[usize],
    failed_indices: &HashSet<usize>,
) -> String {
    if completion_order.is_empty() {
        return summary.to_string();
    }

    let start = completion_order.len().saturating_sub(20);
    let visible_indices = &completion_order[start..];
    let more_count = completion_order.len().saturating_sub(visible_indices.len());
    let terminal_width = terminal_size()
        .map(|(TermWidth(w), _)| w as usize)
        .unwrap_or(100);
    let table_width = terminal_width.saturating_sub(1).max(40);
    let file_width = table_width.saturating_sub(52).max(24);
    let mut table = build_upload_table(table_width, file_width);

    for index in visible_indices {
        let row = &rows[*index];
        let (status, color) = if failed_indices.contains(index) {
            ("failed", Color::Red)
        } else {
            ("uploaded", Color::Green)
        };
        table.add_row([
            Cell::new(truncate_middle(&row.path, file_width)),
            Cell::new(ByteSize(row.size).to_string()),
            Cell::new(truncate_middle(&row.mime_type, 16)),
            Cell::new(status).fg(color),
        ]);
    }

    if more_count > 0 {
        format!(
            "{}\n{} more {}\n{summary}",
            table,
            more_count,
            plural(more_count, "file", "files")
        )
    } else {
        format!("{table}\n{summary}")
    }
}

fn render_upload_table(
    summary: &str,
    rows: &[UploadProgressRow],
    active: &HashSet<usize>,
    completed: &HashSet<usize>,
    tick: usize,
    stopped: bool,
) -> String {
    const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    let total = rows.len();
    let remaining: Vec<usize> = (0..total).filter(|idx| !completed.contains(idx)).collect();

    if remaining.is_empty() {
        return summary.to_string();
    }

    let terminal_width = terminal_size()
        .map(|(TermWidth(w), _)| w as usize)
        .unwrap_or(100);
    let table_width = terminal_width.saturating_sub(1).max(40);
    let file_width = table_width.saturating_sub(52).max(24);
    let mut table = build_upload_table(table_width, file_width);

    let spinner = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
    let visible_count = remaining.len().min(20);
    for index in remaining.iter().take(visible_count) {
        let (status, color) = if active.contains(index) {
            (format!("{spinner} uploading"), Color::Cyan)
        } else if stopped {
            ("queued".to_string(), Color::DarkGrey)
        } else {
            ("queued".to_string(), Color::DarkGrey)
        };
        let row = &rows[*index];
        table.add_row([
            Cell::new(truncate_middle(&row.path, file_width)),
            Cell::new(ByteSize(row.size).to_string()),
            Cell::new(truncate_middle(&row.mime_type, 16)),
            Cell::new(status).fg(color),
        ]);
    }

    let more_count = remaining.len().saturating_sub(visible_count);
    if more_count > 0 {
        format!(
            "{}\n{} more {}\n{summary}",
            table,
            more_count,
            plural(more_count, "file", "files")
        )
    } else {
        format!("{table}\n{summary}")
    }
}

fn build_upload_table(table_width: usize, file_width: usize) -> Table {
    let mut table = Table::new();
    table
        .load_preset(presets::NOTHING)
        .set_content_arrangement(ContentArrangement::Disabled)
        .set_width(table_width as u16)
        .set_header(
            ["FILE", "SIZE", "TYPE", "STATUS"]
                .into_iter()
                .map(|header| {
                    Cell::new(header)
                        .add_attribute(Attribute::Bold)
                        .fg(Color::Cyan)
                }),
        )
        .set_constraints([
            ColumnConstraint::Absolute(Width::Fixed(file_width as u16)),
            ColumnConstraint::Absolute(Width::Fixed(10)),
            ColumnConstraint::Absolute(Width::Fixed(18)),
            ColumnConstraint::Absolute(Width::Fixed(14)),
        ]);

    for index in 0..4 {
        if let Some(column) = table.column_mut(index) {
            column.set_padding((0, 1));
        }
    }

    table
}

fn truncate_middle(value: &str, max_len: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max_len {
        return value.to_string();
    }
    if max_len <= 1 {
        return "…".to_string();
    }

    let left_len = (max_len.saturating_sub(1)) / 2;
    let right_len = max_len.saturating_sub(1 + left_len);
    let left: String = chars[..left_len].iter().collect();
    let right: String = chars[chars.len() - right_len..].iter().collect();
    format!("{left}…{right}")
}

fn redraw_rendered_block(rendered: &str, last_lines: &mut usize) {
    if *last_lines > 0 {
        for _ in 0..(*last_lines - 1) {
            eprint!("\r\x1b[2K\x1b[1A");
        }
        eprint!("\r\x1b[2K");
    }
    eprint!("{rendered}");
    let _ = io::stderr().flush();
    *last_lines = rendered.lines().count().max(1);
}

pub struct Spinner {
    pub(crate) bar: Option<ProgressBar>,
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
            bar: None,
            multi: None,
        }
    }

    fn create(msg: impl Into<String>, template: &str) -> Self {
        if !io::stderr().is_terminal() {
            return Self {
                bar: None,
                multi: None,
            };
        }
        let multi = MultiProgress::new();
        let bar = multi.add(ProgressBar::new_spinner());
        bar.set_style(ProgressStyle::with_template(template).expect("valid spinner template"));
        bar.set_message(msg.into());
        bar.enable_steady_tick(std::time::Duration::from_millis(100));
        Self {
            bar: Some(bar),
            multi: Some(multi),
        }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        if let Some(pb) = &self.bar {
            pb.set_message(msg.into());
        }
    }

    /// Print a line above the spinner without disrupting it.
    pub fn println(&self, msg: impl AsRef<str>) {
        match &self.multi {
            Some(m) => {
                let _ = m.println(msg.as_ref());
            }
            None => eprintln!("{}", msg.as_ref()),
        }
    }

    pub fn finish(self) {
        if let Some(pb) = self.bar {
            pb.finish_and_clear();
        }
        if let Some(m) = self.multi {
            let _ = m.clear();
        }
    }
}
