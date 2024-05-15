use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{self, AsyncWriteExt};
use tokio::sync::{Notify, RwLock, Semaphore};
use tokio::time::Instant;
use reqwest::Client;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use futures_util::StreamExt;

struct FileMetadata {
    file_size: i64,
    last_accessed: Instant,
}

pub struct ResourceProvider {
    cache_folder: PathBuf,
    existing_files: RwLock<HashMap<String, FileMetadata>>,
    max_cache_size: i64,
    pending_downloads: Mutex<HashMap<String, Arc<Notify>>>,
    client: Client,
    initialized: AtomicBool,
    semaphore: Arc<Semaphore>,
}

impl ResourceProvider {
    // Synchronous constructor that sets up the ResourceProvider
    pub fn new(cache_folder: &str, max_cache_size: i64, max_concurrent_downloads: usize) -> Self {
        ResourceProvider {
            cache_folder: PathBuf::from(cache_folder),
            existing_files: RwLock::new(HashMap::new()),
            max_cache_size,
            pending_downloads: Mutex::new(HashMap::new()),
            client: Client::new(),
            initialized: AtomicBool::new(false),
            semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
        }
    }

    // Private asynchronous function to initialize the cache
    async fn initialize(&self) -> Result<(), io::Error> {
        let mut existing_files = self.existing_files.write().await;
        let dir = std::fs::read_dir(&self.cache_folder)?;
        for entry in dir {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                let file_path = entry.path();
                if file_path.extension().and_then(|ext| ext.to_str()) == Some("tmp") {
                    fs::remove_file(&file_path).await?;
                    continue;
                }
                let metadata = entry.metadata()?;
                let file_size = metadata.len() as i64;
                let file_path_str = file_path.to_str().unwrap().to_string();
                self.add_file(&mut existing_files, file_path_str, file_size).await;
            }
        }
        Ok(())
    }

    async fn add_file(&self, existing_files: &mut HashMap<String, FileMetadata>, file_path: String, file_size: i64) {
        // If adding the new file exceeds the cache size, remove less used files
        while self.total_size(existing_files) + file_size > self.max_cache_size {
            if !self.remove_less_used(existing_files).await {
                break;
            }
        }

        let metadata = FileMetadata {
            file_size,
            last_accessed: Instant::now(),
        };
        existing_files.insert(file_path, metadata);
    }

    async fn remove_file(&self, existing_files: &mut HashMap<String, FileMetadata>, file_path: &str) -> Option<FileMetadata> {
        if let Some(metadata) = existing_files.remove(file_path) {
            let _ = fs::remove_file(file_path).await;
            Some(metadata)
        } else {
            None
        }
    }

    fn total_size(&self, existing_files: &HashMap<String, FileMetadata>) -> i64 {
        existing_files.values().map(|metadata| metadata.file_size).sum()
    }

    async fn remove_less_used(&self, existing_files: &mut HashMap<String, FileMetadata>) -> bool {
        if let Some((file_path, _)) = existing_files.iter()
            .min_by_key(|(_, metadata)| metadata.last_accessed)
            .map(|(path, metadata)| (path.clone(), metadata)) {
                self.remove_file(existing_files, &file_path).await;
            true
        } else {
            false
        }
    }

    fn touch_file(&self, existing_files: &mut HashMap<String, FileMetadata>, file_path: &str) {
        if let Some(metadata) = existing_files.get_mut(file_path) {
            metadata.last_accessed = Instant::now();
        }
    }

    async fn download_file(&self, url: &str, dest: &Path) -> Result<(), String> {
        let tmp_dest = dest.with_extension("tmp");
        let response = self.client.get(url).send().await.map_err(|e| format!("Request error: {:?}", e))?;
        let mut file = fs::File::create(&tmp_dest).await.map_err(|e| format!("File creation error: {:?}", e))?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {:?}", e))?;
            file.write_all(&chunk).await.map_err(|e| format!("File write error: {:?}", e))?;
        }

        fs::rename(&tmp_dest, dest).await.map_err(|e| format!("Failed to rename file: {:?}", e))?;

        Ok(())
    }

    pub async fn fetch_resource_or_wait(
        &self,
        url: &String,
        file_hash: &String,
        absolute_file_path: &String,
    ) -> Result<(), String> {
        if !self.initialized.load(Ordering::SeqCst) {
            self.initialize().await.map_err(|_| "Error initializing the cache")?;
            self.initialized.store(true, Ordering::SeqCst);
        }

        let notify = {
            let mut pending_downloads = self.pending_downloads.lock().unwrap();
            if let Some(notify) = pending_downloads.get(file_hash) {
                Some(notify.clone())
            } else {
                let notify = Arc::new(Notify::new());
                pending_downloads.insert(file_hash.clone(), notify.clone());
                None
            }
        };

        if let Some(notify) = notify {
            notify.notified().await;
            let existing_files = self.existing_files.read().await;
            if existing_files.contains_key(absolute_file_path) {
                return Ok(());
            } else {
                return Err("File not found after waiting".to_string());
            }
        }

        let permit = self.semaphore.acquire().await.unwrap();

        if tokio::fs::metadata(&absolute_file_path).await.is_err() {
            self.download_file(url, Path::new(absolute_file_path)).await?;

            let metadata = tokio::fs::metadata(absolute_file_path)
                .await
                .map_err(|e| format!("Failed to get metadata: {:?}", e))?;
            let file_size = metadata.len() as i64;

            let mut existing_files = self.existing_files.write().await;
            self.add_file(&mut existing_files, absolute_file_path.clone(), file_size).await;
        } else {
            let mut existing_files = self.existing_files.write().await;
            self.touch_file(&mut existing_files, absolute_file_path);
        }

        let mut pending_downloads = self.pending_downloads.lock().unwrap();
        if let Some(notify) = pending_downloads.remove(file_hash) {
            notify.notify_waiters();
        }

        drop(permit);

        Ok(())
    }

    // Method to clear the cache and delete all files from the file system
    pub async fn clear(&self) {
        let mut existing_files = self.existing_files.write().await;
        let file_paths: Vec<String> = existing_files.keys().cloned().collect();
        for file_path in file_paths {
            self.remove_file(&mut existing_files, &file_path).await;
        }
    }

    // Method to change the number of concurrent downloads
    pub fn set_max_concurrent_downloads(&mut self, max: usize) {
        self.semaphore = Arc::new(Semaphore::new(max));
    }

    // Method to change the max cache size
    pub fn set_max_cache_size(&mut self, size: i64) {
        self.max_cache_size = size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::Result;
    use futures_util::future::join_all;

    async fn setup_cache_folder(path: &str) -> Result<()> {
        if tokio::fs::metadata(path).await.is_err() {
            tokio::fs::create_dir_all(path).await?;
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_resource_or_wait() {
        let path = "./cache";
        let max_cache_size = 1024 * 1024 * 1024; // Set the cache size to 1 GB

        setup_cache_folder(path).await.expect("Failed to create cache folder");

        let provider = Arc::new(ResourceProvider::new(path, max_cache_size, 2));
        provider.clear().await;

        let files_to_download = vec![
            ("https://link.testfile.org/15MB", "bafkreibmrvrdgqthfrvehyell552sk7ivuas2ozzjdmlojbzttqlcrxiya"),
            ("https://link.testfile.org/15MB", "bafkreic4osvzsjzyqutwjxt2xmyd4hjrwukrxzclvixke3putyrihggmam"),
            ("https://link.testfile.org/15MB", "bafkreibhjuitdcu3jwu7khjcg2fo6xf2h3hilnfv4liy4p5h2olxj6tcce"),
        ];

        // Create a vector to hold the handles of the spawned tasks
        let handles: Vec<_> = files_to_download.clone().into_iter().map(|(url, file_hash)| {
            let url = url.to_string();
            let file_hash = file_hash.to_string();
            let absolute_file_path = format!("{}/{}", path, file_hash);

            let provider_clone = provider.clone();
            tokio::spawn(async move {
                provider_clone.fetch_resource_or_wait(&url, &file_hash, &absolute_file_path).await.expect("Failed to fetch resource");
            })
        }).collect();

        // Await all the handles
        join_all(handles).await;

        // Extract file hashes from the files_to_download vector
        let file_hashes: Vec<_> = files_to_download.iter().map(|(_, file_hash)| *file_hash).collect();

        // Check if all files have been downloaded
        for file_hash in file_hashes {
            let absolute_file_path = format!("{}/{}", path, file_hash);
            let existing_files = provider.existing_files.read().await;
            assert!(existing_files.contains_key(&absolute_file_path));
        }

        {
            let mut existing_files = provider.existing_files.write().await;
            provider.remove_less_used(&mut existing_files).await;
            assert!(existing_files.len() == 2);
        }

        {
            provider.clear().await;
            let existing_files = provider.existing_files.read().await;
            assert!(provider.total_size(&existing_files) == 0);
            assert!(existing_files.is_empty());
        }
    }
}
