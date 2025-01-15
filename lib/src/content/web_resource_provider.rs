use godot::builtin::{GString, PackedByteArray, PackedStringArray};
use godot::engine::file_access::ModeFlags;
use godot::engine::{DirAccess, FileAccess, TlsOptions};
use godot::obj::{Gd, NewGd};
use godot::prelude::ToGodot;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io;
use tokio::sync::{Notify, OnceCell, RwLock, Semaphore};
use tokio::time::Instant;

use crate::content::semaphore_ext::SemaphoreExt;
use crate::godot_classes::dcl_global::DclGlobal;
use crate::http_request::http_queue_requester::HttpQueueRequester;
use crate::http_request::request_response::{RequestOption, ResponseType};

use super::packed_array::PackedByteArrayFromVec;

pub struct FileMetadata {
    file_size: i64,
    last_accessed: Instant,
}

pub struct ResourceProvider {
    cache_folder: String,
    existing_files: RwLock<HashMap<String, FileMetadata>>,
    max_cache_size: AtomicI64,
    downloaded_size: AtomicU64,
    pending_downloads: RwLock<HashMap<String, Arc<Notify>>>,
    initialized: OnceCell<()>,
    semaphore: Arc<Semaphore>,
    http_queue_requester: Arc<HttpQueueRequester>,
}

const UPDATE_THRESHOLD: u64 = 1_024 * 1_024; // 1 MB threshold

impl ResourceProvider {
    // Synchronous constructor that sets up the ResourceProvider
    pub fn new(cache_folder: &str, max_cache_size: i64, max_concurrent_downloads: usize) -> Self {
        ResourceProvider {
            cache_folder: cache_folder.to_string(),
            existing_files: RwLock::new(HashMap::new()),
            max_cache_size: AtomicI64::new(max_cache_size),
            pending_downloads: RwLock::new(HashMap::new()),
            initialized: OnceCell::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent_downloads)),
            downloaded_size: AtomicU64::new(0),
            http_queue_requester: Arc::new(HttpQueueRequester::new(
                6,
                DclGlobal::get_network_inspector_sender(),
            )),
        }
    }

    // Private asynchronous function to initialize the cache
    async fn initialize(&self) -> Result<(), io::Error> {
        let cache_folder: GString = self.cache_folder.clone().into();
        let mut existing_files = self.existing_files.blocking_write();

        // Use GodotFileSystem to check if directory exists and create if needed
        if !DirAccess::dir_exists_absolute(cache_folder.clone()) {
            DirAccess::make_dir_recursive_absolute(cache_folder.clone());
        }

        // List files in directory using DirAccess
        let dir = DirAccess::open(cache_folder.clone());
        if dir.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to open directory",
            ));
        }
        let mut dir = dir.unwrap();

        dir.list_dir_begin();
        loop {
            let file = dir.get_next();
            if file.is_empty() {
                break;
            }

            let file_path: String = format!("{}/{}", self.cache_folder, file);
            let file_path_gstr: GString = file_path.clone().into();

            // Skip directories
            if DirAccess::dir_exists_absolute(file_path_gstr.clone()) {
                continue;
            }

            // Handle temporary files
            if file_path.as_str().ends_with(".tmp") {
                DirAccess::remove_absolute(file_path_gstr.clone());
                continue;
            }

            // Get file size using FileAccess
            if let Some(file_handle) = FileAccess::open(file_path_gstr.clone(), ModeFlags::READ) {
                let file_size = file_handle.get_length() as i64;
                self.add_file(&mut existing_files, file_path, file_size);
            }
        }

        drop(dir);

        self.ensure_space_for(&mut existing_files, 0);
        Ok(())
    }

    fn ensure_space_for(
        &self,
        existing_files: &mut HashMap<String, FileMetadata>,
        file_size: i64,
    ) {
        // If adding the new file exceeds the cache size, remove less used files
        let max_cache_size = self.max_cache_size.load(Ordering::SeqCst);
        while self.total_size(existing_files) + file_size > max_cache_size {
            if !self.remove_less_used(existing_files) {
                break;
            }
        }
    }

    fn add_file(
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
            DirAccess::remove_absolute(file_path.into());
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

    fn remove_less_used(&self, existing_files: &mut HashMap<String, FileMetadata>) -> bool {
        if let Some((file_path, _)) = existing_files
            .iter()
            .min_by_key(|(_, metadata)| metadata.last_accessed)
            .map(|(path, metadata)| (path.clone(), metadata))
        {
            self.remove_file(existing_files, &file_path);
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
        let data = self.download_file_with_buffer(url, dest).await?;

        // Write the downloaded data to the file
        if let Some(mut file) =
            FileAccess::open(dest.to_string_lossy().as_ref().into(), ModeFlags::WRITE)
        {
            file.store_buffer(PackedByteArray::from_vec(&data));
            Ok(())
        } else {
            Err(format!(
                "Failed to open file for writing: {}",
                dest.display()
            ))
        }
    }

    async fn download_file_with_buffer(&self, url: &str, dest: &Path) -> Result<Vec<u8>, String> {
        let request = RequestOption::new(
            0,
            url.to_string(),
            http::Method::GET,
            ResponseType::ToFile(dest.to_string_lossy().as_ref().into()),
            None,
            None,
            None,
        );
        let request_response = self.http_queue_requester.request(request, 0).await;
        match request_response {
            Ok(request_response) => {
                let file_path =
                    FileAccess::open(dest.to_string_lossy().as_ref().into(), ModeFlags::READ)
                        .ok_or(format!("Failed open file: {}", dest.display()))?;
                let data = file_path.get_buffer(file_path.get_length() as i64);
                Ok(data.to_vec())
            }
            Err(e) => Err(e.error_message),
        }
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

        let file_handle = FileAccess::open(absolute_file_path.into(), ModeFlags::READ);
        if let Some(file) = file_handle {
            let data = file.get_buffer(file.get_length() as i64);
            Ok(data.to_vec())
        } else {
            Err(format!("Failed to open file: {}", absolute_file_path))
        }
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

    fn _get_file_size(&self, absolute_file_path: &String) -> Result<i64, String> {
        let file_handle = FileAccess::open(absolute_file_path.into(), ModeFlags::READ);
        if let Some(file) = file_handle {
            Ok(file.get_length() as i64)
        } else {
            Err(format!("Failed to open file: {}", absolute_file_path))
        }
    }

    pub async fn fetch_resource(
        &self,
        url: &str,
        file_hash: &String,
        absolute_file_path: &String,
    ) -> Result<(), String> {
        tracing::info!("Fetching resource: {}", absolute_file_path);
        self.ensure_initialized().await?;

        tracing::info!("Handling pending download");
        self.handle_pending_download(file_hash, absolute_file_path)
            .await?;

        tracing::info!("Acquiring semaphore");
        let permit = self.semaphore.acquire().await.unwrap();

        tracing::info!("Checking if file exists");
        if !DirAccess::dir_exists_absolute(absolute_file_path.into()) {
            tracing::info!("Downloading file");
            self.download_file(url, Path::new(absolute_file_path))
                .await?;

            tracing::info!("Getting file size");
            let file_size = self._get_file_size(absolute_file_path)?;

            tracing::info!("Ensuring space for file");
            let mut existing_files = self.existing_files.write().await;
            self.ensure_space_for(&mut existing_files, file_size);
            self.add_file(&mut existing_files, absolute_file_path.clone(), file_size);
        } else {
            tracing::info!("File exists, handling existing file");
            self.handle_existing_file(absolute_file_path).await?;
        }

        let mut pending_downloads = self.pending_downloads.write().await;
        if let Some(notify) = pending_downloads.remove(file_hash) {
            tracing::info!("Notifying waiters");
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
        let data = if !FileAccess::file_exists(absolute_file_path.into()) {
            let data = self
                .download_file_with_buffer(url, Path::new(absolute_file_path))
                .await?;

            let metadata = FileAccess::open(absolute_file_path.into(), ModeFlags::READ)
                .ok_or(format!("Failed open file: {}", absolute_file_path))?;
            let file_size = metadata.get_length() as i64;
            let mut existing_files = self.existing_files.blocking_write();
            self.ensure_space_for(&mut existing_files, file_size);
            self.add_file(&mut existing_files, absolute_file_path.clone(), file_size);
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
