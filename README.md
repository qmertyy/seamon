# Rust Sea Spy

A minimal ship tracking application built in Rust that displays real-time AIS (Automatic Identification System) data on a map using MapLibre GL with OpenStreetMap and OpenSeaMap layers.

## Features

- Real-time AIS data streaming from aisstream.io
- Ships displayed as triangular markers pointing in their heading direction
- Color-coded ships (green for moving, red for stationary)
- Geohash-based spatial indexing for efficient querying
- MapLibre GL with OpenStreetMap base layer and OpenSeaMap nautical overlay
- Ship information popup on hover/click
- Automatic cache cleanup for old ship data
- Responsive web interface

## Prerequisites

- Rust (latest stable version)
- AIS Stream API key from [aisstream.io](https://aisstream.io)

## Setup

1. **Clone or create the project:**
   ```bash
   cargo new rust-seaspy
   cd rust-seaspy
   ```

2. **Set up the project structure:**
   ```
   rust-seaspy/
   ├── Cargo.toml
   ├── src/
   │   ├── main.rs
   │   ├── ais.rs
   │   └── ship.rs
   ├── static/
   │   └── index.html
   └── README.md
   ```

3. **Set your AIS Stream API key:**
   ```bash
   export AIS_STREAM_API_KEY="your_api_key_here"
   ```

4. **Create the static directory:**
   ```bash
   mkdir static
   ```

5. **Run the application:**
   ```bash
   cargo run
   ```

6. **Open your browser:**
   Navigate to [http://127.0.0.1:8080](http://127.0.0.1:8080)

## How It Works

### Backend Architecture

The Rust backend consists of several key components:

1. **AIS Stream Handler** (`ais.rs`):
   - Connects to aisstream.io WebSocket
   - Handles authentication and message parsing
   - Processes both PositionReport and ShipStaticData messages

2. **Ship Management** (`ship.rs`):
   - Maintains ship state in memory
   - Uses geohash indexing for spatial queries
   - Provides bounding box queries for map viewport

3. **Web Server** (`main.rs`):
   - Axum-based HTTP server
   - Serves static files and provides REST API
   - Handles ship data queries by geographic bounds

### Frontend

- **MapLibre GL**: Modern web mapping library
- **OpenStreetMap**: Base map layer
- **OpenSeaMap**: Nautical charts overlay
- **Real-time updates**: Ships update every 10 seconds
- **Interactive**: Click ships for detailed information

### Geohashing

Ships are indexed using geohash strings for efficient spatial queries:
- Each ship position is converted to a geohash
- Bounding box queries use geohash prefixes to quickly find candidate ships
- Results are filtered by exact coordinates

## API Endpoints

- `GET /` - Main application page
- `GET /api/ships/{sw_lat}/{sw_lng}/{ne_lat}/{ne_lng}` - Get ships in bounding box
- `GET /api/ship/{mmsi}` - Get detailed ship information
- `GET /static/*` - Static file serving

## Configuration

The application uses sensible defaults but can be customized:

- **Port**: Server runs on port 8080
- **Cleanup interval**: Ships not seen for 24 hours are removed
- **Update frequency**: Frontend updates every 10 seconds
- **Geohash precision**: 6 characters for spatial indexing

## Development Notes

This is a minimal working example. For production use, consider:

- Persistent storage (PostgreSQL with PostGIS)
- Better error handling and logging
- Rate limiting and authentication
- Ship history tracking
- More sophisticated geohash range queries
- WebSocket updates instead of polling
- Clustering for high-density areas

## Dependencies

Key Rust crates used:
- `tokio` - Async runtime
- `axum` - Web framework
- `tokio-tungstenite` - WebSocket client
- `serde` - JSON serialization
- `geohash` - Spatial indexing
- `anyhow` - Error handling

## Troubleshooting

**WebSocket connection issues:**
- Verify your AIS_STREAM_API_KEY is set correctly
- Check network connectivity to aisstream.io

**No ships appearing:**
- Wait a few minutes for data to populate
- Check browser developer console for errors
- Ensure the backend is receiving AIS messages (check terminal output)

**Performance issues:**
- Reduce update frequency if needed
- Consider implementing ship clustering for high-density areas

## License

This project is for educational purposes. Respect aisstream.io's terms of service and rate limits.