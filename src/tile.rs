use std::collections::HashMap;

use geojson::feature::Id;
use geojson::{FeatureCollection, JsonValue};
use serde_json::Number;

use crate::types::{
    VtEmpty, VtFeature, VtFeatures, VtGeometry, VtGeometryCollection, VtLineString,
    VtMultiLineString, VtMultiPoint, VtMultiPolygon, VtPoint, VtPolygon,
};
use crate::BBox;

pub struct Tile {
    features: geojson::FeatureCollection,
    num_points: u32,
    num_simplified: u32,
}

pub struct InternalTile {
    extent: u16,
    z: u8,
    x: u32,
    y: u32,
    z2: f64,
    tolerance: f64,
    sq_tolerance: f64,
    line_metrics: bool,
    source_features: VtFeatures,
    bbox: BBox,
    pub tile: Tile,
}

impl InternalTile {
    pub fn new(
        source: &VtFeatures,
        z: u8,
        x: u32,
        y: u32,
        extent: u16,
        tolerance: f64,
        line_metrics: bool,
    ) -> InternalTile {
        let mut _tile = Self {
            extent,
            z,
            x,
            y,
            z2: 2i32.pow(z as u32) as f64,
            tolerance,
            sq_tolerance: tolerance * tolerance,
            line_metrics,
            source_features: vec![],
            bbox: Default::default(),
            tile: Tile {
                features: FeatureCollection {
                    bbox: None,
                    features: Vec::with_capacity(source.len()),
                    foreign_members: None,
                },
                num_points: 0,
                num_simplified: 0,
            },
        };

        for feature in source {
            let geom = &feature.geometry;
            // TODO assert!(feature.properties);
            let props = &feature.properties;
            let id = &feature.id;

            _tile.tile.num_points += feature.num_points;

            // TODO Verify if this is correct
            _tile.add_geometry_feature(geom, props, id);

            _tile.bbox.min.x = feature.bbox.min.x.min(_tile.bbox.min.x);
            _tile.bbox.min.y = feature.bbox.min.y.min(_tile.bbox.min.y);
            _tile.bbox.max.x = feature.bbox.max.x.max(_tile.bbox.max.x);
            _tile.bbox.max.y = feature.bbox.max.y.max(_tile.bbox.max.y);
        }

        _tile
    }
}

impl InternalTile {
    fn add_geometry_feature(
        &self,
        geom: &VtGeometry,
        props: &HashMap<String, JsonValue>,
        id: &Option<geojson::feature::Id>,
    ) {
        match geom {
            VtGeometry::Empty(empty) => self.add_empty_feature(empty, props, id),
            VtGeometry::Point(point) => self.add_point_feature(point, props, id),
            VtGeometry::MultiPoint(multi_point) => {
                self.ass_multi_point_feature(multi_point, props, id)
            }
            VtGeometry::LineString(line_string) => {
                self.add_line_string_feature(line_string, props, id)
            }
            VtGeometry::MultiLineString(multi_line_string) => {
                self.add_multi_line_string_feature(multi_line_string, props, id)
            }
            VtGeometry::Polygon(polygon) => self.add_polygon_feature(polygon, props, id),
            VtGeometry::MultiPolygon(multi_polygon) => {
                self.add_multi_polygon_feature(multi_polygon, props, id)
            }
            VtGeometry::GeometryCollection(geometry_collection) => {
                self.add_geometry_collection_feature(geometry_collection, props, id)
            }
        }
    }

    fn add_empty_feature(
        &self,
        value: &VtEmpty,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        self.tile.features.push(transform_empty(value), props, id);
    }

    fn add_point_feature(
        &self,
        value: &VtPoint,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        self.tile.features.push(transform_point(value), props, id);
    }

    fn ass_multi_point_feature(
        &self,
        value: &VtMultiPoint,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        let new_multi = transform_multi_point_feature(value);

        match new_multi.len() {
            0 => {}
            1 => self
                .tile
                .features
                .push(VtFeature::new(new_multi[0], props, id)),
            _ => self
                .tile
                .features
                .push(VtFeature::new(new_multi, props, id)),
        }
    }
    fn add_line_string_feature(
        &self,
        line: &VtLineString,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        let new_line = transform_line_string(line);
        if (!new_line.empty()) {
            if (self.line_metrics) {
                let mut newProps = props;
                newProps.insert(
                    "mapbox_clip_start".to_string(),
                    JsonValue::Number(Number::from_f64(line.seg_start / line.dist).unwrap()),
                );
                newProps.insert(
                    "mapbox_clip_end".to_string(),
                    JsonValue::Number(Number::from_f64(line.seg_end / line.dist).unwrap()),
                );
                self.tile
                    .features
                    .push(VtFeature::new(new_line, newProps, id));
            } else {
                self.tile.features.push(VtFeature::new(new_line, props, id));
            }
        }
    }
    fn add_multi_line_string_feature(
        &self,
        value: &VtMultiLineString,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        let new_multi = transform_multi_line_string(value);

        match new_multi.len() {
            0 => {}
            1 => self
                .tile
                .features
                .push(VtFeature::new(new_multi[0], props, id)),
            _ => self
                .tile
                .features
                .push(VtFeature::new(new_multi, props, id)),
        }
    }
    fn add_polygon_feature(
        &self,
        value: &VtPolygon,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        let new_polygon = transform_polygon(value);
        if (!new_polygon.empty()) {
            self.tile
                .features
                .emplace_back(VtFeature::new(new_polygon, props, id));
        }
    }
    fn add_multi_polygon_feature(
        &self,
        value: &VtMultiPolygon,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        let new_multi = transform_multi_polygon_feature(value);

        match new_multi.len() {
            0 => {}
            1 => self
                .tile
                .features
                .push(VtFeature::new(new_multi[0], props, id)),
            _ => self
                .tile
                .features
                .push(VtFeature::new(new_multi, props, id)),
        }
    }
    fn add_geometry_collection_feature(
        &self,
        value: &VtGeometryCollection,
        props: &HashMap<String, JsonValue>,
        id: &Option<Id>,
    ) {
        for geom in value {
            // TODO verify this is correct
            self.add_geometry_feature(geom, props, id)
        }
    }
}

fn transform_multi_polygon_feature(p0: &VtMultiPolygon) -> _ {
    todo!()
}

fn transform_multi_point_feature(p0: &VtMultiPoint) -> _ {
    todo!()
}

fn transform_line_string(p0: &VtLineString) -> _ {
    todo!()
}

fn transform_multi_line_string(p0: &VtMultiLineString) -> _ {
    todo!()
}

fn transform_polygon(p0: &VtPolygon) -> _ {
    todo!()
}

fn transform_point(p0: &VtPoint) -> _ {
    todo!()
}

fn transform_empty(empty: VtEmpty) -> _ {
    todo!()
}
