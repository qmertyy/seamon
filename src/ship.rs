use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ship {
    pub mmsi: u32,
    pub name: String,
    pub lat: f64,
    pub lng: f64,
    pub heading: u32,
    pub speed: f64,
    pub nav_status: u32,
    pub ship_type: u32,
    pub destination: String,
    pub imo_number: u32,
    pub last_update: u64,
    pub geohash: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShipState {
    pub mmsi: u32,
    pub name: String,
    pub lat: f64,
    pub lng: f64,
    pub heading: u32,
    pub speed: f64,
    pub ship_type: u32,
    pub last_update: u64,
}

pub struct ShipCache {
    pub ships: HashMap<u32, Ship>,
    pub geohash_index: HashMap<String, Vec<u32>>, // geohash -> list of MMSIs
}

impl Ship {
    pub fn new(mmsi: u32, name: String) -> Self {
        Self {
            mmsi,
            name,
            lat: 0.0,
            lng: 0.0,
            heading: 0,
            speed: 0.0,
            nav_status: 0,
            ship_type: 0,
            destination: String::new(),
            imo_number: 0,
            last_update: 0,
            geohash: String::new(),
        }
    }

    pub fn to_state(&self) -> ShipState {
        ShipState {
            mmsi: self.mmsi,
            name: self.name.clone(),
            lat: self.lat,
            lng: self.lng,
            heading: self.heading,
            speed: self.speed,
            ship_type: self.ship_type,
            last_update: self.last_update,
        }
    }
}

impl ShipCache {
    pub fn new() -> Self {
        Self {
            ships: HashMap::new(),
            geohash_index: HashMap::new(),
        }
    }

    pub fn update_geohash_index(&mut self, mmsi: u32, geohash: &str) {
        // Remove from old geohash entry
        self.geohash_index.retain(|_, ships| {
            ships.retain(|&m| m != mmsi);
            !ships.is_empty()
        });

        // Add to new geohash entry
        let geohash_prefix = &geohash[..std::cmp::min(6, geohash.len())]; // Use 6-char precision
        self.geohash_index
            .entry(geohash_prefix.to_string())
            .or_insert_with(Vec::new)
            .push(mmsi);
    }

    pub fn get_ships_in_bbox(&self, sw_lat: f64, sw_lng: f64, ne_lat: f64, ne_lng: f64) -> Vec<ShipState> {
        let mut result = Vec::new();

        // Generate geohashes for the bounding box corners and find relevant prefixes
        let sw_geohash = geohash::encode(geohash::Coord { x: sw_lng, y: sw_lat }, 6).unwrap_or_default();
        let ne_geohash = geohash::encode(geohash::Coord { x: ne_lng, y: ne_lat }, 6).unwrap_or_default();
        
      
        let mut candidate_mmsis = std::collections::HashSet::new();
        
        for (geohash_prefix, mmsis) in &self.geohash_index {
            // Simple check: if geohash starts with similar chars, include candidates
            if geohash_overlaps_bbox(geohash_prefix, sw_lat, sw_lng, ne_lat, ne_lng) {
                for &mmsi in mmsis {
                    candidate_mmsis.insert(mmsi);
                }
            }
        }

        // Filter candidates by exact bounding box
        for &mmsi in &candidate_mmsis {
            if let Some(ship) = self.ships.get(&mmsi) {
                if ship.lat >= sw_lat && ship.lat <= ne_lat &&
                   ship.lng >= sw_lng && ship.lng <= ne_lng &&
                   ship.lat != 0.0 && ship.lng != 0.0 { // Filter out invalid positions
                    result.push(ship.to_state());
                }
            }
        }

        result
    }
}


fn geohash_overlaps_bbox(geohash_prefix: &str, sw_lat: f64, sw_lng: f64, ne_lat: f64, ne_lng: f64) -> bool {
    if let Ok((coord, lat_err, lng_err)) = geohash::decode(geohash_prefix) {
        let lat = coord.y;
        let lng = coord.x;
        
        // Add some margin for geohash cell overlap
        let margin = 1.0; // degrees
        lat >= (sw_lat - margin) && lat <= (ne_lat + margin) &&
        lng >= (sw_lng - margin) && lng <= (ne_lng + margin)
    } else {
        false
    }
}