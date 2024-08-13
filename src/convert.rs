use std::f64::consts::PI;

use geojson::feature::Id;
use geojson::{FeatureCollection, Geometry, LineStringType, PointType, PolygonType, Value};
use serde_json::Number;

use crate::simplify::simplify_wrapper;
use crate::types::{
    VtEmpty, VtFeature, VtFeatures, VtGeometry, VtGeometryCollection, VtLineString, VtLinearRing,
    VtMultiLineString, VtMultiPoint, VtMultiPolygon, VtPoint, VtPolygon,
};
use crate::{LinearRingType, MultiLineStringType, MultiPointType, MultiPolygonType};

pub struct Project {
    pub tolerance: f64,
}

impl Project {
    
    // TODO
    pub fn project_empty(&self) -> VtEmpty {
    }

    pub fn project_point(&self, p: PointType) -> VtPoint {
        let sine = (p[1] * PI / 180.).sin();
        let x = p[0] / 360. + 0.5;
        let y = (0.5 - 0.25 * ((1. + sine) / (1. - sine)).ln() / PI)
            .min(1.0)
            .max(0.0);
        VtPoint { x, y, z: 0.0 }
    }

    pub fn project_line_string(&self, points: LineStringType) -> VtLineString {
        let mut result = VtLineString::default();
        let len = points.len();

        if len == 0 {
            return result;
        }

        result.elements.reserve(len);

        for p in points {
            result.elements.push(self.project_point(p));
        }

        for i in 0..len - 1 {
            let a = result.elements[i];
            let b = result.elements[i + 1];
            result.dist += (b.x - a.x).hypot(b.y - a.y);
        }

        simplify_wrapper(&mut result.elements, self.tolerance);

        result.seg_start = 0.;
        result.seg_end = result.dist;

        result
    }

    pub fn project_linear_ring(&self, ring: LinearRingType) -> VtLinearRing {
        let mut result: VtLinearRing = VtLinearRing::default();
        let len = ring.len();

        if len == 0 {
            return result;
        }

        result.elements.reserve(len);

        for p in ring {
            result.elements.push(self.project_point(p));
        }

        let mut area: f64 = 0.0;

        for i in 0..len - 1 {
            let a = result.elements[i];
            let b = result.elements[i + 1];
            area += a.x * b.y - b.x * a.y;
        }
        result.area = (area / 2.).abs();

        simplify_wrapper(&mut result.elements, self.tolerance);

        result
    }

    pub fn project_geometry(&self, geometry: &Geometry) -> VtGeometry {
        match &geometry.value {
            Value::Point(value) => VtGeometry::Point(self.project_point(value.clone())),
            Value::MultiPoint(value) => VtGeometry::MultiPoint(self.project_multi_point(value)),
            Value::LineString(value) => {
                VtGeometry::LineString(self.project_line_string(value.clone()))
            }
            Value::MultiLineString(value) => {
                VtGeometry::MultiLineString(self.project_multi_line_string(value))
            }
            Value::Polygon(value) => VtGeometry::Polygon(self.project_polygon(value)),
            Value::MultiPolygon(value) => {
                VtGeometry::MultiPolygon(self.project_multi_polygon(value))
            }
            Value::GeometryCollection(value) => {
                VtGeometry::GeometryCollection(self.project_geometry_collection(value))
            }
        }
    }

    pub fn project_multi_point(&self, vector: &MultiPointType) -> VtMultiPoint {
        let mut result = Vec::with_capacity(vector.len());
        for e in vector {
            result.push(self.project_point(e.clone()));
        }
        result
    }

    pub fn project_multi_line_string(&self, vector: &MultiLineStringType) -> VtMultiLineString {
        let mut result = Vec::with_capacity(vector.len());
        for e in vector {
            result.push(self.project_line_string(e.clone()));
        }
        result
    }

    pub fn project_multi_polygon(&self, vector: &MultiPolygonType) -> VtMultiPolygon {
        let mut result = Vec::with_capacity(vector.len());
        for e in vector {
            result.push(self.project_polygon(e));
        }
        result
    }

    pub fn project_polygon(&self, vector: &PolygonType) -> VtPolygon {
        let mut result = Vec::with_capacity(vector.len());
        for e in vector {
            result.push(self.project_linear_ring(e.clone()));
        }
        result
    }
    fn project_geometry_collection(&self, vector: &Vec<Geometry>) -> VtGeometryCollection {
        // TODO: verify if translated correctly
        let mut result = Vec::with_capacity(vector.len());
        for e in vector {
            result.push(self.project_geometry(e));
        }
        result
    }
}

pub fn convert(features: &FeatureCollection, tolerance: f64, generate_id: bool) -> VtFeatures {
    let mut projected = Vec::with_capacity(features.features.len());

    let mut gen_id: u64 = 0;
    for feature in features {
        let mut feature_id = feature.id.clone();
        if generate_id {
            feature_id = Some(Id::Number(Number::from(gen_id)));
            gen_id += 1;
        }

        let project = Project { tolerance };

        let feature = VtFeature::new(
            project.project_geometry(feature.geometry.as_ref().unwrap()),
            feature
                .properties
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(), // TODO is this unwrapping oke?
            feature_id.clone(),
        );
        if let Some(feature) = feature {
            projected.push(feature);
        }
    }
    projected
}
