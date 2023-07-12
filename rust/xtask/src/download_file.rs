use reqwest::Url;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

enum DownloadEvent {
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

    let mut last_download_report = 0;
    // Process events
    loop {
        match receiver.recv() {
            Ok(event) => match event {
                DownloadEvent::Progress(bytes) => {
                    if bytes - last_download_report > 5e6 as u64 {
                        println!("Bytes downloaded: {}", bytes);
                        last_download_report = bytes;
                    }
                }
                DownloadEvent::Result(res) => {
                    return res;
                }
            },
            Err(err) => {
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
