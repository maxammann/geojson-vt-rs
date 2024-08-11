use std::collections::HashMap;

use euclid::{Point2D, UnknownUnit};

use crate::BBox;

pub type VtEmpty = ();
pub type VtGeometryCollection = Vec<VtGeometry>;

#[derive(Clone, PartialEq)]
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
pub fn calc_progress_x(a: &VtPoint, b: &VtPoint, x: f64) -> f64 {
    (x - a.x) / (b.x - a.x)
}

pub fn calc_progress_y(a: &VtPoint, b: &VtPoint, y: f64) -> f64 {
    (y - a.y) / (b.y - a.y)
}

// Intersection calculation based on linear interpolation
pub fn intersect_x(a: &VtPoint, b: &VtPoint, x: f64, t: f64) -> VtPoint {
    let y = (b.y - a.y) * t + a.y;
    VtPoint::new(x, y, 1.0)
}

pub fn intersect_y(a: &VtPoint, b: &VtPoint, y: f64, t: f64) -> VtPoint {
    let x = (b.x - a.x) * t + a.x;
    VtPoint::new(x, y, 1.0)
}

pub type VtMultiPoint = Vec<VtPoint>;

#[derive(Clone, PartialEq)]
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
}

#[derive(Clone, PartialEq)]
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
}

pub type VtMultiLineString = Vec<VtLineString>;
pub type VtPolygon = Vec<VtLinearRing>;
pub type VtMultiPolygon = Vec<VtPolygon>;

#[derive(Clone, PartialEq)]
pub struct VtFeature {
    pub geometry: VtGeometry,
    pub properties: HashMap<String, geojson::JsonValue>,
    pub id: Option<geojson::feature::Id>,
    pub bbox: BBox,
    pub num_points: u32,
}

impl VtFeature {
    pub fn new(
        geom: VtGeometry,
        props: HashMap<String, geojson::JsonValue>,
        id: Option<geojson::feature::Id>,
    ) -> Self {
        let mut feature = Self {
            geometry: geom,
            properties: props,
            id,
            bbox: Default::default(),
            num_points: 0,
        };
        feature.process_geometry();
        feature
    }
}

impl VtFeature {
    fn process_geometry(&mut self) {
        // TODO verify this translation
        match &mut self.geometry {
            VtGeometry::Point(point) => {
                self.bbox.min.x = (point.x).min(self.bbox.min.x);
                self.bbox.min.y = (point.y).min(self.bbox.min.y);
                self.bbox.max.x = (point.x).max(self.bbox.max.x);
                self.bbox.max.y = (point.y).max(self.bbox.max.y);
                self.num_points = self.num_points + 1;
            }
            _ => {}
        }
    }
}

pub type VtFeatures = Vec<VtFeature>;
