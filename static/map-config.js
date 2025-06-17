// Simplified Maritime Map Configuration
const map = new maplibregl.Map({
    container: 'map',
    style: {
        version: 8,
        glyphs: "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
        sources: {
            // OpenStreetMap base layer
            'openstreetmap': {
                type: 'raster',
                tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
                tileSize: 256,
                attribution: '© OpenStreetMap contributors'
            },
            
            // OpenSeaMap overlay
            'openseamap': {
                type: 'raster',
                tiles: ['https://tiles.openseamap.org/seamark/{z}/{x}/{y}.png'],
                tileSize: 256,
                attribution: '© OpenSeaMap contributors'
            },
            
            // Custom maritime polygons
            'maritime-tiles': {
                type: 'vector',
                tiles: ['http://localhost:65000/images/{z}/{x}/{y}.pbf'],
                minzoom: 0,
                maxzoom: 12,
                tileSize: 512,
                scheme: 'xyz'
            }
        },
        layers: [
            // Base OpenStreetMap layer
            {
                id: 'openstreetmap',
                type: 'raster',
                source: 'openstreetmap',
                minzoom: 0,
                maxzoom: 18
            },
            
            // OpenSeaMap overlay
            {
                id: 'openseamap',
                type: 'raster',
                source: 'openseamap',
                minzoom: 0,
                maxzoom: 18
            },
            
            // Maritime polygon layers
            ...createMaritimePolygonLayers()
        ]
    },
    center: [0, 20],
    zoom: 3,
    minZoom: 2,
    maxZoom: 18
});

// Create maritime polygon layers
function createMaritimePolygonLayers() {
    return [
        // Movement areas
        {
            'id': 'movement-areas',
            'type': 'fill',
            'source': 'maritime-tiles',
            'source-layer': 'merged_polygons',
            'filter': ['==', 'polygon_type', 'movement'],
            'paint': {
                'fill-color': ['case', ['==', ['get', 'maneuver_area'], true], '#f59e0b', '#fb923c'],
                'fill-opacity': 0.6
            }
        },
        
        // Stopping areas
        {
            'id': 'stopping-areas',
            'type': 'fill',
            'source': 'maritime-tiles',
            'source-layer': 'merged_polygons',
            'filter': ['==', 'polygon_type', 'stopping'],
            'paint': {
                'fill-color': ['case', ['==', ['get', 'maneuver_area'], true], '#8b5cf6', '#3b82f6'],
                'fill-opacity': 0.6
            }
        },
        
        // Movement area outlines
        {
            'id': 'movement-outlines',
            'type': 'line',
            'source': 'maritime-tiles',
            'source-layer': 'merged_polygons',
            'filter': ['all', ['==', 'polygon_type', 'movement'], ['!=', 'maneuver_area', true]],
            'paint': {
                'line-color': '#ea580c',
                'line-width': ['interpolate', ['linear'], ['zoom'], 0, 0.5, 8, 1, 12, 1.5],
                'line-opacity': 0.7
            }
        },
        
        // Stopping area outlines
        {
            'id': 'stopping-outlines',
            'type': 'line',
            'source': 'maritime-tiles',
            'source-layer': 'merged_polygons',
            'filter': ['all', ['==', 'polygon_type', 'stopping'], ['!=', 'maneuver_area', true]],
            'paint': {
                'line-color': '#1d4ed8',
                'line-width': ['interpolate', ['linear'], ['zoom'], 0, 0.5, 8, 1, 12, 1.5],
                'line-opacity': 0.7
            }
        },
        
        // Maneuver area outlines (special highlighting)
        {
            'id': 'maneuver-outlines',
            'type': 'line',
            'source': 'maritime-tiles',
            'source-layer': 'merged_polygons',
            'filter': ['==', 'maneuver_area', true],
            'paint': {
                'line-color': '#dc2626',
                'line-width': ['interpolate', ['linear'], ['zoom'], 0, 1, 8, 2, 12, 3],
                'line-opacity': 0.8
            }
        }
    ];
}

// Make map available globally
window.map = map;

// Setup event handlers after map loads
map.on('load', function() {
    console.log('Map loaded successfully');
    setupLayerControls();
    setupClickHandlers();
    setupHoverEffects();
});

// Layer control functions
function setupLayerControls() {
    const controls = {
        'toggle-openstreetmap': ['openstreetmap'],
        'toggle-openseamap': ['openseamap'],
        'toggle-movement': ['movement-areas', 'movement-outlines'],
        'toggle-stopping': ['stopping-areas', 'stopping-outlines'],
        'toggle-maneuver': ['maneuver-outlines']
    };
    
    Object.entries(controls).forEach(([toggleId, layerIds]) => {
        const toggleElement = document.getElementById(toggleId);
        if (toggleElement) {
            toggleElement.addEventListener('change', function(e) {
                const visibility = e.target.checked ? 'visible' : 'none';
                layerIds.forEach(layerId => {
                    if (map.getLayer(layerId)) {
                        map.setLayoutProperty(layerId, 'visibility', visibility);
                    }
                });
            });
        }
    });
}

// Click handlers for polygon information
function setupClickHandlers() {
    map.on('click', function(e) {
        const features = map.queryRenderedFeatures(e.point);
        
        console.log('=== MAP CLICK DEBUG ===');
        console.log('Click coordinates:', e.lngLat);
        console.log('All features found:', features.length);
        
        if (features.length === 0) return;
        
        const feature = features[0];
        const props = feature.properties;
        console.log('Feature selected:', feature.layer.id, props);
        
        let content = `<strong>Layer:</strong> ${feature.layer.id}<br><strong>Source:</strong> ${feature.sourceLayer}<br><br>`;
        
        if (props.polygon_type) {
            content += `<strong>Type:</strong> ${props.polygon_type}<br>`;
            content += `<strong>Points:</strong> ${props.num_points || 'N/A'}<br>`;
            content += `<strong>MMSIs:</strong> ${props.num_uniq_mmsis || 'N/A'}<br>`;
            content += `<strong>Maneuver Area:</strong> ${props.maneuver_area ? 'Yes' : 'No'}<br>`;
            content += `<strong>ID:</strong> ${props.id || 'N/A'}<br>`;
        }
        
        new maplibregl.Popup()
            .setLngLat(e.lngLat)
            .setHTML(content)
            .addTo(map);
    });
}

// Hover effects for interactive layers
function setupHoverEffects() {
    const interactiveLayers = [
        'movement-areas', 'stopping-areas'
    ];
    
    interactiveLayers.forEach(layerId => {
        map.on('mouseenter', layerId, () => {
            map.getCanvas().style.cursor = 'pointer';
        });
        
        map.on('mouseleave', layerId, () => {
            map.getCanvas().style.cursor = '';
        });
    });
}