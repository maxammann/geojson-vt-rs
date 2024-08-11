use crate::simplify::simplify;
use crate::types::{
    VtEmpty, VtFeature, VtFeatures, VtGeometry, VtLineString, VtLinearRing, VtMultiLineString,
    VtMultiPoint, VtMultiPolygon, VtPoint,
};
use crate::{MultiLineStringType, MultiPointType, MultiPolygonType};
use geojson::{Feature, FeatureCollection, Geometry, LineStringType, PointType, Value};

pub struct Project {
    pub tolerance: f64,
}

impl Project {
    pub fn project_empty(&self, empty: EmptyType) -> VtEmpty {
        return empty;
    }

    pub fn project_point(&self, p: PointType) -> VtPoint {
        let sine = std::sin(p.y * M_PI / 180);
        let x = p.x / 360 + 0.5;
        let y = std::max(
            std::min(0.5 - 0.25 * std::log((1 + sine) / (1 - sine)) / M_PI, 1.0),
            0.0,
        );
        return VtPoint { x, y, z: 0.0 };
    }

    pub fn project_line_string(&self, points: LineStringType) -> VtLineString {
        let mut result = VtLineString::new();
        let len = points.size();

        if len == 0 {
            return result;
        }

        result.reserve(len);

        for p in points {
            result.push_back(operator()(p));
        }

        for i in 0..len - 1 {
            let a = result.elements[i];
            let b = result.elements[i + 1];
            result.dist += (b.x - a.x).hypot(b.y - a.y);
        }

        simplify(result, self.tolerance);

        result.seg_start = 0;
        result.seg_end = result.dist;

        return result;
    }

    pub fn project_linear_ring(&self, ring: LinearRingType) -> VtLinearRing {
        let mut result: VtLinearRing = Vec::new();
        let len = ring.size();

        if len == 0 {
            return result;
        }

        result.reserve(len);

        for p in ring {
            result.push_back(operator()(p));
        }

        let mut area = 0.0;

        for i in 0..len - 1 {
            let a = result[i];
            let b = result[i + 1];
            area += a.x * b.y - b.x * a.y;
        }
        result.area = (area / 2).abs();

        simplify(&result, self.tolerance);

        return result;
    }

    pub fn project_geometry(&self, geometry: &Geometry) -> VtGeometry {
        // TODO return geometry::geometry<double>::visit(geometry, project{ tolerance });
    }

    pub fn project_multi_point(&self, geometry: &MultiPointType) -> VtMultiPoint {
        let result;
        result.reserve(vector.size());
        for e in vector {
            result.push_back(operator()(e));
        }
        return result;
    }

    pub fn project_multi_line_string(&self, geometry: &MultiLineStringType) -> VtMultiLineString {
        unimplemented!()
    }

    pub fn project_multi_polygon(&self, geometry: &MultiPolygonType) -> VtMultiPolygon {
        unimplemented!()
    }
}

pub fn convert(features: &FeatureCollection, tolerance: f64, generate_id: bool) -> VtFeatures {
    let mut projected = Vec::new();
    projected.reserve(features.size());
    let genId: u64 = 0;
    for feature in features {
        let featureId = feature.id;
        if generate_id {
            // TODOfeatureId = { uint64_t {genId++} };
        }
        // TODO projected.push(geometry::geometry<double>::visit(feature.geometry, project{ tolerance }), feature.properties, featureId);
    }
    return projected;
}
