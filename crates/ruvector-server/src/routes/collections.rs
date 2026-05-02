//! Collection management endpoints

use crate::{error::Error, state::AppState, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ruvector_core::{types::DbOptions, DistanceMetric, VectorDB};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Collection creation request
#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    /// Collection name
    pub name: String,
    /// Vector dimension
    pub dimension: usize,
    /// Distance metric (optional, defaults to Cosine)
    pub metric: Option<DistanceMetric>,
}

/// Collection info response
#[derive(Debug, Serialize)]
pub struct CollectionInfo {
    /// Collection name
    pub name: String,
    /// Vector dimension
    pub dimension: usize,
    /// Distance metric
    pub metric: DistanceMetric,
}

/// List of collections response
#[derive(Debug, Serialize)]
pub struct CollectionsList {
    /// Collection names
    pub collections: Vec<String>,
}

/// Query parameters for listing collections
#[derive(Debug, Deserialize)]
pub struct ListCollectionsParams {
    /// Optional prefix filter — only collections whose names start with this value are returned
    pub prefix: Option<String>,
}

/// Create collection routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_collection).get(list_collections))
        .route("/:name", get(get_collection).delete(delete_collection))
}

/// Create a new collection
///
/// POST /collections
async fn create_collection(
    State(state): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<impl IntoResponse> {
    if state.contains_collection(&req.name) {
        return Err(Error::CollectionExists(req.name));
    }

    let mut options = DbOptions::default();
    options.dimensions = req.dimension;
    options.distance_metric = req.metric.unwrap_or(DistanceMetric::Cosine);
    // Use in-memory storage for server (storage path will be ignored for memory storage)
    options.storage_path = format!("memory://{}", req.name);

    let db = VectorDB::new(options.clone()).map_err(Error::Core)?;
    state.insert_collection(req.name.clone(), Arc::new(db));

    let info = CollectionInfo {
        name: req.name,
        dimension: req.dimension,
        metric: options.distance_metric,
    };

    Ok((StatusCode::CREATED, Json(info)))
}

/// List all collections, optionally filtered by a name prefix
///
/// GET /collections
/// GET /collections?prefix=prod
async fn list_collections(
    State(state): State<AppState>,
    Query(params): Query<ListCollectionsParams>,
) -> Result<impl IntoResponse> {
    let collections = match params.prefix.as_deref() {
        Some(p) if !p.is_empty() => state.collection_names_by_prefix(p),
        _ => state.collection_names(),
    };
    Ok(Json(CollectionsList { collections }))
}

/// Get collection information
///
/// GET /collections/:name
async fn get_collection(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    let _db = state
        .get_collection(&name)
        .ok_or_else(|| Error::CollectionNotFound(name.clone()))?;

    // Note: VectorDB doesn't expose config directly, so we return basic info
    let info = CollectionInfo {
        name,
        dimension: 0, // Would need to be stored separately or queried from DB
        metric: DistanceMetric::Cosine, // Default assumption
    };

    Ok(Json(info))
}

/// Delete a collection
///
/// DELETE /collections/:name
async fn delete_collection(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse> {
    state
        .remove_collection(&name)
        .ok_or_else(|| Error::CollectionNotFound(name))?;

    Ok(StatusCode::NO_CONTENT)
}
