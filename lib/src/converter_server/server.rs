/*
 * HTTP Server for Content Converter
 *
 * Runs an HTTP server that accepts file uploads and converts them to
 * optimized Godot resources for mobile platforms.
 *
 * Reuses the existing content_provider pipeline for GLTF and texture conversion.
 */

use godot::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::sync::Semaphore;

use crate::content::content_provider::SceneGltfContext;
use crate::content::resource_provider::ResourceProvider;
use crate::godot_classes::dcl_config::TextureQuality;
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
    pub resource_provider: Arc<ResourceProvider>,
    pub godot_single_thread: Arc<Semaphore>,
    pub texture_quality: TextureQuality,
}

impl ConverterState {
    pub fn new(cache_folder: PathBuf, port: u16) -> Self {
        // Create cache folder if it doesn't exist
        if !cache_folder.exists() {
            std::fs::create_dir_all(&cache_folder).ok();
        }

        let cache_folder_str = cache_folder.to_string_lossy().to_string();

        // Create ResourceProvider for managing downloads and file caching
        let resource_provider = Arc::new(ResourceProvider::new(
            &cache_folder_str,
            2048 * 1000 * 1000, // 2GB cache
            32,                 // Max concurrent downloads
        ));

        Self {
            cache_folder,
            assets: RwLock::new(HashMap::new()),
            port,
            resource_provider,
            godot_single_thread: Arc::new(Semaphore::new(1)),
            texture_quality: TextureQuality::Low, // Mobile-optimized by default
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

    /// Create a SceneGltfContext for use with the GLTF pipeline
    pub fn create_gltf_context(&self) -> SceneGltfContext {
        SceneGltfContext {
            content_folder: Arc::new(self.cache_folder.to_string_lossy().to_string()),
            resource_provider: self.resource_provider.clone(),
            godot_single_thread: self.godot_single_thread.clone(),
            texture_quality: self.texture_quality.clone(),
        }
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
    running: bool,
}

#[godot_api]
impl INode for ConverterServer {
    fn ready(&mut self) {
        // Check command line args for converter server mode
        let args = godot::classes::Os::singleton().get_cmdline_args();
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
            let user_path = godot::classes::Os::singleton().get_user_data_dir();
            PathBuf::from(user_path.to_string())
                .join(self.cache_folder.to_string().trim_start_matches("user://"))
        } else {
            PathBuf::from(self.cache_folder.to_string())
        };

        let state = Arc::new(ConverterState::new(cache_path, self.port));
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
        self.state = None;
    }

    #[func]
    pub fn is_running(&self) -> bool {
        self.running
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
