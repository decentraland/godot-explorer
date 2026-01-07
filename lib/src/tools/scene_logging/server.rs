//! HTTP server for the scene logging web frontend.

use super::{config::SceneLoggingConfig, get_stats, LoggingStats};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub log_directory: PathBuf,
    pub stats: Arc<RwLock<LoggingStats>>,
    pub current_session_id: String,
}

/// Query parameters for paginated requests.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query parameters for message filtering.
#[derive(Debug, Deserialize)]
pub struct MessageFilterParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub component: Option<u32>,
    pub entity: Option<u32>,
    pub tick_from: Option<u32>,
    pub tick_to: Option<u32>,
    pub operation: Option<String>,
}

/// Query parameters for op call filtering.
#[derive(Debug, Deserialize)]
pub struct OpFilterParams {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub op_name: Option<String>,
    pub is_async: Option<bool>,
    pub has_error: Option<bool>,
}

/// Paginated response wrapper.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

/// Session metadata.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub file_path: String,
    pub file_size: u64,
    pub is_current: bool,
}

/// Entity summary.
#[derive(Debug, Serialize)]
pub struct EntitySummary {
    pub entity_id: u32,
    pub entity_number: u16,
    pub entity_version: u16,
    pub first_seen_tick: u32,
    pub last_seen_tick: u32,
    pub components: Vec<String>,
    pub message_count: u64,
}

/// Stats response.
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub session_id: String,
    pub total_crdt_messages: u64,
    pub total_op_calls: u64,
    pub unique_entities: usize,
    pub component_distribution: HashMap<String, u64>,
}

/// Starts the HTTP server for the scene logging web frontend.
pub async fn start_server(config: &SceneLoggingConfig, current_session_id: String) {
    let state = AppState {
        log_directory: config.log_directory.clone(),
        stats: get_stats(),
        current_session_id,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Static frontend
        .route("/", get(serve_index))
        .route("/app.js", get(serve_app_js))
        .route("/styles.css", get(serve_styles_css))
        // API routes
        .route("/api/v1/sessions", get(list_sessions))
        .route("/api/v1/sessions/:id", get(get_session))
        .route("/api/v1/sessions/:id/stats", get(get_session_stats))
        .route("/api/v1/sessions/:id/entities", get(list_entities))
        .route("/api/v1/sessions/:id/entities/:eid", get(get_entity))
        .route("/api/v1/sessions/:id/messages", get(list_messages))
        .route("/api/v1/sessions/:id/op-calls", get(list_op_calls))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.server_port));
    tracing::info!("Scene logging server started at http://{}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind scene logging server: {}", e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Scene logging server error: {}", e);
    }
}

// Static file handlers
async fn serve_index() -> impl IntoResponse {
    Html(include_str!("web/static/index.html"))
}

async fn serve_app_js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        include_str!("web/static/app.js"),
    )
}

async fn serve_styles_css() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css")],
        include_str!("web/static/styles.css"),
    )
}

// API handlers

async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    let mut sessions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&state.log_directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let session_id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);

                sessions.push(SessionInfo {
                    is_current: session_id == state.current_session_id,
                    session_id,
                    file_path: path.to_string_lossy().to_string(),
                    file_size,
                });
            }
        }
    }

    // Sort by current first, then by file size
    sessions.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then(b.file_size.cmp(&a.file_size))
    });

    Json(sessions)
}

async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

    Ok(Json(SessionInfo {
        session_id: session_id.clone(),
        file_path: file_path.to_string_lossy().to_string(),
        file_size,
        is_current: session_id == state.current_session_id,
    }))
}

async fn get_session_stats(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    // For current session, use live stats
    if session_id == state.current_session_id {
        let stats = state.stats.read().await;
        return Ok(Json(StatsResponse {
            session_id,
            total_crdt_messages: stats.total_crdt_messages,
            total_op_calls: stats.total_op_calls,
            unique_entities: 0, // Would need to track this
            component_distribution: HashMap::new(),
        }));
    }

    // For historical sessions, scan the file
    let (crdt_count, op_count, component_dist) = scan_session_stats(&file_path);

    Ok(Json(StatsResponse {
        session_id,
        total_crdt_messages: crdt_count,
        total_op_calls: op_count,
        unique_entities: 0,
        component_distribution: component_dist,
    }))
}

async fn list_entities(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let entities = scan_entities(&file_path, limit, offset);
    let total = entities.len();

    Ok(Json(PaginatedResponse {
        has_more: total == limit,
        data: entities,
        total,
        offset,
        limit,
    }))
}

async fn get_entity(
    State(state): State<AppState>,
    Path((session_id, entity_id)): Path<(String, u32)>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    let messages = scan_entity_messages(&file_path, entity_id);

    Ok(Json(messages))
}

async fn list_messages(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<MessageFilterParams>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let messages = scan_messages(&file_path, &params, limit, offset);
    let total = messages.len();

    Ok(Json(PaginatedResponse {
        has_more: total == limit,
        data: messages,
        total,
        offset,
        limit,
    }))
}

async fn list_op_calls(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<OpFilterParams>,
) -> impl IntoResponse {
    let file_path = state.log_directory.join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err((StatusCode::NOT_FOUND, "Session not found"));
    }

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    let calls = scan_op_calls(&file_path, &params, limit, offset);
    let total = calls.len();

    Ok(Json(PaginatedResponse {
        has_more: total == limit,
        data: calls,
        total,
        offset,
        limit,
    }))
}

// File scanning helpers

fn scan_session_stats(file_path: &PathBuf) -> (u64, u64, HashMap<String, u64>) {
    let mut crdt_count = 0u64;
    let mut op_count = 0u64;
    let mut component_dist: HashMap<String, u64> = HashMap::new();

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                match entry.get("type").and_then(|t| t.as_str()) {
                    Some("crdt") => {
                        crdt_count += 1;
                        if let Some(name) = entry.get("component_name").and_then(|n| n.as_str()) {
                            *component_dist.entry(name.to_string()).or_insert(0) += 1;
                        }
                    }
                    Some("op_call_start") | Some("op_call_end") => op_count += 1,
                    _ => {}
                }
            }
        }
    }

    (crdt_count, op_count, component_dist)
}

fn scan_entities(file_path: &PathBuf, limit: usize, offset: usize) -> Vec<EntitySummary> {
    let mut entities: HashMap<u32, EntitySummary> = HashMap::new();

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                if entry.get("type").and_then(|t| t.as_str()) == Some("crdt") {
                    if let (
                        Some(entity_id),
                        Some(entity_number),
                        Some(entity_version),
                        Some(tick),
                        Some(component_name),
                    ) = (
                        entry.get("entity_id").and_then(|e| e.as_u64()),
                        entry.get("entity_number").and_then(|e| e.as_u64()),
                        entry.get("entity_version").and_then(|e| e.as_u64()),
                        entry.get("tick").and_then(|t| t.as_u64()),
                        entry.get("component_name").and_then(|n| n.as_str()),
                    ) {
                        let entity_id = entity_id as u32;
                        let tick = tick as u32;

                        let summary = entities.entry(entity_id).or_insert(EntitySummary {
                            entity_id,
                            entity_number: entity_number as u16,
                            entity_version: entity_version as u16,
                            first_seen_tick: tick,
                            last_seen_tick: tick,
                            components: Vec::new(),
                            message_count: 0,
                        });

                        summary.last_seen_tick = summary.last_seen_tick.max(tick);
                        summary.first_seen_tick = summary.first_seen_tick.min(tick);
                        summary.message_count += 1;

                        if !summary.components.contains(&component_name.to_string()) {
                            summary.components.push(component_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut entities: Vec<_> = entities.into_values().collect();
    entities.sort_by_key(|e| e.entity_id);
    entities.into_iter().skip(offset).take(limit).collect()
}

fn scan_entity_messages(file_path: &PathBuf, entity_id: u32) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                if entry.get("type").and_then(|t| t.as_str()) == Some("crdt") {
                    if let Some(eid) = entry.get("entity_id").and_then(|e| e.as_u64()) {
                        if eid as u32 == entity_id {
                            messages.push(entry);
                        }
                    }
                }
            }
        }
    }

    messages
}

fn scan_messages(
    file_path: &PathBuf,
    params: &MessageFilterParams,
    limit: usize,
    offset: usize,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::new();
    let mut skipped = 0;

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                if entry.get("type").and_then(|t| t.as_str()) != Some("crdt") {
                    continue;
                }

                // Apply filters
                if let Some(component_filter) = params.component {
                    if entry.get("component_id").and_then(|c| c.as_u64())
                        != Some(component_filter as u64)
                    {
                        continue;
                    }
                }

                if let Some(entity_filter) = params.entity {
                    if entry.get("entity_id").and_then(|e| e.as_u64()) != Some(entity_filter as u64)
                    {
                        continue;
                    }
                }

                if let Some(tick_from) = params.tick_from {
                    if let Some(tick) = entry.get("tick").and_then(|t| t.as_u64()) {
                        if (tick as u32) < tick_from {
                            continue;
                        }
                    }
                }

                if let Some(tick_to) = params.tick_to {
                    if let Some(tick) = entry.get("tick").and_then(|t| t.as_u64()) {
                        if (tick as u32) > tick_to {
                            continue;
                        }
                    }
                }

                if let Some(ref op_filter) = params.operation {
                    if entry.get("operation").and_then(|o| o.as_str()) != Some(op_filter) {
                        continue;
                    }
                }

                // Apply pagination
                if skipped < offset {
                    skipped += 1;
                    continue;
                }

                messages.push(entry);

                if messages.len() >= limit {
                    break;
                }
            }
        }
    }

    messages
}

fn scan_op_calls(
    file_path: &PathBuf,
    params: &OpFilterParams,
    limit: usize,
    offset: usize,
) -> Vec<serde_json::Value> {
    let mut calls = Vec::new();
    let mut skipped = 0;

    if let Ok(file) = File::open(file_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                let entry_type = entry.get("type").and_then(|t| t.as_str());
                if entry_type != Some("op_call_start") && entry_type != Some("op_call_end") {
                    continue;
                }

                // Apply filters
                if let Some(ref op_name_filter) = params.op_name {
                    if let Some(op_name) = entry.get("op_name").and_then(|n| n.as_str()) {
                        if !op_name.contains(op_name_filter) {
                            continue;
                        }
                    }
                }

                if let Some(is_async_filter) = params.is_async {
                    if entry.get("is_async").and_then(|a| a.as_bool()) != Some(is_async_filter) {
                        continue;
                    }
                }

                if let Some(has_error_filter) = params.has_error {
                    let has_error = entry.get("error").map(|e| !e.is_null()).unwrap_or(false);
                    if has_error != has_error_filter {
                        continue;
                    }
                }

                // Apply pagination
                if skipped < offset {
                    skipped += 1;
                    continue;
                }

                calls.push(entry);

                if calls.len() >= limit {
                    break;
                }
            }
        }
    }

    calls
}
