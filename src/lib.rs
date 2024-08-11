use std::collections::HashMap;
use geojson::{FeatureCollection, GeoJson, PointType};

mod clip;
mod convert;
mod simplify;
mod tile;
mod types;
mod wrap;


pub struct ToFeatureCollection;

impl ToFeatureCollection {
    pub fn visit(&self, value: &GeoJson) -> FeatureCollection {
        // C++ equivalent:
        // feature_collection operator()(const feature_collection& value) const {
        //     return value;
        // }
        // feature_collection operator()(const feature& value) const {
        //     return { value };
        // }
        // feature_collection operator()(const geometry& value) const {
        //     return { { value } };
        // }
        unimplemented!()
    }
}

#[derive(Clone, Default)]
pub struct TileOptions {
    pub tolerance: f64,     // simplification tolerance (higher means simpler)
    pub extent: u16,        // tile extent
    pub buffer: u16,        // tile buffer on each side
    pub line_metrics: bool, // enable line metrics tracking for LineString/MultiLineString features
}

#[derive(Clone, Default)]
pub struct Options {
    pub max_zoom: u8,          // max zoom to preserve detail on
    pub index_max_zoom: u8,    // max zoom in the tile index
    pub index_max_points: u32, // max number of points per tile in the tile index
    pub generate_id: bool,     // whether to generate feature ids, overriding existing ids
}

pub struct Tile;

pub const EMPTY_TILE: Tile = Tile;

pub fn to_id(z: u8, x: u32, y: u32) -> u64 {
    // C++ equivalent:
    // return (((1ull << z) * y + x) * 32) + z;
    unimplemented!()
}

pub fn geojson_to_tile(
    geojson: &GeoJson,
    z: u8,
    x: u32,
    y: u32,
    options: &TileOptions,
    wrap: bool,
    clip: bool,
) -> Tile {
    // C++ equivalent:
    // const auto features_ = geojson::visit(geojson_, ToFeatureCollection{});
    // auto z2 = 1u << z;
    // auto tolerance = (options.tolerance / options.extent) / z2;
    // auto features = detail::convert(features_, tolerance, false);
    // if (wrap) {
    //     features = detail::wrap(features, double(options.buffer) / options.extent, options.lineMetrics);
    // }
    // if (clip || options.lineMetrics) {
    //     const double p = double(options.buffer) / options.extent;
    //     const auto left = detail::clip<0>(features, (x - p) / z2, (x + 1 + p) / z2, -1, 2, options.lineMetrics);
    //     features = detail::clip<1>(left, (y - p) / z2, (y + 1 + p) / z2, -1, 2, options.lineMetrics);
    // }
    // return detail::InternalTile({ features, z, x, y, options.extent, tolerance, options.lineMetrics }).tile;
    unimplemented!()
}

pub struct GeoJSONVT {
    pub options: Options,
    pub stats: HashMap<u8, u32>,
    pub total: u32,
    tiles: HashMap<u64, InternalTile>,
}

impl GeoJSONVT {
    pub fn new(features: &FeatureCollection, options: &Options) -> Self {
        // C++ equivalent:
        // const uint32_t z2 = 1u << options.maxZoom;
        // auto converted = detail::convert(features_, (options.tolerance / options.extent) / z2, options.generateId);
        // auto features = detail::wrap(converted, double(options.buffer) / options.extent, options.lineMetrics);
        // splitTile(features, 0, 0, 0);
        unimplemented!()
    }

    pub fn get_tile(&self, z: u8, x_: u32, y: u32) -> &Tile {
        // C++ equivalent:
        // if (z > options.maxZoom)
        //     throw std::runtime_error("Requested zoom higher than maxZoom: " + std::to_string(z));
        // const uint32_t z2 = 1u << z;
        // const uint32_t x = ((x_ % z2) + z2) % z2; // wrap tile x coordinate
        // const uint64_t id = toID(z, x, y);
        // auto it = tiles.find(id);
        // if (it != tiles.end())
        //     return it->second.tile;
        // it = findParent(z, x, y);
        // if (it == tiles.end())
        //     throw std::runtime_error("Parent tile not found");
        // const auto& parent = it->second;
        // splitTile(parent.source_features, parent.z, parent.x, parent.y, z, x, y);
        // it = tiles.find(id);
        // if (it != tiles.end())
        //     return it->second.tile;
        // it = findParent(z, x, y);
        // if (it == tiles.end())
        //     throw std::runtime_error("Parent tile not found");
        // return empty_tile;
        unimplemented!()
    }

    pub fn get_internal_tiles(&self) -> &HashMap<u64, InternalTile> {
        // C++ equivalent:
        // return tiles;
        unimplemented!()
    }

    fn find_parent(&self, z: u8, x: u32, y: u32) -> Option<&InternalTile> {
        // C++ equivalent:
        // uint8_t z0 = z;
        // uint32_t x0 = x;
        // uint32_t y0 = y;
        // const auto end = tiles.end();
        // auto parent = end;
        // while ((parent == end) && (z0 != 0)) {
        //     z0--;
        //     x0 = x0 / 2;
        //     y0 = y0 / 2;
        //     parent = tiles.find(toID(z0, x0, y0));
        // }
        // return parent;
        unimplemented!()
    }

    fn split_tile(
        &mut self,
        features: FeatureCollection,
        z: u8,
        x: u32,
        y: u32,
        cz: u8,
        cx: u32,
        cy: u32,
    ) {
        // C++ equivalent:
        // const double z2 = 1u << z;
        // const uint64_t id = toID(z, x, y);
        // auto it = tiles.find(id);
        // if (it == tiles.end()) {
        //     const double tolerance =
        //         (z == options.maxZoom ? 0 : options.tolerance / (z2 * options.extent));
        //     it = tiles
        //              .emplace(id, detail::InternalTile{ features, z, x, y, options.extent, tolerance, options.lineMetrics })
        //              .first;
        //     stats[z] = (stats.count(z) ? stats[z] + 1 : 1);
        //     total++;
        // }
        // auto& tile = it->second;
        // if (features.empty())
        //     return;
        // if (cz == 0u) {
        //     if (z == options.indexMaxZoom || tile.tile.num_points <= options.indexMaxPoints) {
        //         tile.source_features = features;
        //         return;
        //     }
        // } else {
        //     if (z == options.maxZoom)
        //         return;
        //     if (z == cz) {
        //         tile.source_features = features;
        //         return;
        //     }
        //     const double m = 1u << (cz - z);
        //     if (x != static_cast<uint32_t>(std::floor(cx / m)) ||
        //         y != static_cast<uint32_t>(std::floor(cy / m))) {
        //         tile.source_features = features;
        //         return;
        //     }
        // }
        // const double p = 0.5 * options.buffer / options.extent;
        // const auto& min = tile.bbox.min;
        // const auto& max = tile.bbox.max;
        // const auto left = detail::clip<0>(features, (x - p) / z2, (x + 0.5 + p) / z2, min.x, max.x, options.lineMetrics);
        // splitTile(detail::clip<1>(left, (y - p) / z2, (y + 0.5 + p) / z2, min.y, max.y, options.lineMetrics), z + 1,
        //           x * 2, y * 2, cz, cx, cy);
        // splitTile(detail::clip<1>(left, (y + 0.5 - p) / z2, (y + 1 + p) / z2, min.y, max.y, options.lineMetrics), z + 1,
        //           x * 2, y * 2 + 1, cz, cx, cy);
        // const auto right =
        //     detail::clip<0>(features, (x + 0.5 - p) / z2, (x + 1 + p) / z2, min.x, max.x, options.lineMetrics);
        // splitTile(detail::clip<1>(right, (y - p) / z2, (y + 0.5 + p) / z2, min.y, max.y, options.lineMetrics), z + 1,
        //           x * 2 + 1, y * 2, cz, cx, cy);
        // splitTile(detail::clip<1>(right, (y + 0.5 - p) / z2, (y + 1 + p) / z2, min.y, max.y, options.lineMetrics), z + 1,
        //           x * 2 + 1, y * 2 + 1, cz, cx, cy);
        // tile.source_features = {};
        unimplemented!()
    }
}

pub struct InternalTile {
    pub tile: Tile,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub extent: u16,
    pub tolerance: f64,
    pub line_metrics: bool,
    pub source_features: FeatureCollection,
    pub bbox: BBox,
}

#[derive(Clone, Default)]
pub struct BBox {
    pub min: PointType,
    pub max: PointType,
}


pub struct Rect<T>(T);

impl<T> Rect<T> {
    pub fn new(x: Coordinate, y: Coordinate) -> Self {
        todo!()
    }
}

pub struct Features;
pub struct Coordinate {
    x: f64,
    y: f64
}