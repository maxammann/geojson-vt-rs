use std::collections::HashMap;

use geojson::feature::Id;
use geojson::{
    Feature, FeatureCollection, Geometry, JsonObject, JsonValue, LineStringType, PointType,
    PolygonType, Value,
};
use serde_json::Number;

use crate::types::{
    VtEmpty, VtFeature, VtFeatures, VtGeometry, VtGeometryCollection, VtLineString, VtLinearRing,
    VtMultiLineString, VtMultiPoint, VtMultiPolygon, VtPoint, VtPolygon,
};
use crate::{BBox, LinearRingType, MultiLineStringType, MultiPointType, MultiPolygonType};

pub static EMPTY_TILE: Tile = Tile {
    features: FeatureCollection {
        bbox: None,
        features: vec![],
        foreign_members: None,
    },
    num_points: 0,
    num_simplified: 0,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Tile {
    pub features: FeatureCollection,
    pub num_points: u32,
    pub num_simplified: u32,
}

#[derive(PartialEq, Clone)]
pub struct InternalTile {
    extent: u16,
    pub z: u8,
    pub x: u32,
    pub y: u32,
    z2: f64,
    tolerance: f64,
    sq_tolerance: f64,
    line_metrics: bool,
    pub source_features: VtFeatures,
    pub bbox: BBox,
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
            _tile.add_geometry_feature(geom, if props.is_empty() {
                None
            } else {
                Some(props.clone())
            }, id);

            _tile.bbox.min.x = feature.bbox.min.x.min(_tile.bbox.min.x);
            _tile.bbox.min.y = feature.bbox.min.y.min(_tile.bbox.min.y);
            _tile.bbox.max.x = feature.bbox.max.x.max(_tile.bbox.max.x);
            _tile.bbox.max.y = feature.bbox.max.y.max(_tile.bbox.max.y);
        }

        _tile
    }
}

impl InternalTile {
    fn add_geometry_feature(&mut self, geom: &VtGeometry, props: Option<JsonObject>, id: &Option<Id>) {
        match geom {
            VtGeometry::Empty(empty) => self.add_empty_feature(empty, props, id),
            VtGeometry::Point(point) => self.add_point_feature(point, props, id),
            VtGeometry::MultiPoint(multi_point) => {
                self.add_multi_point_feature(multi_point, props, id)
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

    fn add_empty_feature(&mut self, value: &VtEmpty, props: Option<JsonObject>, id: &Option<Id>) {
        // TODO self.tile.features.features.push(self.transform_empty(value), props, id);
    }

    fn add_point_feature(&mut self, value: &VtPoint, props: Option<JsonObject>, id: &Option<Id>) {
        let geometry = Some(Geometry::new(Value::Point(self.transform_point(value))));
        self.tile.features.features.push(Feature {
            bbox: None,
            geometry: geometry,
            id: id.clone(),
            properties: props,
            foreign_members: None,
        });
    }

    fn add_multi_point_feature(
        &mut self,
        value: &VtMultiPoint,
        props: Option<JsonObject>,
        id: &Option<Id>,
    ) {
        let new_multi = self.transform_multi_point_feature(value);

        match new_multi.len() {
            0 => {}
            1 => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::Point(new_multi[0].clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
            _ => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::MultiPoint(new_multi.clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
        }
    }
    fn add_line_string_feature(
        &mut self,
        line: &VtLineString,
        props: Option<JsonObject>,
        id: &Option<Id>,
    ) {
        let new_line = self.transform_line_string(line);
        if !new_line.is_empty() {
            if self.line_metrics {
                let mut newProps = props.unwrap_or_default();
                newProps.insert(
                    "mapbox_clip_start".to_string(),
                    JsonValue::Number(Number::from_f64(line.seg_start / line.dist).unwrap()),
                );
                newProps.insert(
                    "mapbox_clip_end".to_string(),
                    JsonValue::Number(Number::from_f64(line.seg_end / line.dist).unwrap()),
                );
                self.tile.features.features.push(Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(new_line.clone()))),
                    id: id.clone(),
                    properties: Some(newProps),
                    foreign_members: None,
                });
            } else {
                self.tile.features.features.push(Feature {
                    bbox: None,
                    geometry: Some(Geometry::new(Value::LineString(new_line.clone()))),
                    id: id.clone(),
                    properties: props,
                    foreign_members: None,
                });
            }
        }
    }
    fn add_multi_line_string_feature(
        &mut self,
        value: &VtMultiLineString,
        props: Option<JsonObject>,
        id: &Option<Id>,
    ) {
        let new_multi = self.transform_multi_line_string(value);

        match new_multi.len() {
            0 => {}
            1 => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::LineString(new_multi[0].clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
            _ => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::MultiLineString(new_multi.clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
        }
    }
    fn add_polygon_feature(&mut self, value: &VtPolygon, props: Option<JsonObject>, id: &Option<Id>) {
        let new_polygon = self.transform_polygon(value);
        if !new_polygon.is_empty() {
            self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::Polygon(new_polygon.clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            });
        }
    }
    fn add_multi_polygon_feature(
        &mut self,
        value: &VtMultiPolygon,
        props: Option<JsonObject>,
        id: &Option<Id>,
    ) {
        let new_multi = self.transform_multi_polygon_feature(value);

        match new_multi.len() {
            0 => {}
            1 => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::Polygon(new_multi[0].clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
            _ => self.tile.features.features.push(Feature {
                bbox: None,
                geometry: Some(Geometry::new(Value::MultiPolygon(new_multi.clone()))),
                id: id.clone(),
                properties: props,
                foreign_members: None,
            }),
        }
    }
    fn add_geometry_collection_feature(
        &mut self,
        value: &VtGeometryCollection,
        props: Option<JsonObject>,
        id: &Option<Id>,
    ) {
        for geom in value {
            // TODO verify this is correct
            // FIXME should this become two features? what about props?
            self.add_geometry_feature(geom, props.clone(), id); // TODO clone is probably not correct here.
        }
    }

    fn transform_multi_polygon_feature(&mut self, polygons: &VtMultiPolygon) -> MultiPolygonType {
        let mut result: MultiPolygonType = Vec::new();
        result.reserve(polygons.len());
        for polygon in polygons {
            let p = self.transform_polygon(polygon);
            if !p.is_empty() {
                result.push(p);
            }
        }
        return result;
    }

    fn transform_multi_point_feature(&mut self, points: &VtMultiPoint) -> MultiPointType {
        let mut result: MultiPointType = Vec::new();
        result.reserve(points.len());
        for p in points {
            result.push(self.transform_point(p));
        }
        return result;
    }

    fn transform_line_string(&mut self, line: &VtLineString) -> LineStringType {
        let mut result: LineStringType = Vec::new();
        if line.dist > self.tolerance {
            result.reserve(line.elements.len());
            for p in &line.elements {
                if p.z > self.sq_tolerance {
                    result.push(self.transform_point(p));
                }
            }
        }
        return result;
    }

    fn transform_multi_line_string(&mut self, lines: &VtMultiLineString) -> MultiLineStringType {
        let mut result: MultiLineStringType = Vec::new();
        result.reserve(lines.len());
        for line in lines {
            if line.dist > self.tolerance {
                result.push(self.transform_line_string(line));
            }
        }
        return result;
    }

    fn transform_polygon(&mut self, rings: &VtPolygon) -> PolygonType {
        let mut result: PolygonType = Vec::new();
        result.reserve(rings.len());
        for ring in rings {
            if ring.area > self.sq_tolerance {
                result.push(self.transform_linear_ring(ring));
            }
        }
        return result;
    }

    fn transform_point(&mut self, p: &VtPoint) -> PointType {
        self.tile.num_simplified += 1;
        return Vec::from(&[
            ((p.x * self.z2 - self.x as f64) * self.extent as f64).round(), // TODO do these have the right type. Shouldnt it be i16?
            ((p.y * self.z2 - self.y as f64) * self.extent as f64).round(),
        ]);
    }

    // TODO
    // fn transform_empty(&self, empty: VtEmpty) -> _ {
    //    return empty;
    //}

    fn transform_linear_ring(&mut self, ring: &VtLinearRing) -> LinearRingType {
        let mut result: LinearRingType = Vec::new();
        //println!("sq_tolerance{:?}", self.sq_tolerance);
        if ring.area > self.sq_tolerance {
            result.reserve(ring.elements.len());
            for p in &ring.elements {
                if p.z > self.sq_tolerance {
                    //eprintln!("p{:.6}, {:.6}, z{:.15}", p.x, p.y, p.z);
                    let vec = self.transform_point(p);
                    //eprintln!("{:?}", vec);
                    result.push(vec);
                }
            }
        }
        return result;
    }
}
