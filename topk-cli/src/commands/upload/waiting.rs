use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures::stream::{self, StreamExt, TryStreamExt};
use topk_rs::{Client, Error};

use crate::output::Output;

/// Wait for every upload handle to be processed, concurrently.
///
/// Returns `Ok(true)` if all handles processed, `Ok(false)` if the user
/// pressed Enter to skip the wait in interactive mode. In JSON / non-TTY
/// mode there is no cancel affordance — the future runs to completion.
pub async fn wait_for_all(
    client: &Client,
    dataset: &str,
    handles: Vec<String>,
    concurrency: usize,
    output: &Output,
) -> Result<bool, Error> {
    let total = handles.len() as u64;
    let spinner = output.spinner(format!("0/{total} processed — press Enter to skip"));
    let pb = spinner.bar.clone();

    let process_fut = {
        let client = client.clone();
        let dataset = dataset.to_string();
        async move {
            let done = Arc::new(AtomicU64::new(0));
            stream::iter(handles)
                .map(|handle| {
                    let client = client.clone();
                    let dataset = dataset.to_string();
                    let pb = pb.clone();
                    let done = done.clone();
                    async move {
                        client
                            .dataset(&dataset)
                            .wait_for_handle(&handle, None)
                            .await?;
                        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                        if let Some(pb) = &pb {
                            pb.set_message(format!("{n}/{total} processed — press Enter to skip"));
                        }
                        Ok::<_, Error>(())
                    }
                })
                .buffer_unordered(concurrency)
                .try_collect::<()>()
                .await
        }
    };

    let processed = if output.is_human() {
        let cancel = cancel_on_enter();
        tokio::select! {
            r = process_fut => { r?; true }
            _ = cancel => { false }
        }
    } else {
        process_fut.await?;
        true
    };

    spinner.finish();
    Ok(processed)
}

/// Spawn an OS thread that reads a line from stdin and signals the returned
/// receiver. The thread is detached — when the main future wins the `select!`
/// the process exits and the OS tears the thread down.
fn cancel_on_enter() -> tokio::sync::oneshot::Receiver<()> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    std::thread::spawn(move || {
        let _ = std::io::stdin().read_line(&mut String::new());
        let _ = tx.send(());
    });
    rx
}
