use futures_util::StreamExt;
use reqwest::Client;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Notify, OnceCell, RwLock, Semaphore};
use tokio::time::Instant;

#[cfg(feature = "use_resource_tracking")]
use super::resource_download_tracking::ResourceDownloadTracking;
use crate::content::semaphore_ext::SemaphoreExt;

pub struct FileMetadata {
    file_size: i64,
    last_accessed: Instant,
}

pub struct ResourceProvider {
    cache_folder: PathBuf,
    existing_files: RwLock<HashMap<String, FileMetadata>>,
    max_cache_size: AtomicI64,
    downloaded_size: AtomicU64,
    pending_downloads: RwLock<HashMap<String, Arc<Notify>>>,
    client: Client,
    initialized: OnceCell<()>,
    semaphore: Arc<Semaphore>,
    #[cfg(feature = "use_resource_tracking")]
    download_tracking: Arc<ResourceDownloadTracking>,
}

const UPDATE_THRESHOLD: u64 = 1_024 * 1_024; // 1 MB threshold

impl ResourceProvider {
    // Synchronous constructor that sets up the ResourceProvider
    pub fn new(
        cache_folder: &str,
        max_cache_size: i64,
        max_concurrent_downloads: usize,
        #[cfg(feature = "use_resource_tracking")] download_tracking: Arc<ResourceDownloadTracking>,
    ) -> Self {
        ResourceProvider {
            cache_folder: PathBuf::from(cache_folder),
            existing_files: RwLock::new(HashMap::new()),
            max_cache_size: AtomicI64::new(max_cache_size),
            pending_downloads: RwLock::new(HashMap::new()),
            client: Client::new(),
            initialized: OnceCell::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
            downloaded_size: AtomicU64::new(0),
            #[cfg(feature = "use_resource_tracking")]
            download_tracking,
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
                self.add_file(&mut existing_files, file_path_str, file_size)
                    .await;
            }
        }
        self.ensure_space_for(&mut existing_files, 0).await;
        Ok(())
    }

    async fn ensure_space_for(
        &self,
        existing_files: &mut HashMap<String, FileMetadata>,
        file_size: i64,
    ) {
        // If adding the new file exceeds the cache size, remove less used files
        let max_cache_size = self.max_cache_size.load(Ordering::SeqCst);
        while self.total_size(existing_files) + file_size > max_cache_size {
            if !self.remove_less_used(existing_files).await {
                break;
            }
        }
    }

    async fn add_file(
        &self,
        existing_files: &mut HashMap<String, FileMetadata>,
        file_path: String,
        file_size: i64,
    ) {
        let metadata = FileMetadata {
            file_size,
            last_accessed: Instant::now(),
        };
        existing_files.insert(file_path, metadata);
    }

    async fn remove_file(
        &self,
        existing_files: &mut HashMap<String, FileMetadata>,
        file_path: &str,
    ) -> Option<FileMetadata> {
        if let Some(metadata) = existing_files.remove(file_path) {
            let _ = fs::remove_file(file_path).await;
            Some(metadata)
        } else {
            None
        }
    }

    fn total_size(&self, existing_files: &HashMap<String, FileMetadata>) -> i64 {
        existing_files
            .values()
            .map(|metadata| metadata.file_size)
            .sum()
    }

    async fn remove_less_used(&self, existing_files: &mut HashMap<String, FileMetadata>) -> bool {
        if let Some((file_path, _)) = existing_files
            .iter()
            .min_by_key(|(_, metadata)| metadata.last_accessed)
            .map(|(path, metadata)| (path.clone(), metadata))
        {
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

    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        #[cfg(feature = "use_resource_tracking")] file_hash: &str,
    ) -> Result<(), String> {
        let tmp_dest = dest.with_extension("tmp");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request error: {:?}", e))?;

        #[cfg(feature = "use_resource_tracking")]
        self.download_tracking.start(file_hash.to_string()).await;

        #[cfg(feature = "use_resource_tracking")]
        let mut current_size = 0;

        if !response.status().is_success() {
            return Err(format!("Failed to download file: {:?}", response.status()));
        }

        let mut file = fs::File::create(&tmp_dest)
            .await
            .map_err(|e| format!("File creation error: {:?}", e))?;
        let mut stream = response.bytes_stream();

        let mut accumulated_size = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {:?}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("File write error: {:?}", e))?;

            accumulated_size += chunk.len() as u64;
            if accumulated_size > UPDATE_THRESHOLD {
                self.downloaded_size
                    .fetch_add(accumulated_size, Ordering::Relaxed);
                #[cfg(feature = "use_resource_tracking")]
                {
                    current_size += accumulated_size;
                    self.download_tracking
                        .report_progress(file_hash, current_size)
                        .await;
                }
                accumulated_size = 0;
            }
        }

        if accumulated_size > 0 {
            self.downloaded_size
                .fetch_add(accumulated_size, Ordering::Relaxed);
            #[cfg(feature = "use_resource_tracking")]
            {
                current_size += accumulated_size;
                self.download_tracking
                    .report_progress(file_hash, current_size)
                    .await;
            }
        }

        fs::rename(&tmp_dest, dest).await.map_err(|e| {
            format!(
                "Failed to rename file: {:?} from: {:?} to: {:?}",
                e, tmp_dest, dest
            )
        })?;

        #[cfg(feature = "use_resource_tracking")]
        self.download_tracking.end(file_hash).await;

        Ok(())
    }

    async fn download_file_with_buffer(
        &self,
        url: &str,
        dest: &Path,
        #[cfg(feature = "use_resource_tracking")] file_hash: &str,
    ) -> Result<Vec<u8>, String> {
        let tmp_dest = dest.with_extension("tmp");
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Request error: {:?}", e))?;

        #[cfg(feature = "use_resource_tracking")]
        self.download_tracking.start(file_hash.to_string()).await;
        #[cfg(feature = "use_resource_tracking")]
        let mut current_size = 0;

        let mut file = fs::File::create(&tmp_dest)
            .await
            .map_err(|e| format!("File creation error: {:?}", e))?;
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();

        let mut accumulated_size = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream error: {:?}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("File write error: {:?}", e))?;
            buffer.extend_from_slice(&chunk);

            accumulated_size += chunk.len() as u64;
            if accumulated_size > UPDATE_THRESHOLD {
                self.downloaded_size
                    .fetch_add(accumulated_size, Ordering::Relaxed);
                #[cfg(feature = "use_resource_tracking")]
                {
                    current_size += accumulated_size;
                    self.download_tracking
                        .report_progress(file_hash, current_size)
                        .await;
                }
                accumulated_size = 0;
            }
        }

        if accumulated_size > 0 {
            self.downloaded_size
                .fetch_add(accumulated_size, Ordering::Relaxed);
            #[cfg(feature = "use_resource_tracking")]
            {
                current_size += accumulated_size;
                self.download_tracking
                    .report_progress(file_hash, current_size)
                    .await;
            }
        }

        fs::rename(&tmp_dest, dest).await.map_err(|e| {
            format!(
                "Failed to rename file: {:?} from: {:?} to: {:?}",
                e, tmp_dest, dest
            )
        })?;

        #[cfg(feature = "use_resource_tracking")]
        self.download_tracking.end(file_hash).await;

        Ok(buffer)
    }

    async fn ensure_initialized(&self) -> Result<(), String> {
        self.initialized
            .get_or_try_init(|| async { self.initialize().await.map_err(|e| e.to_string()) })
            .await
            .map(|_| ())
    }

    async fn handle_existing_file(&self, absolute_file_path: &String) -> Result<Vec<u8>, String> {
        let mut existing_files = self.existing_files.write().await;
        self.touch_file(&mut existing_files, absolute_file_path);

        let mut file = fs::File::open(absolute_file_path)
            .await
            .map_err(|e| format!("Failed to open file: {:?}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file: {:?}", e))?;
        Ok(buffer)
    }

    async fn handle_pending_download(
        &self,
        file_hash: &String,
        absolute_file_path: &String,
    ) -> Result<(), String> {
        let notify = {
            let mut pending_downloads = self.pending_downloads.write().await;
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

        Ok(())
    }

    pub async fn file_exists(&self, file_hash: &str) -> bool {
        let existing_files = self.existing_files.read().await;
        let absolute_file_path = self.cache_folder.join(file_hash);
        let absolute_file_path = absolute_file_path.to_str().unwrap().to_string();
        existing_files.contains_key(&absolute_file_path)
    }

    pub async fn store_file(&self, file_hash: &str, bytes: &[u8]) -> Result<(), String> {
        self.ensure_initialized().await?;
        let absolute_file_path = self.cache_folder.join(file_hash);

        // Write the bytes to a temporary file first
        let tmp_dest = absolute_file_path.with_extension("tmp");
        let mut file = fs::File::create(&tmp_dest)
            .await
            .map_err(|e| format!("File creation error: {:?}", e))?;
        file.write_all(bytes)
            .await
            .map_err(|e| format!("File write error: {:?}", e))?;
        fs::rename(&tmp_dest, &absolute_file_path)
            .await
            .map_err(|e| {
                format!(
                    "Failed to rename file: {:?} from: {:?} to: {:?}",
                    e, tmp_dest, absolute_file_path
                )
            })?;

        // Update the cache map
        let file_size = bytes.len() as i64;
        let mut existing_files = self.existing_files.write().await;
        self.ensure_space_for(&mut existing_files, file_size).await;
        self.add_file(
            &mut existing_files,
            absolute_file_path.to_str().unwrap().to_string(),
            file_size,
        )
        .await;

        Ok(())
    }

    pub async fn fetch_resource(
        &self,
        url: String,
        file_hash: String,
        absolute_file_path: String,
    ) -> Result<(), String> {
        self.ensure_initialized().await?;

        self.handle_pending_download(&file_hash, &absolute_file_path)
            .await?;

        let permit = self.semaphore.acquire().await.unwrap();

        if tokio::fs::metadata(&absolute_file_path).await.is_err() {
            self.download_file(
                &url,
                Path::new(&absolute_file_path),
                #[cfg(feature = "use_resource_tracking")]
                file_hash,
            )
            .await?;

            let metadata = tokio::fs::metadata(&absolute_file_path)
                .await
                .map_err(|e| format!("Failed to get metadata: {:?}", e))?;
            let file_size = metadata.len() as i64;

            let mut existing_files = self.existing_files.write().await;
            self.ensure_space_for(&mut existing_files, file_size).await;
            self.add_file(&mut existing_files, absolute_file_path.clone(), file_size)
                .await;
        } else {
            self.handle_existing_file(&absolute_file_path).await?;
        }

        let mut pending_downloads = self.pending_downloads.write().await;
        if let Some(notify) = pending_downloads.remove(&file_hash) {
            notify.notify_waiters();
        }

        drop(permit);

        Ok(())
    }

    // Method to fetch resource and wait for the data
    pub async fn fetch_resource_with_data(
        &self,
        url: &str,
        file_hash: &String,
        absolute_file_path: &String,
    ) -> Result<Vec<u8>, String> {
        self.ensure_initialized().await?;

        self.handle_pending_download(file_hash, absolute_file_path)
            .await?;

        let permit = self.semaphore.acquire().await.unwrap();
        let data = if tokio::fs::metadata(&absolute_file_path).await.is_err() {
            let data = self
                .download_file_with_buffer(
                    url,
                    Path::new(absolute_file_path),
                    #[cfg(feature = "use_resource_tracking")]
                    file_hash,
                )
                .await?;
            let metadata = tokio::fs::metadata(absolute_file_path)
                .await
                .map_err(|e| format!("Failed to get metadata: {:?}", e))?;
            let file_size = metadata.len() as i64;
            let mut existing_files = self.existing_files.write().await;
            self.ensure_space_for(&mut existing_files, file_size).await;
            self.add_file(&mut existing_files, absolute_file_path.clone(), file_size)
                .await;
            data
        } else {
            self.handle_existing_file(absolute_file_path).await?
        };

        let mut pending_downloads = self.pending_downloads.write().await;
        if let Some(notify) = pending_downloads.remove(file_hash) {
            notify.notify_waiters();
        }

        drop(permit);

        Ok(data)
    }

    // Method to clear the cache and delete all files from the file system
    pub async fn clear(&self) {
        if self.ensure_initialized().await.is_err() {
            tracing::error!("ResourceLoader failed to load!");
            return;
        }

        let mut existing_files = self.existing_files.write().await;
        let file_paths: Vec<String> = existing_files.keys().cloned().collect();
        for file_path in file_paths {
            self.remove_file(&mut existing_files, &file_path).await;
        }
    }

    pub fn consume_download_size(&self) -> u64 {
        self.downloaded_size.swap(0, Ordering::AcqRel)
    }

    // Method to change the number of concurrent downloads
    pub fn set_max_concurrent_downloads(&self, max: usize) {
        self.semaphore.set_permits(max)
    }

    // Method to change the max cache size
    pub fn set_max_cache_size(&self, size: i64) {
        self.max_cache_size.store(size, Ordering::SeqCst);
    }

    pub fn get_cache_total_size(&self) -> i64 {
        let existing_files = self.existing_files.blocking_read();
        self.total_size(&existing_files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::join_all;
    use tokio::io::Result;

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

        setup_cache_folder(path)
            .await
            .expect("Failed to create cache folder");

        #[cfg(feature = "use_resource_tracking")]
        let resource_download_tracking = Arc::new(ResourceDownloadTracking::new());

        let provider = Arc::new(ResourceProvider::new(
            path,
            max_cache_size,
            2,
            #[cfg(feature = "use_resource_tracking")]
            resource_download_tracking.clone(),
        ));
        provider.clear().await;

        let files_to_download = vec![
            (
                "https://link.testfile.org/15MB",
                "bafkreibmrvrdgqthfrvehyell552sk7ivuas2ozzjdmlojbzttqlcrxiya",
            ),
            (
                "https://link.testfile.org/15MB",
                "bafkreic4osvzsjzyqutwjxt2xmyd4hjrwukrxzclvixke3putyrihggmam",
            ),
            (
                "https://link.testfile.org/15MB",
                "bafkreibhjuitdcu3jwu7khjcg2fo6xf2h3hilnfv4liy4p5h2olxj6tcce",
            ),
        ];

        // Create a vector to hold the handles of the spawned tasks
        let handles: Vec<_> = files_to_download
            .clone()
            .into_iter()
            .map(|(url, file_hash)| {
                let url = url.to_string();
                let file_hash = file_hash.to_string();
                let absolute_file_path = format!("{}/{}", path, file_hash);

                let provider_clone = provider.clone();
                tokio::spawn(async move {
                    provider_clone
                        .fetch_resource(url, file_hash, absolute_file_path)
                        .await
                        .expect("Failed to fetch resource");
                })
            })
            .collect();

        // Await all the handles
        join_all(handles).await;

        // Extract file hashes from the files_to_download vector
        let file_hashes: Vec<_> = files_to_download
            .iter()
            .map(|(_, file_hash)| *file_hash)
            .collect();

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
