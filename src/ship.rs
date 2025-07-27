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

// KD-Tree node for spatial indexing
#[derive(Debug, Clone)]
struct KdNode {
    mmsi: u32,
    lat: f64,
    lng: f64,
    left: Option<Box<KdNode>>,
    right: Option<Box<KdNode>>,
    depth: usize,
}

impl KdNode {
    fn new(mmsi: u32, lat: f64, lng: f64, depth: usize) -> Self {
        Self {
            mmsi,
            lat,
            lng,
            left: None,
            right: None,
            depth,
        }
    }

    fn dimension(&self) -> usize {
        self.depth % 2 // 0 for latitude, 1 for longitude
    }

    fn coordinate(&self, dim: usize) -> f64 {
        match dim {
            0 => self.lat,
            1 => self.lng,
            _ => unreachable!(),
        }
    }
}

// KD-Tree for fast spatial queries
#[derive(Debug)]
struct KdTree {
    root: Option<Box<KdNode>>,
}

impl KdTree {
    fn new() -> Self {
        Self { root: None }
    }

    fn build_from_ships(ships: &HashMap<u32, Ship>) -> Self {
        let mut points: Vec<(u32, f64, f64)> = ships
            .iter()
            .filter(|(_, ship)| ship.lat != 0.0 && ship.lng != 0.0) // Filter invalid positions
            .map(|(&mmsi, ship)| (mmsi, ship.lat, ship.lng))
            .collect();

        let root = Self::build_recursive(&mut points, 0);
        Self { root }
    }

    fn build_recursive(points: &mut [(u32, f64, f64)], depth: usize) -> Option<Box<KdNode>> {
        if points.is_empty() {
            return None;
        }

        let dim = depth % 2; // 0 for lat, 1 for lng

        // Sort by the current dimension
        points.sort_by(|a, b| {
            let coord_a = if dim == 0 { a.1 } else { a.2 };
            let coord_b = if dim == 0 { b.1 } else { b.2 };
            coord_a.partial_cmp(&coord_b).unwrap()
        });

        let median = points.len() / 2;
        let (mmsi, lat, lng) = points[median];

        let mut node = Box::new(KdNode::new(mmsi, lat, lng, depth));

        // Recursively build left and right subtrees
        node.left = Self::build_recursive(&mut points[..median], depth + 1);
        node.right = Self::build_recursive(&mut points[median + 1..], depth + 1);

        Some(node)
    }

    fn range_query(&self, sw_lat: f64, sw_lng: f64, ne_lat: f64, ne_lng: f64) -> Vec<u32> {
        let mut result = Vec::new();
        if let Some(ref root) = self.root {
            Self::range_query_recursive(root, sw_lat, sw_lng, ne_lat, ne_lng, &mut result);
        }
        result
    }

    fn range_query_recursive(
        node: &KdNode,
        sw_lat: f64,
        sw_lng: f64,
        ne_lat: f64,
        ne_lng: f64,
        result: &mut Vec<u32>,
    ) {
        // Check if current node is within the bounding box
        if node.lat >= sw_lat && node.lat <= ne_lat && node.lng >= sw_lng && node.lng <= ne_lng {
            result.push(node.mmsi);
        }

        let dim = node.dimension();
        let split_value = node.coordinate(dim);
        let (range_min, range_max) = if dim == 0 {
            (sw_lat, ne_lat)
        } else {
            (sw_lng, ne_lng)
        };

        // Recursively search left subtree if needed
        if let Some(ref left) = node.left {
            if range_min <= split_value {
                Self::range_query_recursive(left, sw_lat, sw_lng, ne_lat, ne_lng, result);
            }
        }

        // Recursively search right subtree if needed
        if let Some(ref right) = node.right {
            if range_max >= split_value {
                Self::range_query_recursive(right, sw_lat, sw_lng, ne_lat, ne_lng, result);
            }
        }
    }
}

pub struct ShipCache {
    pub ships: HashMap<u32, Ship>,
    kdtree: Option<KdTree>,
    dirty: bool, // Track if we need to rebuild the tree
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
            kdtree: None,
            dirty: false,
        }
    }

    pub fn insert_ship(&mut self, mmsi: u32, ship: Ship) {
        self.ships.insert(mmsi, ship);
        self.dirty = true; // Mark for rebuild
    }

    pub fn update_ship(&mut self, mmsi: u32, ship: Ship) {
        if self.ships.insert(mmsi, ship).is_some() {
            self.dirty = true; // Mark for rebuild only if ship existed
        } else {
            self.dirty = true; // New ship, mark for rebuild
        }
    }

    pub fn remove_ship(&mut self, mmsi: u32) -> Option<Ship> {
        let result = self.ships.remove(&mmsi);
        if result.is_some() {
            self.dirty = true; // Mark for rebuild
        }
        result
    }

    pub fn rebuild_index(&mut self) {
        if !self.ships.is_empty() {
            self.kdtree = Some(KdTree::build_from_ships(&self.ships));
            self.dirty = false;
        } else {
            self.kdtree = None;
            self.dirty = false;
        }
    }

    pub fn get_ships_in_bbox(
        &mut self, // Note: now takes mutable reference for lazy rebuilding
        sw_lat: f64,
        sw_lng: f64,
        ne_lat: f64,
        ne_lng: f64,
    ) -> Vec<ShipState> {
        // Rebuild index if dirty
        if self.dirty || self.kdtree.is_none() {
            self.rebuild_index();
        }

        // Use KD-tree for fast spatial query
        let mmsis = if let Some(ref kdtree) = self.kdtree {
            kdtree.range_query(sw_lat, sw_lng, ne_lat, ne_lng)
        } else {
            Vec::new()
        };

        // Convert MMSIs to ShipStates
        mmsis
            .into_iter()
            .filter_map(|mmsi| self.ships.get(&mmsi).map(|ship| ship.to_state()))
            .collect()
    }

    // Alternative immutable version that falls back to linear search if index is dirty
    pub fn get_ships_in_bbox_immutable(
        &self,
        sw_lat: f64,
        sw_lng: f64,
        ne_lat: f64,
        ne_lng: f64,
    ) -> Vec<ShipState> {
        if !self.dirty && self.kdtree.is_some() {
            // Use KD-tree for fast query
            let mmsis = self
                .kdtree
                .as_ref()
                .unwrap()
                .range_query(sw_lat, sw_lng, ne_lat, ne_lng);
            mmsis
                .into_iter()
                .filter_map(|mmsi| self.ships.get(&mmsi).map(|ship| ship.to_state()))
                .collect()
        } else {
            // Fall back to linear search (original implementation)
            let mut result = Vec::new();
            for ship in self.ships.values() {
                if ship.lat >= sw_lat
                    && ship.lat <= ne_lat
                    && ship.lng >= sw_lng
                    && ship.lng <= ne_lng
                    && ship.lat != 0.0
                    && ship.lng != 0.0
                {
                    result.push(ship.to_state());
                }
            }
            result
        }
    }

    pub fn force_rebuild(&mut self) {
        self.dirty = true;
        self.rebuild_index();
    }

    pub fn len(&self) -> usize {
        self.ships.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ships.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn create_test_ship(mmsi: u32, name: &str, lat: f64, lng: f64) -> Ship {
        Ship {
            mmsi,
            name: name.to_string(),
            lat,
            lng,
            heading: 0,
            speed: 0.0,
            nav_status: 0,
            ship_type: 0,
            destination: String::new(),
            imo_number: 0,
            last_update: 0,
        }
    }

    fn create_test_cache() -> ShipCache {
        let mut cache = ShipCache::new();

        cache.insert_ship(1, create_test_ship(1, "NYC Ship", 40.7128, -74.0060));
        cache.insert_ship(2, create_test_ship(2, "London Ship", 51.5074, -0.1278));
        cache.insert_ship(3, create_test_ship(3, "Tokyo Ship", 35.6762, 139.6503));
        cache.insert_ship(4, create_test_ship(4, "Sydney Ship", -33.8688, 151.2093));
        cache.insert_ship(5, create_test_ship(5, "Invalid Ship", 0.0, 0.0));
        cache.insert_ship(6, create_test_ship(6, "Near NYC", 40.7500, -73.9000));

        cache
    }

    #[test]
    fn test_kdtree_correctness() {
        let mut cache = create_test_cache();

        // Test NYC area
        let result = cache.get_ships_in_bbox(40.5, -74.5, 41.0, -73.5);
        assert_eq!(result.len(), 2);
        let mmsis: Vec<u32> = result.iter().map(|s| s.mmsi).collect();
        assert!(mmsis.contains(&1));
        assert!(mmsis.contains(&6));
    }

    #[test]
    fn test_kdtree_vs_linear_performance() {
        let mut cache = ShipCache::new();

        // Create 50,000 ships for meaningful comparison
        for i in 0..50_000 {
            let lat = ((i * 7) % 180) as f64 - 90.0;
            let lng = ((i * 11) % 360) as f64 - 180.0;
            if lat != 0.0 || lng != 0.0 {
                let name = format!("Ship_{}", i);
                cache.insert_ship(i, create_test_ship(i, &name, lat, lng));
            }
        }

        println!("Created {} ships in cache", cache.len());

        // Test KD-tree performance (with rebuild)
        let start = Instant::now();
        let kdtree_result = cache.get_ships_in_bbox(40.0, -75.0, 41.0, -73.0);
        let kdtree_duration = start.elapsed();

        // Test linear search performance
        let start = Instant::now();
        let linear_result = cache.get_ships_in_bbox_immutable(40.0, -75.0, 41.0, -73.0);
        let linear_duration = start.elapsed();

        println!("KD-tree query time: {:?}", kdtree_duration);
        println!("Linear search time: {:?}", linear_duration);
        println!("KD-tree found {} ships", kdtree_result.len());
        println!("Linear search found {} ships", linear_result.len());

        // Results should be the same
        assert_eq!(kdtree_result.len(), linear_result.len());

        if linear_duration > Duration::from_millis(1) {
            println!(
                "Speedup: {:.2}x",
                linear_duration.as_nanos() as f64 / kdtree_duration.as_nanos() as f64
            );
        }
    }

    #[test]
    fn test_multiple_queries_performance() {
        let mut cache = ShipCache::new();

        // Create test dataset
        for i in 0..100_000 {
            let lat = ((i * 7) % 180) as f64 - 90.0;
            let lng = ((i * 11) % 360) as f64 - 180.0;
            if lat != 0.0 || lng != 0.0 {
                let name = format!("Ship_{}", i);
                cache.insert_ship(i, create_test_ship(i, &name, lat, lng));
            }
        }

        println!("Testing with {} ships", cache.len());

        // Build index once
        cache.rebuild_index();

        // Multiple queries (index already built)
        let queries = vec![
            (40.0, -75.0, 41.0, -73.0),   // NYC area
            (51.0, -1.0, 52.0, 1.0),      // London area
            (35.0, 139.0, 36.0, 140.0),   // Tokyo area
            (-34.0, 151.0, -33.0, 152.0), // Sydney area
        ];

        let start = Instant::now();
        let mut total_results = 0;
        for _ in 0..100000 {
            for (sw_lat, sw_lng, ne_lat, ne_lng) in &queries {
                let result = cache.get_ships_in_bbox(*sw_lat, *sw_lng, *ne_lat, *ne_lng);
                total_results += result.len();
            }
        }

        let kdtree_duration = start.elapsed();

        println!(
            "KD-tree: {} queries in {:?} (avg: {:?})",
            queries.len() * 10000,
            kdtree_duration,
            kdtree_duration / queries.len() as u32
        );
        println!("Total results: {}", total_results);

        // KD-tree should be very fast for subsequent queries
        assert!(kdtree_duration < Duration::from_millis(100));
    }

    #[test]
    fn test_index_rebuild_on_updates() {
        let mut cache = ShipCache::new();

        // Add initial ships
        cache.insert_ship(1, create_test_ship(1, "Ship1", 40.0, -74.0));
        cache.insert_ship(2, create_test_ship(2, "Ship2", 41.0, -73.0));

        // Query to build index
        let result1 = cache.get_ships_in_bbox(39.0, -75.0, 42.0, -72.0);
        assert_eq!(result1.len(), 2);

        // Add more ships
        cache.insert_ship(3, create_test_ship(3, "Ship3", 40.5, -73.5));

        // Index should be rebuilt and include new ship
        let result2 = cache.get_ships_in_bbox(39.0, -75.0, 42.0, -72.0);
        assert_eq!(result2.len(), 3);
    }
}
