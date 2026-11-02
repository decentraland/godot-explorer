//! HTTP server for the asset optimization server.
//!
//! Uses hyper 1.0 RC for the HTTP server.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use tokio::net::TcpListener;

use super::handlers::{
    handle_batch_status, handle_health, handle_job_status, handle_jobs, handle_process,
    handle_process_scene,
};
use super::job_manager::JobManager;
use super::processor::{create_default_context, ProcessorContext};
use super::types::{ProcessRequest, ProcessSceneRequest};

/// Asset optimization server.
pub struct AssetServer {
    port: u16,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
}

impl AssetServer {
    /// Create a new asset server.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            job_manager: Arc::new(JobManager::new()),
            ctx: create_default_context(),
        }
    }

    /// Run the server.
    ///
    /// This function blocks until the server is shut down.
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Asset server listening on http://{}", addr);
        println!("Asset Optimization Server listening on http://{}", addr);
        println!("Endpoints:");
        println!("  POST /process              - Submit assets for processing");
        println!("  POST /process-scene        - Process all assets from a scene entity");
        println!(
            "  GET  /status/{{batch_id}}    - Get batch status (includes all jobs and ZIP path)"
        );
        println!("  GET  /status/job/{{job_id}}  - Get individual job status");
        println!("  GET  /jobs                 - List all jobs and batches");
        println!("  GET  /health               - Health check");

        // Spawn cleanup task
        let job_manager_cleanup = self.job_manager.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
                job_manager_cleanup.cleanup_old_jobs().await;
            }
        });

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = hyper_util::rt::TokioIo::new(stream);

            let job_manager = self.job_manager.clone();
            let ctx = self.ctx.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let job_manager = job_manager.clone();
                    let ctx = ctx.clone();
                    async move { handle_request(req, job_manager, ctx, remote_addr).await }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    tracing::error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

/// Handle an HTTP request.
async fn handle_request(
    req: Request<hyper::body::Incoming>,
    job_manager: Arc<JobManager>,
    ctx: ProcessorContext,
    remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    tracing::debug!("{} {} from {}", method, path, remote_addr);

    let response = match (method, path.as_str()) {
        (Method::GET, "/health") => {
            let health = handle_health();
            json_response(StatusCode::OK, &health)
        }

        (Method::GET, "/jobs") => {
            let jobs = handle_jobs(job_manager).await;
            json_response(StatusCode::OK, &jobs)
        }

        (Method::GET, path) if path.starts_with("/status/job/") => {
            // Individual job status: GET /status/job/{job_id}
            let job_id = path.strip_prefix("/status/job/").unwrap_or("");
            if job_id.is_empty() {
                error_response(StatusCode::BAD_REQUEST, "Missing job ID")
            } else {
                match handle_job_status(job_id.to_string(), job_manager).await {
                    Ok(status) => json_response(StatusCode::OK, &status),
                    Err(e) => error_response(StatusCode::NOT_FOUND, &e),
                }
            }
        }

        (Method::GET, path) if path.starts_with("/status/") => {
            // Batch status: GET /status/{batch_id}
            let batch_id = path.strip_prefix("/status/").unwrap_or("");
            if batch_id.is_empty() {
                error_response(StatusCode::BAD_REQUEST, "Missing batch ID")
            } else {
                match handle_batch_status(batch_id.to_string(), job_manager).await {
                    Ok(status) => json_response(StatusCode::OK, &status),
                    Err(e) => error_response(StatusCode::NOT_FOUND, &e),
                }
            }
        }

        (Method::POST, "/process") => {
            // Read request body
            let body_bytes = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Failed to read body: {}", e),
                    ));
                }
            };

            // Parse JSON
            let request: ProcessRequest = match serde_json::from_slice(&body_bytes) {
                Ok(req) => req,
                Err(e) => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Invalid JSON: {}", e),
                    ));
                }
            };

            match handle_process(request, job_manager, ctx).await {
                Ok(response) => json_response(StatusCode::ACCEPTED, &response),
                Err(e) => error_response(StatusCode::BAD_REQUEST, &e),
            }
        }

        (Method::POST, "/process-scene") => {
            // Read request body
            let body_bytes = match req.collect().await {
                Ok(collected) => collected.to_bytes(),
                Err(e) => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Failed to read body: {}", e),
                    ));
                }
            };

            // Parse JSON
            let request: ProcessSceneRequest = match serde_json::from_slice(&body_bytes) {
                Ok(req) => req,
                Err(e) => {
                    return Ok(error_response(
                        StatusCode::BAD_REQUEST,
                        &format!("Invalid JSON: {}", e),
                    ));
                }
            };

            match handle_process_scene(request, job_manager, ctx).await {
                Ok(response) => json_response(StatusCode::ACCEPTED, &response),
                Err(e) => error_response(StatusCode::BAD_REQUEST, &e),
            }
        }

        (Method::OPTIONS, _) => {
            // CORS preflight
            cors_preflight_response()
        }

        _ => error_response(StatusCode::NOT_FOUND, "Not found"),
    };

    Ok(add_cors_headers(response))
}

/// Create a JSON response.
fn json_response<T: serde::Serialize>(status: StatusCode, data: &T) -> Response<Full<Bytes>> {
    let json = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

/// Create an error response.
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let json = serde_json::json!({ "error": message });
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(json.to_string())))
        .unwrap()
}

/// Create a CORS preflight response.
fn cors_preflight_response() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Max-Age", "86400")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Add CORS headers to a response.
fn add_cors_headers(mut response: Response<Full<Bytes>>) -> Response<Full<Bytes>> {
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, POST, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type".parse().unwrap(),
    );
    response
}

/// Start the asset server on the given port.
///
/// This function is meant to be called from Godot/GDScript.
pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = AssetServer::new(port);
    server.run().await
}
