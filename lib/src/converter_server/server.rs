/*
 * HTTP Server for Content Converter
 *
 * Runs an HTTP server that accepts file uploads and converts them to
 * optimized Godot resources for mobile platforms.
 *
 * Uses the full ContentProvider infrastructure with promises, threading, and caching.
 */

use godot::classes::Os;
use godot::obj::InstanceId;
use godot::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;

use crate::content::content_provider::ContentProvider;
use crate::godot_classes::promise::Promise;
use crate::scene_runner::tokio_runtime::TokioRuntime;

use super::handlers;

/// Represents a converted asset in the cache
#[derive(Clone, Debug)]
pub struct CachedAsset {
    pub hash: String,
    pub asset_type: AssetType,
    pub file_path: PathBuf,
    pub original_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AssetType {
    Scene,
    Texture,
}

/// Shared state for the converter server
pub struct ConverterState {
    pub cache_folder: PathBuf,
    pub assets: RwLock<HashMap<String, CachedAsset>>,
    pub port: u16,
    /// InstanceId of the ContentProvider node (for accessing from async handlers)
    pub content_provider_id: RwLock<Option<InstanceId>>,
    /// Semaphore for Godot thread safety
    pub godot_single_thread: Arc<Semaphore>,
}

impl ConverterState {
    pub fn new(cache_folder: PathBuf, port: u16) -> Self {
        // Create cache folder if it doesn't exist
        if !cache_folder.exists() {
            std::fs::create_dir_all(&cache_folder).ok();
        }

        Self {
            cache_folder,
            assets: RwLock::new(HashMap::new()),
            port,
            content_provider_id: RwLock::new(None),
            godot_single_thread: Arc::new(Semaphore::new(1)),
        }
    }

    pub fn get_asset(&self, hash: &str) -> Option<CachedAsset> {
        self.assets.read().ok()?.get(hash).cloned()
    }

    pub fn add_asset(&self, asset: CachedAsset) {
        if let Ok(mut assets) = self.assets.write() {
            assets.insert(asset.hash.clone(), asset);
        }
    }

    /// Get the ContentProvider instance from its stored InstanceId
    pub fn get_content_provider(&self) -> Option<Gd<ContentProvider>> {
        let id = self.content_provider_id.read().ok()?.as_ref()?.clone();
        Gd::<ContentProvider>::try_from_instance_id(id).ok()
    }

    /// Set the ContentProvider InstanceId
    pub fn set_content_provider(&self, provider: &Gd<ContentProvider>) {
        if let Ok(mut id) = self.content_provider_id.write() {
            *id = Some(provider.instance_id());
        }
    }
}

/// Result of waiting for a promise - uses primitive types that are Send
pub enum PromiseResult {
    /// Promise resolved successfully with serialized data
    Resolved(String),
    /// Promise rejected with error message
    Rejected(String),
}

/// Wait for a Promise to resolve or reject
/// Takes InstanceId instead of Gd<Promise> to be Send-safe
/// Returns the result as a PromiseResult enum with serialized data
pub async fn wait_for_promise(
    promise_id: InstanceId,
    semaphore: Arc<Semaphore>,
) -> Result<PromiseResult, String> {
    use crate::content::thread_safety::set_thread_safety_checks_enabled;
    use crate::godot_classes::promise::PromiseError;

    loop {
        // Acquire semaphore to safely access Godot API
        let _guard = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Semaphore error: {}", e))?;

        set_thread_safety_checks_enabled(false);

        // Access Godot types in a block that ends before await
        let poll_result: Result<Option<PromiseResult>, String> = (|| {
            // Recreate the Gd<Promise> from InstanceId each iteration
            let promise = match Gd::<Promise>::try_from_instance_id(promise_id) {
                Ok(p) => p,
                Err(_) => {
                    return Err("Promise no longer valid".to_string());
                }
            };

            let promise_ref = promise.bind();
            let is_resolved = promise_ref.is_resolved();
            let is_rejected = promise_ref.is_rejected();

            if is_resolved {
                let data = promise_ref.get_data();

                // Convert Variant to String while we have thread safety disabled
                let data_str = data.to_string();
                let error_str = if is_rejected {
                    match data.try_to::<Gd<PromiseError>>() {
                        Ok(err) => err.bind().get_error().to_string(),
                        Err(_) => "Unknown error".to_string(),
                    }
                } else {
                    String::new()
                };

                if is_rejected {
                    return Ok(Some(PromiseResult::Rejected(error_str)));
                }
                return Ok(Some(PromiseResult::Resolved(data_str)));
            }

            Ok(None) // Not yet resolved
        })();

        set_thread_safety_checks_enabled(true);
        drop(_guard);

        // Now check the result without holding any Godot types
        match poll_result {
            Err(e) => return Err(e),
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {
                // Not resolved yet, continue polling
            }
        }

        // Small delay before checking again
        tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
    }
}

/// The converter server Godot class
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ConverterServer {
    base: Base<Node>,

    #[var]
    port: u16,

    #[var]
    cache_folder: GString,

    state: Option<Arc<ConverterState>>,
    content_provider: Option<Gd<ContentProvider>>,
    running: bool,
}

#[godot_api]
impl INode for ConverterServer {
    fn ready(&mut self) {
        // Check command line args for converter server mode
        let args = Os::singleton().get_cmdline_args();
        let args: Vec<String> = args.to_vec().iter().map(|s| s.to_string()).collect();

        let mut is_converter_mode = false;
        let mut port: u16 = 3000;
        let mut cache_folder = String::from("user://converter-cache");

        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--converter-server" => {
                    is_converter_mode = true;
                }
                "--port" => {
                    if i + 1 < args.len() {
                        port = args[i + 1].parse().unwrap_or(3000);
                        i += 1;
                    }
                }
                "--cache-folder" => {
                    if i + 1 < args.len() {
                        cache_folder = args[i + 1].clone();
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }

        if is_converter_mode {
            self.port = port;
            self.cache_folder = GString::from(&cache_folder);
            self.start_server();
        }
    }

    fn exit_tree(&mut self) {
        self.stop_server();
    }
}

#[godot_api]
impl ConverterServer {
    #[func]
    pub fn start_server(&mut self) {
        if self.running {
            tracing::warn!("Converter server already running");
            return;
        }

        let cache_path = if self.cache_folder.to_string().starts_with("user://") {
            let user_path = Os::singleton().get_user_data_dir();
            PathBuf::from(user_path.to_string())
                .join(self.cache_folder.to_string().trim_start_matches("user://"))
        } else {
            PathBuf::from(self.cache_folder.to_string())
        };

        // Create state first
        let state = Arc::new(ConverterState::new(cache_path, self.port));

        // Create ContentProvider as a child node
        let content_provider = ContentProvider::new_alloc();
        self.base_mut()
            .add_child(&content_provider.clone().upcast::<Node>());

        // Store ContentProvider's InstanceId in state
        state.set_content_provider(&content_provider);
        self.content_provider = Some(content_provider);

        self.state = Some(state.clone());

        let port = self.port;

        tracing::info!("Starting converter server on port {}", port);

        // Start the HTTP server in a background task
        TokioRuntime::spawn(async move {
            if let Err(e) = run_http_server(state, port).await {
                tracing::error!("Converter server error: {:?}", e);
            }
        });

        self.running = true;

        tracing::info!(
            "Converter server started. Endpoints:\n\
             - POST /convert/gltf - Convert GLB/GLTF to .scn\n\
             - POST /convert/texture - Convert image to .res\n\
             - POST /package/scene - Create ZIP package\n\
             - GET /asset/{{hash}} - Download converted asset\n\
             - GET /health - Health check"
        );
    }

    #[func]
    pub fn stop_server(&mut self) {
        if self.running {
            tracing::info!("Converter server stopped");
            self.running = false;
        }
        // Remove ContentProvider child
        if let Some(mut content_provider) = self.content_provider.take() {
            content_provider.queue_free();
        }
        self.state = None;
    }

    #[func]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get the ContentProvider instance for conversion operations
    #[func]
    pub fn get_content_provider(&self) -> Option<Gd<ContentProvider>> {
        self.content_provider.clone()
    }
}

/// Run the HTTP server
async fn run_http_server(state: Arc<ConverterState>, port: u16) -> Result<(), anyhow::Error> {
    use std::net::SocketAddr;

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Create a simple TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Converter server listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, state).await {
                tracing::error!("Connection error: {:?}", e);
            }
        });
    }
}

/// Handle an HTTP connection
async fn handle_connection(
    mut stream: tokio::net::TcpStream,
    state: Arc<ConverterState>,
) -> Result<(), anyhow::Error> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

    let mut reader = BufReader::new(&mut stream);

    // Read the request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    let parts: Vec<&str> = request_line.trim().split(' ').collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut headers: HashMap<String, String> = HashMap::new();
    let mut content_length: usize = 0;

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            if key == "content-length" {
                content_length = value.parse().unwrap_or(0);
            }
            headers.insert(key, value);
        }
    }

    // Read body if present
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }

    // Route the request
    let response = match (method, path) {
        ("GET", "/health") => handlers::health_handler(&state).await,
        ("GET", p) if p.starts_with("/asset/") => {
            let hash = p.trim_start_matches("/asset/");
            handlers::get_asset_handler(&state, hash).await
        }
        ("POST", "/convert/gltf") => handlers::convert_gltf_handler(&state, &headers, &body).await,
        ("POST", "/convert/texture") => {
            handlers::convert_texture_handler(&state, &headers, &body).await
        }
        ("POST", "/package/scene") => {
            handlers::package_scene_handler(&state, &headers, &body).await
        }
        ("DELETE", "/cache") => handlers::clear_cache_handler(&state).await,
        _ => handlers::not_found_handler().await,
    };

    // Write response
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}
