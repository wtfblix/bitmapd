use axum::extract::{Path, State};
use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use crate::modules::database::Database;

/// Shared application state
struct AppState {
    db: Arc<Database>,
}

#[derive(Serialize)]
pub struct ParcelInfo {
    pub index: u64,
    pub inscription_id: String,
}

#[derive(Serialize)]
pub struct DistrictResponse {
    pub district: u64,
    pub inscription_id: String,
    pub parcels: Vec<ParcelInfo>,
}

pub async fn run_server(db: Arc<Database>) -> anyhow::Result<()> {
    let state = Arc::new(AppState { db });

    let app = Router::new()
        .route("/district/:id", get(get_district))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("📡 bitmapd API running on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_district(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<DistrictResponse>, (axum::http::StatusCode, String)> {
    // 1. Fetch District
    let inscription_id = state.db.get_district(id).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let inscription_id = match inscription_id {
        Some(id) => id,
        None => return Err((axum::http::StatusCode::NOT_FOUND, format!("District {} not found", id))),
    };

    // 2. Fetch Parcels
    let parcels_raw = state.db.get_parcels(id).map_err(|e| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let parcels = parcels_raw
        .into_iter()
        .map(|(index, inscription_id)| ParcelInfo { index, inscription_id })
        .collect();

    Ok(Json(DistrictResponse {
        district: id,
        inscription_id,
        parcels,
    }))
}