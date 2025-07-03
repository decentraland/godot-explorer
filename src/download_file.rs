use reqwest::Url;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use crate::ui::create_download_progress;

enum DownloadEvent {
    TotalSize(u64),
    Progress(u64),
    Result(Result<(), anyhow::Error>),
}

async fn download_file_thread(
    url: Url,
    path: PathBuf,
    sender: std::sync::mpsc::Sender<DownloadEvent>,
) {
    let client = reqwest::Client::new();
    let mut response = match client.get(url).send().await {
        Ok(response) => response,
        Err(err) => {
            let _ = sender.send(DownloadEvent::Result(Err(err.into())));
            return;
        }
    };
    
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

    while let Some(chunk) = response.chunk().await.unwrap() {
        if let Err(err) = file.write_all(&chunk) {
            let _ = sender.send(DownloadEvent::Result(Err(err.into())));
            return;
        }
        downloaded += chunk.len() as u64;

        // Send progress update
        let _ = sender.send(DownloadEvent::Progress(downloaded));
    }

    let _ = sender.send(DownloadEvent::Result(Ok(())));
}

pub fn _download_file(url: &str, path: &str) -> Result<(), anyhow::Error> {
    let (sender, receiver) = std::sync::mpsc::channel::<DownloadEvent>();
    let url = Url::parse(url)?;
    let path = PathBuf::from(path);

    tokio::spawn(async move {
        download_file_thread(url, path, sender).await;
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
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async { _download_file(url, path) })
}
