use std::collections::HashMap;

use euclid::{Point2D, UnknownUnit};
use geojson::{JsonObject, JsonValue};

use crate::BBox;

pub type VtEmpty = ();
pub type VtGeometryCollection = Vec<VtGeometry>;

#[derive(Debug, Clone, PartialEq)]
pub enum VtGeometry {
    Empty(VtEmpty),
    Point(VtPoint),
    MultiPoint(VtMultiPoint),
    LineString(VtLineString),
    MultiLineString(VtMultiLineString),
    Polygon(VtPolygon),
    MultiPolygon(VtMultiPolygon),
    GeometryCollection(VtGeometryCollection),
}

impl VtGeometry {
    pub fn point(self) -> Option<VtPoint> {
        match self {
            VtGeometry::Point(value) => Some(value),
            _ => None,
        }
    }

    pub fn multi_point(self) -> Option<VtMultiPoint> {
        match self {
            VtGeometry::MultiPoint(value) => Some(value),
            _ => None,
        }
    }

    pub fn line_string(self) -> Option<VtLineString> {
        match self {
            VtGeometry::LineString(value) => Some(value),
            _ => None,
        }
    }
    pub fn multi_line_string(self) -> Option<VtMultiLineString> {
        match self {
            VtGeometry::MultiLineString(value) => Some(value),
            _ => None,
        }
    }
    pub fn polygon(self) -> Option<VtPolygon> {
        match self {
            VtGeometry::Polygon(value) => Some(value),
            _ => None,
        }
    }

    pub fn multi_polygon(self) -> Option<VtMultiPolygon> {
        match self {
            VtGeometry::MultiPolygon(value) => Some(value),
            _ => None,
        }
    }
    pub fn geometry_collection(self) -> Option<VtGeometryCollection> {
        match self {
            VtGeometry::GeometryCollection(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VtPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64, // Simplification tolerance
}

impl VtPoint {
    // Constructor with z value
    pub fn new(x: f64, y: f64, z: f64) -> VtPoint {
        VtPoint { x, y, z }
    }

    // Constructor without z value
    pub fn new_without_z(x: f64, y: f64) -> VtPoint {
        VtPoint::new(x, y, 0.0)
    }
}

// Function templates to get coordinates
pub trait GetCoordinate<const I: usize> {
    fn get(&self) -> f64;
}

impl<const I: usize> GetCoordinate<I> for VtPoint {
    fn get(&self) -> f64 {
        match I {
            0 => self.x,
            1 => self.y,
            _ => {
                panic!("GetCoordinate is only implemented for I = 0 and I = 1")
            }
        }
    }
}

impl<const I: usize> GetCoordinate<I> for Point2D<f64, UnknownUnit> {
    fn get(&self) -> f64 {
        match I {
            0 => self.x,
            1 => self.y,
            _ => {
                panic!("GetCoordinate is only implemented for I = 0 and I = 1")
            }
        }
    }
}

// Calculation of progress along a line
pub fn calc_progress<const I: usize>(a: &VtPoint, b: &VtPoint, v: f64) -> f64 {
    match I {
        0 => (v - a.x) / (b.x - a.x),
        1 => (v - a.y) / (b.y - a.y),
        _ => {
            panic!("calc_progress is only implemented for I = 0 and I = 1")
        }
    }
}

// Intersection calculation based on linear interpolation
pub fn intersect<const I: usize>(a: &VtPoint, b: &VtPoint, v: f64, t: f64) -> VtPoint {
    match I {
        0 => {
            let y = (b.y - a.y) * t + a.y;
            VtPoint::new(v, y, 1.0)
        }
        1 => {
            let x = (b.x - a.x) * t + a.x;
            VtPoint::new(x, v, 1.0)
        }
        _ => {
            panic!("calc_progress is only implemented for I = 0 and I = 1")
        }
    }
}

pub type VtMultiPoint = Vec<VtPoint>;

#[derive(Debug, Clone, PartialEq)]
pub struct VtLineString {
    pub elements: Vec<VtPoint>,
    pub dist: f64,
    pub seg_start: f64,
    pub seg_end: f64,
}

impl VtLineString {
    pub fn new() -> Self {
        Self {
            elements: vec![],
            dist: 0.0, // line length
            seg_start: 0.0,
            seg_end: 0.0, // seg_start and seg_end are distance along a line in tile units, when lineMetrics = true
        }
    }

    pub fn from_slice(slice: &[VtPoint]) -> Self {
        Self {
            elements: Vec::from(slice),
            dist: 0.0, // line length
            seg_start: 0.0,
            seg_end: 0.0, // seg_start and seg_end are distance along a line in tile units, when lineMetrics = true
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VtLinearRing {
    pub elements: Vec<VtPoint>,
    pub area: f64, // polygon ring area
}

impl VtLinearRing {
    pub fn new() -> VtLinearRing {
        Self {
            elements: vec![],
            area: 0.0,
        }
    }
    pub fn from_slice(points: &[VtPoint]) -> VtLinearRing {
        Self {
            elements: Vec::from(points),
            area: 0.0,
        }
    }
}

pub type VtMultiLineString = Vec<VtLineString>;
pub type VtPolygon = Vec<VtLinearRing>;
pub type VtMultiPolygon = Vec<VtPolygon>;

#[derive(Debug, Clone, PartialEq)]
pub struct VtFeature {
    pub geometry: VtGeometry,
    pub properties: JsonObject,
    pub id: Option<geojson::feature::Id>,
    pub bbox: BBox,
    pub num_points: u32,
}

impl VtFeature {
    pub fn new(
        geom: VtGeometry,
        props: JsonObject,
        id: Option<geojson::feature::Id>,
    ) -> Option<Self> {
        let mut feature = Self {
            geometry: geom,
            properties: props,
            id,
            bbox: Default::default(),
            num_points: 0,
        };
        feature.process_geometry();
        if feature.num_points == 0 {
            // TODO: Is this the right place to filter for empty?
            return None;
        } else {
            Some(feature)
        }
    }
}

pub(crate) fn for_each_point<F>(geometry: &mut VtGeometry, f: &mut F)
where
    F: FnMut(&mut VtPoint),
{
    // TODO verify this translation
    match geometry {
        VtGeometry::Empty(_) => {}
        VtGeometry::Point(point) => f(point),
        VtGeometry::MultiPoint(multi_point) => {
            for point in multi_point {
                f(point)
            }
        }
        VtGeometry::LineString(line_string) => {
            for point in &mut line_string.elements {
                f(point)
            }
        }
        VtGeometry::MultiLineString(multi_line_string) => {
            for line_string in multi_line_string {
                for point in &mut line_string.elements {
                    f(point)
                }
            }
        }
        VtGeometry::Polygon(polygon) => {
            for line_string in polygon {
                for point in &mut line_string.elements {
                    f(point)
                }
            }
        }
        VtGeometry::MultiPolygon(multi_polygon) => {
            for polygon in multi_polygon {
                for line_string in polygon {
                    for point in &mut line_string.elements {
                        f(point)
                    }
                }
            }
        }
        VtGeometry::GeometryCollection(collection) => {
            // TODO: verify if translated correctly
            for geometry in collection {
                for_each_point(geometry, f)
            }
        }
    }
}

impl VtFeature {
    fn process_geometry(&mut self) {
        let mut f = |point: &mut VtPoint| {
            self.bbox.min.x = (point.x).min(self.bbox.min.x);
            self.bbox.min.y = (point.y).min(self.bbox.min.y);
            self.bbox.max.x = (point.x).max(self.bbox.max.x);
            self.bbox.max.y = (point.y).max(self.bbox.max.y);
            self.num_points = self.num_points + 1;
        };
        // TODO verify this translation
        for_each_point(&mut self.geometry, &mut f)
    }
}

pub type VtFeatures = Vec<VtFeature>;
