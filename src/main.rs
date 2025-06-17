use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::time::{interval, Duration};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{error, info, warn, debug};
use url::Url;

mod ais;
mod ship;

use ais::{AisStream, AisMessage};
use ship::{Ship, ShipCache, ShipState};

type SharedShipCache = Arc<RwLock<ShipCache>>;

#[derive(Clone)]
struct AppState {
    ships: SharedShipCache,
}

#[tokio::main]
async fn main() -> Result<()> {
  
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

   
    let crate_name = env!("CARGO_PKG_NAME").replace('-', "_");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")) // Default to 'info' if RUST_LOG is unset/invalid
        .add_directive(format!("{}={}", crate_name, "debug").parse().unwrap());

    tracing_subscriber::registry()
        .with(fmt::layer()) // Standard formatting layer
        .with(env_filter)   // Apply the configured filter
        .init();            // Initialize the subscriber

    // Test logs
    info!("Starting Rust Seawatch - crate: '{}'", crate_name);
    debug!("Debug logging enabled for {}", crate_name);
    let ships = Arc::new(RwLock::new(ShipCache::new()));
    let app_state = AppState {
        ships: ships.clone(),
    };

    // Start AIS stream processing
    tokio::spawn(ais_stream_task(ships.clone()));
    
    // Start cache cleanup task
    tokio::spawn(cache_cleanup_task(ships.clone()));

    // Setup web server
    let app = Router::new()
        .route("/", get(index))
        .route("/api/ships/:sw_lat/:sw_lng/:ne_lat/:ne_lng", get(get_ships_in_bbox))
        .route("/api/ship/:mmsi", get(get_ship_info))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("Server running on http://127.0.0.1:8080");
    
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ais_stream_task(ships: SharedShipCache) {
    loop {
        if let Err(e) = run_ais_stream(ships.clone()).await {
            error!("AIS stream error: {}", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}

async fn run_ais_stream(ships: SharedShipCache) -> Result<()> {
    let api_key = env::var("AIS_STREAM_API_KEY")
        .map_err(|_| anyhow::anyhow!("AIS_STREAM_API_KEY environment variable not set"))?;
    
    let url = Url::parse("wss://stream.aisstream.io/v0/stream")?;
    let mut ais_stream = AisStream::connect(url, api_key).await?;
    
    info!("Connected to AIS stream");
    
    while let Some(message) = ais_stream.next_message().await? {
        process_ais_message(message, ships.clone()).await;
    }
    
    Ok(())
}

async fn process_ais_message(message: AisMessage, ships: SharedShipCache) {
    let mmsi = message.metadata.mmsi;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let geohash = geohash::encode(
        geohash::Coord {
            x: message.metadata.longitude,
            y: message.metadata.latitude,
        },
        12,
    ).unwrap();

    let mut cache = ships.write().unwrap();
    
    // Get or create ship
    let ship = cache.ships.entry(mmsi).or_insert_with(|| {
        Ship::new(mmsi, message.metadata.ship_name.clone())
    });
    
    // Update basic info
    ship.name = message.metadata.ship_name;
    ship.lat = message.metadata.latitude;
    ship.lng = message.metadata.longitude;
    ship.last_update = timestamp;
    ship.geohash = geohash.clone();

    // Update type-specific data
    match message.message_type.as_str() {
        "PositionReport" => {
            if let Some(pos_report) = message.message.position_report {
                ship.heading = pos_report.true_heading;
                ship.speed = pos_report.sog;
                ship.nav_status = pos_report.navigational_status;
            }
        }
        "ShipStaticData" => {
            if let Some(static_data) = message.message.ship_static_data {
                ship.ship_type = static_data.ship_type;
                ship.destination = static_data.destination;
                ship.imo_number = static_data.imo_number;
            }
        }
        _ => {}
    }
    
    // Update geohash index (now using the cloned geohash)
    cache.update_geohash_index(mmsi, &geohash);
}

async fn cache_cleanup_task(ships: SharedShipCache) {
    let mut interval = interval(Duration::from_secs(300)); // Cleanup every 5 minutes
    
    loop {
        interval.tick().await;
        
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut cache = ships.write().unwrap();
        let mut to_remove = Vec::new();
        
        for (&mmsi, ship) in &cache.ships {
            // Remove ships not seen for 24 hours
            if current_time - ship.last_update > 86400 {
                to_remove.push(mmsi);
            }
        }
        
        for mmsi in to_remove {
            cache.ships.remove(&mmsi);
            cache.geohash_index.retain(|_, ships| {
                ships.retain(|&m| m != mmsi);
                !ships.is_empty()
            });
        }
        
        info!("Cache cleanup completed, {} ships remaining", cache.ships.len());
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn get_ships_in_bbox(
    Path((sw_lat, sw_lng, ne_lat, ne_lng)): Path<(f64, f64, f64, f64)>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ShipState>>, StatusCode> {
    let cache = state.ships.read().unwrap();
    
    let ships = cache.get_ships_in_bbox(sw_lat, sw_lng, ne_lat, ne_lng);
    
    Ok(Json(ships))
}

async fn get_ship_info(
    Path(mmsi): Path<u32>,
    State(state): State<AppState>,
) -> Result<Json<Ship>, StatusCode> {
    let cache = state.ships.read().unwrap();
    
    match cache.ships.get(&mmsi) {
        Some(ship) => Ok(Json(ship.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}