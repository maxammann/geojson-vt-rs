## geojson-vt-rs &mdash; GeoJSON Vector Tiles 

Port to Rust of [C++ GeoJSON-VT](https://github.com/mapbox/geojson-vt-cpp) for slicing GeoJSON into vector tiles on the fly.

A highly efficient Rust library for **slicing GeoJSON data into vector tiles on the fly**,  primarily designed to enable rendering and interacting with large geospatial datasets  on the browser side (without a server).

Created to power GeoJSON in [maplibre-rs](https://github.com/maplibre/maplibre-rs), but can be useful in other visualization platforms  like [Leaflet](https://github.com/Leaflet/Leaflet), [OpenLayers](https://openlayers.org/) and [d3](https://github.com/mbostock/d3),  as well as Node.js server applications.

Resulting tiles conform to the JSON equivalent  of the [vector tile specification](https://github.com/mapbox/vector-tile-spec/).
To make data rendering and interaction fast, the tiles are simplified,  retaining the minimum level of detail appropriate for each zoom level (simplifying shapes, filtering out tiny polygons and polylines).

Read more on how the library works [on the Mapbox blog](https://blog.mapbox.com/rendering-big-geodata-on-the-fly-with-geojson-vt-4e4d2a5dd1f2).

There is also a C++11 port: [geojson-vt-cpp](https://github.com/mapbox/geojson-vt-cpp)

### Options

You can fine-tune the results with an options struct, although the defaults are sensible and work well for most use cases.

```rust
Options {
    max_zoom: 18,          // max zoom to preserve detail on; can't be higher than 24
    index_max_zoom: 5,    // max zoom in the tile index
    index_max_points: 100000, // max number of points per tile in the tile index
    generate_id: false,     // whether to generate feature ids, overriding existing ids
    tile: TileOptions {
        tolerance: 3.,     // simplification tolerance (higher means simpler)
        extent: 4096,        // tile extent
        buffer: 64,        // tile buffer on each side
        line_metrics: false, // enable line metrics tracking for LineString/MultiLineString features
    }
}
```

By default, tiles at zoom levels above `index_max_zoom` are generated on the fly, 
but you can pre-generate all possible tiles for `data` by setting `index_max_zoom` and `max_zoom` to the same value and
setting `indexMaxPoints` to `0`.

The `generate_id` option ignores existing `id` values on the feature objects.

**The library only operates on zoom levels up to 24.**


