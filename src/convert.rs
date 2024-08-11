use std::f64::consts::PI;
use crate::simplify::{simplify, simplify_wrapper};
use crate::types::{VtEmpty, VtFeature, VtFeatures, VtGeometry, VtLineString, VtLinearRing, VtMultiLineString, VtMultiPoint, VtMultiPolygon, VtPoint, VtPolygon};
use crate::{LinearRingType, MultiLineStringType, MultiPointType, MultiPolygonType};
use geojson::{Feature, FeatureCollection, Geometry, LineStringType, PointType, PolygonType, Value};
use geojson::feature::Id;
use serde_json::Number;

pub struct Project {
    pub tolerance: f64,
}

impl Project {
    // TODO pub fn project_empty(&self, empty: EmptyType) -> VtEmpty {
    //     return empty;
    // }

    pub fn project_point(&self, p: PointType) -> VtPoint {
        let sine = (p[1] * PI / 180.).sin();
        let x = p[0] / 360. + 0.5;
        // TODO check if this was translated correctly
        let y = (
           (0.5 - 0.25 * ((1. + sine) / (1. - sine)).ln() / PI).min(1.0)).max(
            0.0,
        );
        return VtPoint { x, y, z: 0.0 };
    }

    pub fn project_line_string(&self, points: LineStringType) -> VtLineString {
        let mut result = VtLineString::new();
        let len = points.len();

        if len == 0 {
            return result;
        }

        result.elements.reserve(len);

        for p in points {
            result.elements.push(self.project_point(p)); // TODO check if this call is fine
        }

        for i in 0..len - 1 {
            let a = result.elements[i];
            let b = result.elements[i + 1];
            result.dist += (b.x - a.x).hypot(b.y - a.y);
        }

        simplify_wrapper(&mut result.elements, self.tolerance);

        result.seg_start = 0.;
        result.seg_end = result.dist;

        return result;
    }

    pub fn project_linear_ring(&self, ring: LinearRingType) -> VtLinearRing {
        let mut result: VtLinearRing = VtLinearRing::new();
        let len = ring.len();

        if len == 0 {
            return result;
        }

        result.elements.reserve(len);

        for p in ring {
            result.elements.push(self.project_point(p)); // TODO check if this call is fine
        }

        let mut area = 0.0;

        for i in 0..len - 1 {
            let a = result.elements[i];
            let b = result.elements[i + 1];
            area += a.x * b.y - b.x * a.y;
        }
        result.area = (area / 2.).abs();

        simplify_wrapper(&mut result.elements, self.tolerance);

        return result;
    }

    pub fn project_geometry(&self, geometry: &Geometry) -> VtGeometry {
        // TODO check if this is correct
        match &geometry.value {
                Value::Point(value) => VtGeometry::Point(self.project_point(value.clone())),
                Value::MultiPoint(value) => VtGeometry::MultiPoint(self.project_multi_point(value)),
                Value::LineString(value) =>VtGeometry::LineString(self.project_line_string(value.clone())),
                Value::MultiLineString(value) =>VtGeometry::MultiLineString(self.project_multi_line_string(value)),
                Value::Polygon(value) =>VtGeometry::Polygon(self.project_polygon(value)),
                Value::MultiPolygon(value) =>VtGeometry::MultiPolygon(self.project_multi_polygon(value)),
                Value::GeometryCollection(value) => unimplemented!(),

        }
    }

    pub fn project_multi_point(&self, vector: &MultiPointType) -> VtMultiPoint {
        let mut result = Vec::new();
        result.reserve(vector.len());
        for e in vector {
            result.push(self.project_point(e.clone()));
        }
        return result;
    }

    pub fn project_multi_line_string(&self, vector: &MultiLineStringType) -> VtMultiLineString {
        let mut result = Vec::new();
        result.reserve(vector.len());
        for e in vector {
            result.push(self.project_line_string(e.clone()));
        }
        return result;
    }

    pub fn project_multi_polygon(&self, vector: &MultiPolygonType) -> VtMultiPolygon {
        let mut result = Vec::new();
        result.reserve(vector.len());
        for e in vector {
            result.push(self.project_polygon(e));
        }
        return result;
    }

    pub fn project_polygon(&self, vector: &PolygonType) -> VtPolygon {
        let mut result = Vec::new();
        result.reserve(vector.len());
        for e in vector {
            result.push(self.project_linear_ring(e.clone()));
        }
        return result;
    }
}

pub fn convert(features: &FeatureCollection, tolerance: f64, generate_id: bool) -> VtFeatures {
    let mut projected = Vec::new();
    projected.reserve(features.features.len());
    let mut genId: u64 = 0;
    for feature in features {
        let mut featureId = feature.id.clone();
        if generate_id {
            featureId = Some(Id::Number(Number::from(genId)));
            genId= genId +1;
        }
        
        let project = Project {
            tolerance
        };
        
        projected.push(VtFeature::new(project.project_geometry(&feature.geometry.as_ref().unwrap()), feature.properties.clone().unwrap().into_iter().collect(), featureId.clone()));
    }
    return projected;
}
