use crate::types::{
    GetCoordinate, VtEmpty, VtFeature, VtFeatures, VtGeometry, VtGeometryCollection, VtLineString,
    VtLinearRing, VtMultiLineString, VtMultiPoint, VtMultiPolygon, VtPoint, VtPolygon,
};

trait IsTrue<const B: bool> {}
impl IsTrue<true> for () {}

// Define a generic clipper struct templated with a constant index.
pub struct Clipper<const I: usize> {
    k1: f64,
    k2: f64,
    line_metrics: bool,
}

impl<const I: usize> Clipper<I> {
    pub fn new(k1: f64, k2: f64, line_metrics: bool) -> Self {
        Clipper {
            k1,
            k2,
            line_metrics,
        }
    }

    pub fn clip_empty(&self, empty: &VtEmpty) -> VtGeometry {
        return VtGeometry::Empty(*empty);
    }

    pub fn clip_point(&self, point: &VtPoint) -> VtGeometry {
        return VtGeometry::Point(*point);
    }

    pub fn clip_multi_point(&self, points: &VtMultiPoint) -> VtGeometry {
        let mut part: VtMultiPoint = Vec::new();
        for p in points {
            let ak = GetCoordinate::<I>::get(p);
            if ak >= self.k1 && ak <= self.k2 {
                part.push(*p);
            }
        }
        return VtGeometry::MultiPoint(part);
    }

    pub fn clip_line_string(&self, line: &VtLineString) -> VtGeometry {
        let mut parts: VtMultiLineString = VtMultiLineString::new();
        self.clip_line(line, &mut parts);

        if parts.len() == 1 {
            return VtGeometry::LineString(parts[0].clone());
        }

        return VtGeometry::MultiLineString(parts.clone());
    }

    pub fn clip_multi_line_string(&self, lines: &VtMultiLineString) -> VtGeometry {
        let mut parts: VtMultiLineString = Vec::new();
        for line in lines {
            self.clip_line(line, &mut parts);
        }

        if parts.len() == 1 {
            return VtGeometry::LineString(parts[0].clone());
        }

        return VtGeometry::MultiLineString(parts.clone());
    }

    pub fn clip_polygon(&self, polygon: &VtPolygon) -> VtGeometry {
        let mut result: VtPolygon = Vec::new();
        for ring in polygon {
            let new_ring = self.clip_ring(ring);
            if !new_ring.elements.is_empty() {
                result.push(new_ring);
            }
        }
        return VtGeometry::Polygon(result);
    }

    pub fn clip_multi_polygon(&self, polygons: &VtMultiPolygon) -> VtGeometry {
        let mut result: VtMultiPolygon = Vec::new();
        for polygon in polygons {
            let mut p: VtPolygon = Vec::new();
            for ring in polygon {
                let new_ring = self.clip_ring(ring);
                if !new_ring.elements.is_empty() {
                    p.push(new_ring);
                }
            }
            if !p.is_empty() {
                result.push(p.clone());
            }
        }
        return VtGeometry::MultiPolygon(result);
    }

    pub fn clip_geometry_collection(&self, geometries: &VtGeometryCollection) -> VtGeometry {
        let mut result: VtGeometryCollection = Vec::new();
        for geometry in geometries {
            // TODO: verify if translated correctly
            result.push(self.clip_geometry(geometry));
        }
        return VtGeometry::GeometryCollection(result);
    }
    fn clip_geometry(&self, geometry: &VtGeometry) -> VtGeometry {
        match geometry {
            VtGeometry::Empty(empty) => self.clip_empty(empty),
            VtGeometry::Point(point) => self.clip_point(point),
            VtGeometry::MultiPoint(multi_point) => self.clip_multi_point(multi_point),
            VtGeometry::LineString(line_string) => self.clip_line_string(line_string),
            VtGeometry::MultiLineString(multi_line_string) => {
                self.clip_multi_line_string(multi_line_string)
            }
            VtGeometry::Polygon(polygon) => self.clip_polygon(polygon),
            VtGeometry::MultiPolygon(multi_polygon) => self.clip_multi_polygon(multi_polygon),
            VtGeometry::GeometryCollection(geometry_collection) => {
                self.clip_geometry_collection(geometry_collection)
            }
        }
    }
}

impl<const I: usize> Clipper<I> {
    fn new_slice(&self, line: &VtLineString) -> VtLineString {
        let mut slice: VtLineString = VtLineString::new();
        slice.dist = line.dist;
        if self.line_metrics {
            slice.seg_start = line.seg_start;
            slice.seg_end = line.seg_end;
        }
        return slice;
    }

    // Mimic the clipLine function from C++
    fn clip_line(&self, line: &VtLineString, slices: &mut Vec<VtLineString>) {
        // if line.len() < 2 {
        //     return;
        // }
        // let mut slice = self.new_slice(line);
        // for (i, window) in line.windows(2).enumerate() {
        //     let a = &window[0];
        //     let b = &window[1];
        //     let is_last_seg = i == line.len() - 2;
        //     Implement clipping logic here
        // }
        // if !slice.is_empty() {
        //     slices.push(slice);
        // }
        todo!() // Implement this functionality based on your LineString and Vector geometry
    }

    // Mimic the clipRing function from C++
    fn clip_ring(&self, ring: &VtLinearRing) -> VtLinearRing {
        // let mut slice = LineString::new();
        // slice.area = ring.area; // Assuming LineString has an area attribute
        // for (i, window) in ring.windows(2).enumerate() {
        //     Implement clipping logic here
        // }
        // Close the polygon if endpoints are not the same
        // if !slice.is_empty() && slice[0] != slice[slice.len() - 1] {
        //     slice.push(slice[0]);
        // }
        // return slice;
        todo!() // Implement this functionality based on your LineString structure
    }
}

/* clip features between two axis-parallel lines:
 *     |        |
 *  ___|___     |     /
 * /   |   \____|____/
 *     |        |
 */
pub fn clip<const I: usize>(
    features: &VtFeatures,
    k1: f64,
    k2: f64,
    min_all: f64,
    max_all: f64,
    line_metrics: bool,
) -> VtFeatures {
    // trivial accept
    if min_all >= k1 && max_all < k2 {
        return features.clone();
    }

    // trivial reject
    if max_all < k1 || min_all >= k2 {
        return VtFeatures::new(); // TODO or none?
    }

    let mut clipped: VtFeatures = Vec::new();
    clipped.reserve(features.len());

    for feature in features {
        let geom = &feature.geometry;
        // TODO: assert!(feature.properties);
        let props = &feature.properties;
        let id = &feature.id;

        let min = GetCoordinate::<I>::get(&feature.bbox.min);
        let max = GetCoordinate::<I>::get(&feature.bbox.max);

        if min >= k1 && max < k2 {
            // trivial accept
            clipped.push(feature.clone());
        } else if max < k1 || min >= k2 {
            // trivial reject
            continue;
        } else {
            // TODO: verify if translated correctly
            let clipped_geom = Clipper::<I>::new(k1, k2, line_metrics).clip_geometry(geom);

            match &clipped_geom {
                VtGeometry::MultiLineString(result) => {
                    if line_metrics {
                        for segment in result {
                            clipped.push(VtFeature::new(
                                VtGeometry::LineString(segment.clone()),
                                props.clone(),
                                id.clone(),
                            ));
                        }
                    } else {
                        clipped.push(VtFeature::new(clipped_geom, props.clone(), id.clone()));
                    }
                }
                _ => {
                    clipped.push(VtFeature::new(clipped_geom, props.clone(), id.clone()));
                }
            }
        }
    }

    return clipped;
}
