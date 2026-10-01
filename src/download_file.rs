use crate::ui::{create_download_progress, print_message, MessageType};
use reqwest::Url;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

/// Attempts (first try + retries) for any single HTTP operation.
pub const MAX_HTTP_ATTEMPTS: u32 = 5;
/// Backoff before each retry: 2s, 4s, 8s, 16s.
const RETRY_BASE_DELAY: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);
/// Total timeout for the small blocking requests (npm manifest, protocol tarball).
/// Large file downloads go through `download_file`, which has no total timeout.
const BLOCKING_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
/// Longest wait for the response headers or the next body chunk. A CDN that stops
/// sending mid-body without closing the connection used to hang the download until
/// the CI job timeout; now the attempt fails and `retry_http` starts a new one.
const STALL_TIMEOUT: Duration = Duration::from_secs(60);

enum DownloadEvent {
    TotalSize(u64),
    Progress(u64),
    Result(Result<(), anyhow::Error>),
}

/// Error marker for failures that won't fix themselves on a retry (a 404 for a
/// mistyped/unpublished artifact URL, for instance), so `retry_http` gives up
/// right away instead of burning the whole backoff schedule.
#[derive(Debug)]
struct PermanentError(String);

impl std::fmt::Display for PermanentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PermanentError {}

/// Runs `op`, retrying transient failures with exponential backoff.
///
/// Release CDNs (GitHub, npm, our own R2 buckets) intermittently drop
/// connections mid-handshake — `http2 error: stream error received: refused
/// stream ...` being the usual one in CI — which used to fail the whole
/// `install` command on the first hiccup.
pub fn retry_http<T>(
    what: &str,
    mut op: impl FnMut() -> Result<T, anyhow::Error>,
) -> Result<T, anyhow::Error> {
    let mut attempt = 1;
    loop {
        let err = match op() {
            Ok(value) => return Ok(value),
            Err(err) => err,
        };

        let permanent = err.downcast_ref::<PermanentError>().is_some();
        if permanent || attempt >= MAX_HTTP_ATTEMPTS {
            return Err(err.context(format!("{what} failed after {attempt} attempt(s)")));
        }

        let delay = RETRY_BASE_DELAY * 2u32.pow(attempt - 1);
        print_message(
            MessageType::Warning,
            &format!(
                "{what} failed (attempt {attempt}/{MAX_HTTP_ATTEMPTS}): {err}. Retrying in {}s...",
                delay.as_secs()
            ),
        );
        std::thread::sleep(delay);
        attempt += 1;
    }
}

/// Fails on non-success HTTP statuses, flagging client errors (other than the
/// retryable 408/429) as permanent.
fn check_status(status: reqwest::StatusCode, url: &str) -> Result<(), anyhow::Error> {
    if status.is_success() {
        return Ok(());
    }

    let retryable = !status.is_client_error()
        || status == reqwest::StatusCode::REQUEST_TIMEOUT
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS;

    let message = format!("unexpected HTTP status {status} for {url}");
    if retryable {
        Err(anyhow::anyhow!(message))
    } else {
        Err(anyhow::Error::new(PermanentError(message)))
    }
}

/// Forces HTTP/1.1: the release CDNs answer HTTP/2 requests with sporadic
/// `refused stream` errors, and multiplexing buys nothing for single large
/// downloads anyway.
fn async_client() -> Result<reqwest::Client, anyhow::Error> {
    Ok(reqwest::Client::builder()
        .http1_only()
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?)
}

fn blocking_client() -> Result<reqwest::blocking::Client, anyhow::Error> {
    Ok(reqwest::blocking::Client::builder()
        .http1_only()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(BLOCKING_REQUEST_TIMEOUT)
        .build()?)
}

/// Blocking GET of a whole body into memory, with retries. `what` is used in
/// the retry/failure messages.
pub fn http_get_bytes(url: &str, what: &str) -> Result<Vec<u8>, anyhow::Error> {
    let client = blocking_client()?;
    retry_http(what, || {
        let response = client.get(url).send()?;
        check_status(response.status(), url)?;
        Ok(response.bytes()?.to_vec())
    })
}

async fn download_file_thread(
    client: reqwest::Client,
    url: Url,
    path: PathBuf,
    sender: std::sync::mpsc::Sender<DownloadEvent>,
    stall: Duration,
) {
    let url_str = url.to_string();
    let mut response = match tokio::time::timeout(stall, client.get(url).send()).await {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            let _ = sender.send(DownloadEvent::Result(Err(err.into())));
            return;
        }
        Err(_) => {
            let msg = format!("no response from {url_str} for {}s", stall.as_secs());
            let _ = sender.send(DownloadEvent::Result(Err(anyhow::anyhow!(msg))));
            return;
        }
    };

    if let Err(err) = check_status(response.status(), &url_str) {
        let _ = sender.send(DownloadEvent::Result(Err(err)));
        return;
    }

    // Send total size if available
    if let Some(len) = response.content_length() {
        let _ = sender.send(DownloadEvent::TotalSize(len));
    }

    let mut downloaded = 0;

    let mut file = match File::create(&path) {
        Ok(file) => file,
        Err(err) => {
            let _ = sender.send(DownloadEvent::Result(Err(err.into())));
            return;
        }
    };

    loop {
        // A connection dropped mid-body is reported here: surface it as an
        // error so the caller can retry (it used to panic on `unwrap`).
        let chunk = match tokio::time::timeout(stall, response.chunk()).await {
            Ok(Ok(Some(chunk))) => chunk,
            Ok(Ok(None)) => break,
            Ok(Err(err)) => {
                let _ = sender.send(DownloadEvent::Result(Err(err.into())));
                return;
            }
            Err(_) => {
                let msg = format!("download of {url_str} stalled for {}s", stall.as_secs());
                let _ = sender.send(DownloadEvent::Result(Err(anyhow::anyhow!(msg))));
                return;
            }
        };

        if let Err(err) = file.write_all(&chunk) {
            let _ = sender.send(DownloadEvent::Result(Err(err.into())));
            return;
        }
        downloaded += chunk.len() as u64;

        // Send progress update
        let _ = sender.send(DownloadEvent::Progress(downloaded));
    }

    if let Err(err) = file.flush() {
        let _ = sender.send(DownloadEvent::Result(Err(err.into())));
        return;
    }

    let _ = sender.send(DownloadEvent::Result(Ok(())));
}

fn download_file_attempt(
    client: &reqwest::Client,
    url: &str,
    path: &str,
    stall: Duration,
) -> Result<(), anyhow::Error> {
    let (sender, receiver) = std::sync::mpsc::channel::<DownloadEvent>();
    // Append a cache-busting query param so any intermediate CDN/proxy
    // (and the upstream object store) always serves the latest object.
    // Recomputed per attempt so a retry never gets a cached bad response.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let busted_url = if url.contains('?') {
        format!("{url}&t={ts}")
    } else {
        format!("{url}?t={ts}")
    };
    let url = Url::parse(&busted_url)?;
    let path = PathBuf::from(path);
    let client = client.clone();

    tokio::spawn(async move {
        download_file_thread(client, url, path, sender, stall).await;
    });

    let mut progress_bar = None;

    // Process events
    loop {
        match receiver.recv() {
            Ok(event) => match event {
                DownloadEvent::TotalSize(total) => {
                    progress_bar = Some(create_download_progress(total));
                }
                DownloadEvent::Progress(bytes) => {
                    if let Some(ref pb) = progress_bar {
                        pb.set_position(bytes);
                    }
                }
                DownloadEvent::Result(res) => {
                    if let Some(pb) = progress_bar {
                        if res.is_ok() {
                            pb.finish_with_message("Download completed");
                        } else {
                            pb.finish_with_message("Download failed");
                        }
                    }
                    return res;
                }
            },
            Err(err) => {
                if let Some(pb) = progress_bar {
                    pb.finish_with_message("Download interrupted");
                }
                return Err(err.into());
            }
        }
    }
}

pub fn download_file(url: &str, path: &str) -> Result<(), anyhow::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let client = runtime.block_on(async { async_client() })?;

    retry_http(&format!("Download of {url}"), || {
        let result =
            runtime.block_on(async { download_file_attempt(&client, url, path, STALL_TIMEOUT) });
        if result.is_err() {
            // Don't leave a truncated file behind: callers (android_deps.zip,
            // the download cache) treat an existing file as a finished download.
            let _ = std::fs::remove_file(path);
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::TcpListener;

    /// A server that sends headers and part of the body, then goes silent without
    /// closing, must fail the attempt instead of hanging it (CI Android job, 1.14.1).
    #[test]
    fn stalled_body_fails_the_attempt() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\npartial");
            std::thread::sleep(Duration::from_secs(10));
        });

        let dir = std::env::temp_dir().join(format!("xtask-stall-{port}"));
        let path = dir.to_string_lossy().to_string();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let client = async_client().unwrap();
        let started = std::time::Instant::now();
        let url = format!("http://127.0.0.1:{port}/file.zip");
        let result = runtime.block_on(async {
            download_file_attempt(&client, &url, &path, Duration::from_secs(1))
        });
        let _ = std::fs::remove_file(&path);

        let err = result.expect_err("a stalled body must fail the attempt");
        assert!(err.to_string().contains("stalled"), "{err}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
