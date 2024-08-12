use crate::clip::clip;
use crate::convert::convert;
use crate::tile::{InternalTile, Tile, EMPTY_TILE};
use crate::types::VtFeatures;
use crate::wrap::wrap;
use euclid::{Point2D, UnknownUnit};
use geojson::{
    Feature, FeatureCollection, GeoJson, Geometry, JsonObject, LineStringType, PointType,
    PolygonType,
};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

mod clip;
mod convert;
mod simplify;
mod tile;
mod types;
mod wrap;

#[cfg(test)]
mod tests;

pub struct ToFeatureCollection;

#[derive(Clone)]
pub struct TileOptions {
    pub tolerance: f64,     // simplification tolerance (higher means simpler)
    pub extent: u16,        // tile extent
    pub buffer: u16,        // tile buffer on each side
    pub line_metrics: bool, // enable line metrics tracking for LineString/MultiLineString features
}

impl Default for TileOptions {
    fn default() -> Self {
        Self {
            tolerance: 3.,
            extent: 4096,
            buffer: 64,
            line_metrics: false,
        }
    }
}

#[derive(Clone)]
pub struct Options {
    pub max_zoom: u8,          // max zoom to preserve detail on
    pub index_max_zoom: u8,    // max zoom in the tile index
    pub index_max_points: u32, // max number of points per tile in the tile index
    pub generate_id: bool,     // whether to generate feature ids, overriding existing ids
    pub tile: TileOptions,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_zoom: 18,
            index_max_zoom: 5,
            index_max_points: 100000,
            generate_id: false,
            tile: TileOptions::default(),
        }
    }
}

pub fn to_id(z: u8, x: u32, y: u32) -> u64 {
    return (((1u64 << z as u64) * y as u64 + x as u64) * 32) + z as u64;
}

fn geojson_to_feature_collection(geojson: &GeoJson) -> FeatureCollection {
    // TODO cleanup this conversion
    match geojson {
        GeoJson::Geometry(geom) => FeatureCollection {
            bbox: None,
            features: vec![Feature {
                bbox: None,
                geometry: Some(geom.clone()),
                id: None,
                properties: None,
                foreign_members: None,
            }],
            foreign_members: None,
        },
        GeoJson::Feature(feature) => FeatureCollection {
            bbox: None,
            features: vec![feature.clone()],
            foreign_members: None,
        },
        GeoJson::FeatureCollection(features) => features.clone(),
    }
}

pub fn geojson_to_tile(
    geojson: &GeoJson,
    z: u8,
    x: u32,
    y: u32,
    options: &TileOptions,
    wrap_: bool,
    clip_: bool,
) -> Tile {
    let features_ = &geojson_to_feature_collection(geojson);
    let z2 = 1u32 << z;
    let tolerance = (options.tolerance / options.extent as f64) / z2 as f64;
    let mut features = convert(features_, tolerance, false);
    if wrap_ {
        features = wrap(
            &features,
            options.buffer as f64 / options.extent as f64,
            options.line_metrics,
        );
    }
    if clip_ || options.line_metrics {
        let p = options.buffer as f64 / options.extent as f64;

        let left = clip::<0>(
            &features,
            (x as f64 - p) / z2 as f64,
            (x as f64 + 1. + p) / z2 as f64,
            -1.,
            2.,
            options.line_metrics,
        );
        features = clip::<1>(
            &left,
            (y as f64 - p) / z2 as f64,
            (y as f64 + 1. + p) / z2 as f64,
            -1.,
            2.,
            options.line_metrics,
        );
    }
    return InternalTile::new(
        &features,
        z,
        x,
        y,
        options.extent,
        tolerance,
        options.line_metrics,
    )
    .tile;
}

pub struct GeoJSONVT {
    pub options: Options,
    pub stats: HashMap<u8, u32>,
    pub total: u32,
    tiles: HashMap<u64, InternalTile>,
}

impl GeoJSONVT {
    pub fn from_geojson(geojson: &GeoJson, options: &Options) -> Self {
        let collection = geojson_to_feature_collection(geojson);
        Self::new(&collection, options)
    }
    pub fn new(features_: &FeatureCollection, options: &Options) -> Self {
        let mut vt = Self {
            options: options.clone(),
            stats: Default::default(),
            total: 0,
            tiles: Default::default(),
        };
        let z2 = 1u32 << options.max_zoom;

        let converted = convert(
            features_,
            (options.tile.tolerance as f64 / options.tile.extent as f64) / z2 as f64,
            options.generate_id,
        );
        let features = wrap(
            &converted,
            options.tile.buffer as f64 / options.tile.extent as f64,
            options.tile.line_metrics,
        );

        vt.split_tile_zeros(&features, 0, 0, 0);
        vt
    }

    pub fn get_tile(&mut self, z: u8, x_: u32, y: u32) -> &Tile {
        if z > self.options.max_zoom {
            panic!("Requested zoom higher than maxZoom: {}", z);
        }

        let z2 = 1u32 << z;
        let x = ((x_ % z2) + z2) % z2; // wrap tile x coordinate
        let id = to_id(z, x, y);

        if self.tiles.contains_key(&id) {
            return &self.tiles[&id].tile;
        }

        if let Some(parent) = self.find_parent(z, x, y) {
            // if we found a parent tile containing the original geometry, we can drill down from it

            // drill down parent tile up to the requested one
            self.split_tile(
                &parent.source_features.clone(),
                parent.z,
                parent.x,
                parent.y,
                z,
                x,
                y,
            );

            if self.tiles.contains_key(&id) {
                return &self.tiles[&id].tile;
            }

            if let None = self.find_parent(z, x, y) {
                panic!("Parent tile not found");
            }
        } else {
            panic!("Parent tile not found");
        }

        return &EMPTY_TILE;
    }

    pub fn get_internal_tiles(&self) -> &HashMap<u64, InternalTile> {
        return &self.tiles;
    }

    fn find_parent(&self, z: u8, x: u32, y: u32) -> Option<&InternalTile> {
        let mut z0 = z;
        let mut x0 = x;
        let mut y0 = y;

        let end = None;
        let mut parent = end;

        while (parent == end) && (z0 != 0) {
            z0 -= 1;
            x0 = x0 / 2;
            y0 = y0 / 2;
            parent = self.tiles.get(&to_id(z0, x0, y0));
        }

        return parent;
    }

    fn split_tile_zeros(&mut self, features: &VtFeatures, z: u8, x: u32, y: u32) {
        self.split_tile(features, z, x, y, 0, 0, 0)
    }
    fn split_tile(
        &mut self,
        features: &VtFeatures,
        z: u8,
        x: u32,
        y: u32,
        cz: u8,
        cx: u32,
        cy: u32,
    ) {
        let z2: f64 = (1u32 << z) as f64;
        let id = to_id(z, x, y);

        // try insert
        match self.tiles.entry(id) {
            Entry::Occupied(_) => {}
            Entry::Vacant(entry) => {
                let tolerance = if z == self.options.max_zoom {
                    0.
                } else {
                    self.options.tile.tolerance / (z2 * self.options.tile.extent as f64)
                };

                entry.insert(InternalTile::new(
                    features,
                    z,
                    x,
                    y,
                    self.options.tile.extent,
                    tolerance,
                    self.options.tile.line_metrics,
                ));

                self.stats.insert(
                    z,
                    if self.stats.contains_key(&z) {
                        self.stats[&z] + 1
                    } else {
                        1
                    },
                );
                self.total += 1;
                println!("tile z{z}-{x}-{y}");
            }
        };

        let tile = self.tiles.get_mut(&id).expect("can no longer be None");

        if features.is_empty() {
            eprintln!("no feature");
            return;
        } else {
            //eprintln!("{:?}", features)
        }

        // if it's the first-pass tiling
        if cz == 0u8 {
            // stop tiling if we reached max zoom, or if the tile is too simple
            if z == self.options.index_max_zoom
                || tile.tile.num_points <= self.options.index_max_points
            {
                println!("reached max zoom");
                tile.source_features = features.clone();
                return;
            }
        } else {
            // drilldown to a specific tile;
            // stop tiling if we reached base zoom
            if z == self.options.max_zoom {
                println!("reached base zoom");
                return;
            }

            // stop tiling if it's our target tile zoom
            if z == cz {
                tile.source_features = features.clone();
                println!("target tile zoom");
                return;
            }

            // stop tiling if it's not an ancestor of the target tile
            let m: f64 = (1u32 << (cz - z)) as f64;
            let a = (cx as f64 / m).floor() as u32;
            let b = (cy as f64 / m).floor() as u32;
            if x != a || y != b {
                println!("not an ancestor");
                tile.source_features = features.clone();
                return;
            }
        }

        let p: f64 = 0.5 * self.options.tile.buffer as f64 / self.options.tile.extent as f64; // TODO: Is the calculation order right?
        let min = tile.bbox.min;
        let max = tile.bbox.max;

        let left = clip::<0>(
            features,
            (x as f64 - p) / z2,
            (x as f64 + 0.5 + p) / z2,
            min.x,
            max.x,
            self.options.tile.line_metrics,
        );

        self.split_tile(
            &clip::<1>(
                &left,
                (y as f64 - p) / z2,
                (y as f64 + 0.5 + p) / z2,
                min.y,
                max.y,
                self.options.tile.line_metrics,
            ),
            z + 1,
            x * 2,
            y * 2,
            cz,
            cx,
            cy,
        );
        self.split_tile(
            &clip::<1>(
                &left,
                (y as f64 + 0.5 - p) / z2,
                (y as f64 + 1. + p) / z2,
                min.y,
                max.y,
                self.options.tile.line_metrics,
            ),
            z + 1,
            x * 2,
            y * 2 + 1,
            cz,
            cx,
            cy,
        );

        let right = clip::<0>(
            &features,
            (x as f64 + 0.5 - p) / z2,
            (x as f64 + 1. + p) / z2,
            min.x,
            max.x,
            self.options.tile.line_metrics,
        );

        self.split_tile(
            &clip::<1>(
                &right,
                (y as f64 - p) / z2,
                (y as f64 + 0.5 + p) / z2,
                min.y,
                max.y,
                self.options.tile.line_metrics,
            ),
            z + 1,
            x * 2 + 1,
            y * 2,
            cz,
            cx,
            cy,
        );
        self.split_tile(
            &clip::<1>(
                &right,
                (y as f64 + 0.5 - p) / z2,
                (y as f64 + 1. + p) / z2,
                min.y,
                max.y,
                self.options.tile.line_metrics,
            ),
            z + 1,
            x * 2 + 1,
            y * 2 + 1,
            cz,
            cx,
            cy,
        );

        // if we sliced further down, no need to keep source geometry
        // TODO Cleanup, dont fetch twice
        let tile = self.tiles.get_mut(&id).expect("can no longer be None");
        tile.source_features = Vec::new();
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BBox {
    pub min: Point2D<f64, UnknownUnit>,
    pub max: Point2D<f64, UnknownUnit>,
}

impl BBox {
    pub fn new(min: Point2D<f64, UnknownUnit>, max: Point2D<f64, UnknownUnit>) -> Self {
        Self { min, max }
    }
}

pub type MultiPointType = Vec<PointType>;
pub type MultiLineStringType = Vec<LineStringType>;
pub type MultiPolygonType = Vec<PolygonType>;
pub type GeometryCollectionType = Vec<Geometry>;
pub type LinearRingType = Vec<PointType>;
